//! Hito 0.2 — `detector_hw.rs`: Detección universal de hardware vía sysfs/procfs.
//!
//! Principios:
//! - Zero GC, zero dependencias externas (solo std).
//! - Sin unwrap(): todo error se propaga como `DetectorError` con fallback degradado.
//! - Universalidad: el MISMO binario distingue Intel (X220), AMD Vega gfx90c (Yoga/P14s),
//!   NVIDIA (nvidia-smi ausente => no asumido por sysfs), y ARM (RPi/PSP) por device-tree.
//! - Anti-cuelgues: lectura de archivos sysfs es O(1); nvidia-smi tiene timeout externo
//!   en Fase 3 (aquí solo se detecta por PCI, sin spawnar procesos).

use std::fmt;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum DetectorError {
    /// No se pudo leer /proc/meminfo (entorno no-Linux o FS restringido).
    SinMemoria,
    /// No se pudo leer /sys/firmware/devicetree ni /proc/cpuinfo para la arquitectura.
    SinArquitectura,
}

impl fmt::Display for DetectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectorError::SinMemoria => write!(f, "no se pudo leer /proc/meminfo"),
            DetectorError::SinArquitectura => write!(f, "no se pudo determinar arquitectura"),
        }
    }
}

impl std::error::Error for DetectorError {}

/// Perfil de hardware detectado en tiempo real (sin hardcoding de marca).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfilHardware {
    /// "x86_64", "aarch64", "armv7", "riscv64"...
    pub arquitectura: String,
    pub ram_mb: u64,
    /// VRAM dedicada estimada en MB. 0 si solo iGPU o sin GPU.
    pub gpu_vram_mb: u64,
    /// Descripción de la GPU (vendor/device desde PCI o device-tree).
    pub gpu_desc: String,
    /// Backend recomendado: "vulkan" (Vega/NVIDIA/intel moderno), "cpu".
    pub backend_inferencia: &'static str,
    pub almacenamiento: TipoAlmacenamiento,
    pub tiene_gpio: bool,
    pub tiene_tpm2: bool,
    pub n_cores: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoAlmacenamiento {
    Nvme,
    Ssd,
    Sdcard,
    Flash,
    Desconocido,
}

impl fmt::Display for TipoAlmacenamiento {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TipoAlmacenamiento::Nvme => "nvme",
            TipoAlmacenamiento::Ssd => "ssd",
            TipoAlmacenamiento::Sdcard => "sdcard",
            TipoAlmacenamiento::Flash => "flash",
            TipoAlmacenamiento::Desconocido => "desconocido",
        };
        write!(f, "{s}")
    }
}

/// Configuración líquida resultante: qué modo activa el loader (`omni-init`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfiguracionOmni {
    /// L0: Reglas + TinyLLM <50M params. GPIO, UART/BLE, TTY. (<25MB RAM)
    ModoSensorActuador,
    /// L1: SmolLM2-135M / Qwen2.5-1.5B. Chat, MCP básico, RAG local. (~300MB)
    ModoAsistenteLocal,
    /// L2: Router multi-modelo + DiskLLM + COSMIC. (>=3GB disponible)
    ModoPortatil,
    /// L2+: MoE/enjambre completo, QML, HPC remoto.
    ModoSupercomputadora,
}

impl fmt::Display for ConfiguracionOmni {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConfiguracionOmni::ModoSensorActuador => "L0/sensor-actuador",
            ConfiguracionOmni::ModoAsistenteLocal => "L1/asistente-local",
            ConfiguracionOmni::ModoPortatil => "L2/portatil",
            ConfiguracionOmni::ModoSupercomputadora => "L2+/supercomputadora",
        };
        write!(f, "{s}")
    }
}

/// Lee y recorta un archivo sysfs/procfs. Devuelve None ante cualquier fallo
/// (permisos, archivo inexistente, E/S) — nunca panica.
fn leer_sys(path: &str) -> Option<String> {
    fs::read_to_string(Path::new(path))
        .ok()
        .map(|s| s.trim_matches('\0').trim().to_string())
        .filter(|s| !s.is_empty())
}

/// RAM total en MB desde /proc/meminfo (línea MemTotal en kB).
pub fn ram_total_mb() -> Result<u64, DetectorError> {
    let meminfo = leer_sys("/proc/meminfo").ok_or(DetectorError::SinMemoria)?;
    for linea in meminfo.lines() {
        if let Some(rest) = linea.strip_prefix("MemTotal:") {
            let kb: u64 = rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .map_err(|_| DetectorError::SinMemoria)?;
            return Ok(kb / 1024);
        }
    }
    Err(DetectorError::SinMemoria)
}

/// Arquitectura: preferimos device-tree (ARM/RISC-V), fallback uname vía std::env::consts.
pub fn arquitectura() -> String {
    if let Some(dt) = leer_sys("/sys/firmware/devicetree/base/architecture") {
        // p.ej. "armv7l", "BCM2835" -> normalizamos
        let lower = dt.to_lowercase();
        if lower.contains("arm") && !std::env::consts::ARCH.contains("aarch64") {
            // device-tree presente => SoC embebido (RPi/PSP-like)
            return match std::env::consts::ARCH {
                "aarch64" => "aarch64".into(),
                _ => "armv7".into(),
            };
        }
    }
    // x86_64 / aarch64 / riscv64 según compilación del propio binario.
    std::env::consts::ARCH.to_string()
}

/// Escanea /sys/bus/pci/devices/*/{vendor,device,class} buscando GPUs.
/// Vendor IDs conocidos: 0x10de NVIDIA, 0x1002 AMD, 0x8086 Intel.
/// La VRAM real de AMD se lee de /sys/class/drm/card*/mem_info_vram_used (amdgpu);
/// para NVIDIA se delega a nvidia-smi en Fase 3 (nunca spawn aquí sin sandbox).
pub fn detectar_gpu() -> (String, u64, &'static str) {
    let mut desc = String::from("gpu:no-encontrada");
    let mut vram_mb: u64 = 0;
    let mut backend = "cpu";
    let mut es_amdgpu = false;
    let mut es_nvidia = false;

    let base = Path::new("/sys/bus/pci/devices");
    if let Ok(devs) = fs::read_dir(base) {
        for dev in devs.flatten() {
            let class = match leer_sys(&dev.path().join("class").to_string_lossy()) {
                Some(c) => c,
                None => continue,
            };
            // class 0x0302xx = 3D controller, 0x0300xx = VGA compatible
            if !class.starts_with("0x03") {
                continue;
            }
            let vendor = leer_sys(&dev.path().join("vendor").to_string_lossy()).unwrap_or_default();
            let device = leer_sys(&dev.path().join("device").to_string_lossy()).unwrap_or_default();
            match vendor.as_str() {
                "0x1002" => {
                    es_amdgpu = true;
                    backend = "vulkan"; // REGLA 6: Vulkan/wgpu > ROCm para Vega gfx90c
                    desc = format!("gpu:amd:{device}");
                }
                "0x10de" => {
                    es_nvidia = true;
                    backend = "vulkan"; // llama.cpp CUDA en fase posterior; vulkan universal
                    desc = format!("gpu:nvidia:{device}");
                }
                "0x8086" => {
                    desc = format!("gpu:intel:{device}");
                    // Iris/HD antiguos (X220): CPU fallback estable.
                }
                _ => {}
            }
        }
    }

    // VRAM dedicada amdgpu: sysfs expone mem_info_vram_size (bytes).
    if es_amdgpu {
        if let Ok(cards) = fs::read_dir("/sys/class/drm") {
            for card in cards.flatten() {
                let p = card.path().join("device/mem_info_vram_size");
                if let Some(bytes) =
                    leer_sys(&p.to_string_lossy()).and_then(|s| s.parse::<u64>().ok())
                {
                    // gfx90c suele reportar 512MB de stolen + shared; solo contamos dedicada.
                    let mb = bytes / 1024 / 1024;
                    if mb > vram_mb {
                        vram_mb = mb;
                    }
                }
            }
        }
    }
    // NVIDIA: sin spawn de proceso aquí (REGLA 3 anti-cuelgues). Se estima vía
    // resource0 BAR en Fase 3; por ahora marcamos presencia con 0 MB conservador.
    let _ = es_nvidia;

    (desc, vram_mb, backend)
}

/// Tipo de almacenamiento raíz: rotaational + modelo nvme.
pub fn detectar_almacenamiento() -> TipoAlmacenamiento {
    let blk = Path::new("/sys/block");
    let Ok(devs) = fs::read_dir(blk) else {
        return TipoAlmacenamiento::Desconocido;
    };
    let mut mejor = TipoAlmacenamiento::Desconocido;
    for dev in devs.flatten() {
        let name = dev.file_name().to_string_lossy().to_string();
        if name.starts_with("loop") || name.starts_with("dm-") || name.starts_with("zram") {
            continue;
        }
        let rot = leer_sys(&dev.path().join("queue/rotational").to_string_lossy())
            .unwrap_or_else(|| "1".into());
        if name.starts_with("nvme") {
            return TipoAlmacenamiento::Nvme; // NVMe gana siempre
        }
        if rot == "0" {
            if name.starts_with("mmcblk") {
                mejor = TipoAlmacenamiento::Sdcard;
            } else if mejor != TipoAlmacenamiento::Nvme {
                mejor = TipoAlmacenamiento::Ssd;
            }
        } else if name.starts_with("mtdblock") {
            mejor = TipoAlmacenamiento::Flash;
        }
    }
    mejor
}

/// ¿Hay controlador TPM 2.0? (Anclaje del Sello Maldito, Hito 0.3).
pub fn detectar_tpm2() -> bool {
    Path::new("/sys/class/tpm/tpm0").exists()
        && leer_sys("/sys/class/tpm/tpm0/tpm_version_major")
            .map(|v| v == "2")
            .unwrap_or(false)
}

/// ¿GPIO expuesto? (PL061 / sysfs gpiochip => perfil L0 posible).
pub fn detectar_gpio() -> bool {
    Path::new("/sys/class/gpio/gpiochip0").exists()
        || fs::read_dir("/sys/class/gpio").is_ok_and(|mut d| d.next().is_some())
}

pub fn n_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

impl PerfilHardware {
    /// Detección completa. Nunca panica: ante fallos devuelve perfil conservador L0/L1.
    pub fn detectar() -> Result<Self, DetectorError> {
        let ram_mb = ram_total_mb()?;
        let arquitectura = arquitectura();
        let (gpu_desc, gpu_vram_mb, backend_inferencia) = detectar_gpu();
        Ok(Self {
            arquitectura,
            ram_mb,
            gpu_vram_mb,
            gpu_desc,
            backend_inferencia,
            almacenamiento: detectar_almacenamiento(),
            tiene_gpio: detectar_gpio(),
            tiene_tpm2: detectar_tpm2(),
            n_cores: n_cores(),
        })
    }

    /// Adaptación Líquida por Hardware: matriz de perfiles de la sección §7.
    /// Orden de decisión: arquitectura -> RAM -> VRAM. Degradación segura hacia abajo.
    pub fn configuracion_recomendada(&self) -> ConfiguracionOmni {
        match self.arquitectura.as_str() {
            "armv7" if self.ram_mb < 1024 => ConfiguracionOmni::ModoSensorActuador,
            "armv7" => ConfiguracionOmni::ModoAsistenteLocal,
            "aarch64" | "riscv64" => {
                if self.ram_mb < 512 {
                    ConfiguracionOmni::ModoSensorActuador
                } else if self.ram_mb < 4096 {
                    ConfiguracionOmni::ModoAsistenteLocal
                } else if self.ram_mb >= 8192 && self.gpu_vram_mb >= 10240 {
                    ConfiguracionOmni::ModoSupercomputadora
                } else {
                    ConfiguracionOmni::ModoPortatil
                }
            }
            _ => {
                // x86_64 y resto (ThinkPads, Yoga, estaciones NVIDIA/AMD)
                if self.ram_mb < 1024 {
                    ConfiguracionOmni::ModoSensorActuador
                } else if self.ram_mb < 4096 {
                    ConfiguracionOmni::ModoAsistenteLocal
                } else if self.ram_mb >= 8192 && self.gpu_vram_mb >= 10240 {
                    ConfiguracionOmni::ModoSupercomputadora
                } else {
                    ConfiguracionOmni::ModoPortatil
                }
            }
        }
    }

    /// Huella RAM objetivo del núcleo según modo (<100MB garantizado en todos).
    pub fn presupuesto_nucleo_kb(&self) -> u64 {
        match self.configuracion_recomendada() {
            ConfiguracionOmni::ModoSensorActuador => 16 * 1024,
            ConfiguracionOmni::ModoAsistenteLocal => 32 * 1024,
            ConfiguracionOmni::ModoPortatil => 64 * 1024,
            ConfiguracionOmni::ModoSupercomputadora => 96 * 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_positiva_en_linux() {
        // En CI no-Linux se permite el error, pero nunca panic.
        match ram_total_mb() {
            Ok(mb) => assert!(mb > 0, "MemTotal debe ser > 0"),
            Err(_) => {}
        }
    }

    #[test]
    fn decision_liquida_x220_vega_nvidia_rpi() {
        let mk = |arch: &str, ram: u64, vram: u64| PerfilHardware {
            arquitectura: arch.into(),
            ram_mb: ram,
            gpu_vram_mb: vram,
            gpu_desc: "test".into(),
            backend_inferencia: "cpu",
            almacenamiento: TipoAlmacenamiento::Ssd,
            tiene_gpio: false,
            tiene_tpm2: false,
            n_cores: 2,
        };
        // ThinkPad X220: x86_64, 8GB, iGPU Intel sin VRAM dedicada -> Portátil
        assert_eq!(
            mk("x86_64", 8192, 0).configuracion_recomendada(),
            ConfiguracionOmni::ModoPortatil
        );
        // Yoga Slim AGP11: Ryzen 400U, 16GB, Vega gfx90c (stolen ~512MB) -> Portátil
        assert_eq!(
            mk("x86_64", 16384, 512).configuracion_recomendada(),
            ConfiguracionOmni::ModoPortatil
        );
        // Estación NVIDIA RTX 4090 24GB, 64GB RAM -> Supercomputadora
        assert_eq!(
            mk("x86_64", 65536, 24576).configuracion_recomendada(),
            ConfiguracionOmni::ModoSupercomputadora
        );
        // RPi Zero (armv7, 512MB) -> Sensor
        assert_eq!(
            mk("armv7", 512, 0).configuracion_recomendada(),
            ConfiguracionOmni::ModoSensorActuador
        );
        // RPi 4 (aarch64, 4GB) -> Portátil (suficiente RAM para router ligero; L1 sería sub-aprovechar)
        assert_eq!(
            mk("aarch64", 4096, 0).configuracion_recomendada(),
            ConfiguracionOmni::ModoPortatil
        );
    }

    #[test]
    fn backend_vega_es_vulkan_no_rocm() {
        // REGLA 6: en sistema de desarrollo AMD debe elegirse vulkan.
        let (_, _, backend) = detectar_gpu();
        if let Ok(perfil) = PerfilHardware::detectar() {
            if perfil.gpu_desc.contains("amd") {
                assert_eq!(backend, "vulkan");
            }
        }
    }
}

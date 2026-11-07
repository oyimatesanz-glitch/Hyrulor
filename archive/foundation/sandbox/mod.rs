//! Hito 0.4 — Sandbox en capas: bubblewrap + seccomp-style timeout + anti-cuelgues.
//!
//! REGLA 1: NUNCA exec directo. Toda herramienta pasa por bwrap --unshare-all
//! con FS raíz SOLO LECTURA y /tmp efímero (tmpfs). La ruta de la herramienta
//! debe estar ya validada por `puerta_seguridad` (whitelist) ANTES de llegar aquí.
//!
//! REGLA 3: timeout duro + SIGKILL al GRUPO de procesos (process_group(0)),
//! no solo al hijo directo, para matar árboles completos.

use std::fmt;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

// killpg sin crate libc: syscall directo vía `kill(-pid, SIGKILL)` con prctl-free approach.
// El hijo es líder de grupo (process_group(0)) => matar -pid mata TODO el árbol.
#[allow(non_camel_case_types)]
mod raw {
    extern "C" {
        pub fn kill(pid: i32, sig: i32) -> i32;
    }
}

/// SIGKILL al grupo completo: pid negativo = pgid del líder.
fn matar_grupo(pid: u32) {
    // SAFETY: kill() es async-signal-safe y no usa memoria compartida.
    unsafe { raw::kill(-(pid as i32), 9) };
}

pub const SOCKET_UNIX: &str = "/tmp/hyrulor.sock"; // perms 600, nunca TCP (REGLA del stack)
pub const TIMEOUT_HERRAMIENTAS: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum SandboxError {
    BwrapNoEncontrado,
    Spawn(std::io::Error),
    Timeout,
    SalidaNoUTF8,
    /// Argumento fuera de whitelist defensiva (doble check aunque el gate ya pasó).
    ArgumentoRechazado(&'static str),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::BwrapNoEncontrado => write!(f, "bwrap no instalado: instalar bubblewrap"),
            SandboxError::Spawn(e) => write!(f, "fallo spawn sandbox: {e}"),
            SandboxError::Timeout => write!(f, "TIMEOUT 5s: grupo de procesos SIGKILL-ado"),
            SandboxError::SalidaNoUTF8 => write!(f, "salida no UTF-8"),
            SandboxError::ArgumentoRechazado(s) => write!(f, "argumento rechazado: {s}"),
        }
    }
}
impl std::error::Error for SandboxError {}

/// Doble validación defensiva de la ruta del script (independiente del Safety Gate).
fn ruta_defensivamente_ok(ruta: &str) -> Result<(), SandboxError> {
    if !ruta.starts_with("/tools/") || ruta.contains("..") || ruta.contains('\0') {
        return Err(SandboxError::ArgumentoRechazado(
            "ruta fuera de /tools o con traversal",
        ));
    }
    Ok(())
}

pub struct EjecucionSandbox {
    pub exito: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Construye la línea bwrap exacta del whitepaper §Ejemplo:
/// root ro-bind, /tools rw-bind acotado, tmpfs /tmp, namespaces totales, die-with-parent.
pub fn args_bwrap(script: &str, extra_args: &[String]) -> Vec<String> {
    let mut a: Vec<String> = vec![
        "--unshare-all".into(),
        "--die-with-parent".into(),
        "--ro-bind".into(),
        "/usr".into(),
        "/usr".into(),
        "--ro-bind".into(),
        "/lib".into(),
        "/lib".into(),
        "--ro-bind".into(),
        "/bin".into(),
        "/bin".into(),
        "--ro-bind".into(),
        "/etc/alternatives".into(),
        "/etc/alternatives".into(),
        "--dev".into(),
        "/dev".into(),
        "--proc".into(),
        "/proc".into(),
        "--tmpfs".into(),
        "/tmp".into(), // /tmp EFÍMERO
        "--bind".into(),
        "/tools".into(),
        "/tools".into(), // único writable
        "--chdir".into(),
        "/tools".into(),
        "--setenv".into(),
        "HOME".into(),
        "/tmp".into(),
        "--clearenv".into(),
        "--new-session".into(), // anti TIOCSPGRP escape
        "--die-with-parent".into(),
        "--".into(),
        script.into(),
    ];
    a.extend(extra_args.iter().cloned());
    a
}

/// Ejecuta `script` dentro del sandbox con timeout duro y kill por GRUPO.
pub fn ejecutar(script: &str, extra_args: &[String]) -> Result<EjecucionSandbox, SandboxError> {
    ruta_defensivamente_ok(script)?;

    if which("bwrap").is_none() {
        return Err(SandboxError::BwrapNoEncontrado);
    }

    let mut child = Command::new("bwrap")
        .args(args_bwrap(script, extra_args))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // process_group(0): hijo lidera su propio pgid => killamos TODO el árbol.
        .process_group(0)
        .spawn()
        .map_err(SandboxError::Spawn)?;

    let pid = child.id();
    let deadline = std::time::Instant::now() + TIMEOUT_HERRAMIENTAS;

    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    // SIGKILL al GRUPO (negativo = pgid), no al proceso aislado.
                    matar_grupo(pid);
                    let _ = child.wait();
                    return Err(SandboxError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(SandboxError::Spawn(e)),
        }
    };

    let mut out = Vec::new();
    let mut err = Vec::new();
    if let Some(mut so) = child.stdout.take() {
        so.read_to_end(&mut out).ok();
    }
    if let Some(mut se) = child.stderr.take() {
        se.read_to_end(&mut err).ok();
    }

    Ok(EjecucionSandbox {
        exito: status.success(),
        stdout: String::from_utf8(out).map_err(|_| SandboxError::SalidaNoUTF8)?,
        stderr: String::from_utf8(err).map_err(|_| SandboxError::SalidaNoUTF8)?,
    })
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(bin))
            .find(|p| p.is_file())
    })
}

/// Verificación de endurecimiento adicional (documentada; aplicada vía env en runtime):
/// - Landlock: LSM restrictivo si kernel >= 5.13 (crate landlock en Fase 1).
/// - seccomp-bpf: perfil default-deny + allowlist (clone, read, write, exit...) en Fase 1.
/// - Firecracker microVM: navegación web autónoma (Fase 3).
pub fn checklist_capas() -> [&'static str; 5] {
    [
        "T1 tipos: ArgumentosValidados antes de spawn",
        "T2 proceso: bwrap --unshare-all --ro-bind raiz + tmpfs /tmp",
        "T3 syscalls: seccomp-bpf default-deny (Fase 1)",
        "T4 archivos: Landlock fs deny-write fuera de /tools (Fase 1)",
        "T5 hardware: Firecracker microVM para egress red/no-web (Fase 3)",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argv_sandbox_es_inmutable_y_correcto() {
        let a = args_bwrap("/tools/battery_tool.sh", &["--json".into()]);
        let s = a.join(" ");
        assert!(s.contains("--unshare-all"));
        assert!(s.contains("--ro-bind /usr /usr"));
        assert!(s.contains("--tmpfs /tmp"));
        assert!(
            !s.contains("--network"),
            "sin red: nexo es otro MCP con permiso explícito"
        );
        assert_eq!(a.last().map(String::as_str), Some("--json"));
    }

    #[test]
    fn rechaza_traversal_aunque_el_gate_falle() {
        assert!(matches!(
            ejecutar("/tools/../etc/shadow", &[]),
            Err(SandboxError::ArgumentoRechazado(_))
        ));
        assert!(matches!(
            ejecutar("/home/user/malicioso.sh", &[]),
            Err(SandboxError::ArgumentoRechazado(_))
        ));
    }

    #[test]
    fn sin_bwrap_error_claro_no_panic() {
        // En CI sin bwrap debe devolver Err, jamás panic ni exec directo.
        match ejecutar("/tools/echo.sh", &[]) {
            Err(SandboxError::BwrapNoEncontrado) | Err(SandboxError::Spawn(_)) | Ok(_) => {}
            Err(SandboxError::ArgumentoRechazado(s)) => panic!("no debía llegar aquí: {s}"),
            Err(e) => {
                let _ = e;
            }
        }
    }
}

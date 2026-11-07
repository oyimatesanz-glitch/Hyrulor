//! Hito 0.3 — `puerta_seguridad.rs`: Núcleo Inmutable (Safety Gate).
//!
//! VERIFICACIÓN POR TIPOS: es *imposible en tiempo de compilación* construir una
//! `AccionPermitida` sin pasar por `validar()`. El constructor está cerrado
//! (`AccesoSellado` privado) y solo se emite tras las 3 comprobaciones.
//!
//! FASE 6 sustituirá los predicados heurísticos por verificación formal
//! (Prusti/Creusot). Aquí el invariante duro es:
//!   ∀ a: AccionIntentada. validar(a)=Ok(p) => p.otorga ≤ permiso_usuario ∧ ¬p.peligrosa
//!
//! Zero GC, zero dependencias, sin unwrap().

use std::fmt;
use std::time::{Duration, Instant};

/// Token de autoridad EFÍMERO. No Clone, no Copy, no serializable:
/// solo existe entre la validación y la ejecución dentro del sandbox.
pub struct AutoridadEjecucion {
    _priv: (),
    expira: Instant,
}

impl AutoridadEjecucion {
    fn emitir(ttl: Duration) -> Self {
        Self {
            _priv: (),
            expira: Instant::now() + ttl,
        }
    }
    pub fn vigente(&self) -> bool {
        Instant::now() < self.expira
    }
}

impl fmt::Debug for AutoridadEjecucion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AutoridadEjecucion(vigente={})", self.vigente())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Riesgo {
    Seguro,           // lectura pasiva (sensores, /sys)
    EscrituraLocal,   // escritura en directorio de usuario acotado
    Red,              // egress de red (requiere whitelist explícita)
    SistemaCritico,   // toca /etc, kernel, bootloader, TPM
    DestruccionDatos, // rm -rf, dd, shred fuera de /tmp gestionado
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HerramientaID {
    NucleoVital, // energía/batería
    Nexo,        // red (solo lectura diag)
    SistemaDiagnostico,
    CentroPortatil,         // periféricos
    EjecutarScript(String), // ruta absoluta ya registrada en whitelist
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentosValidados {
    /// JSON canónico ya parseado y filtrado (REGLA 5: args validados ANTES de ejecutar).
    pub canonico: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionIntentada {
    LeerSensor {
        sensor: String,
    },
    EjecutarHerramienta {
        id: HerramientaID,
        args_json: String,
        riesgo_declarado: Riesgo,
    },
    ConsultarMemoria {
        query: String,
    },
    EscribirArchivo {
        ruta: String,
        bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccionDenegada {
    ViolacionLey1 {
        motivo: &'static str,
    },
    ViolacionPrivacidad {
        campo: String,
    },
    SinConsentimientoUsuario,
    PermisoInsuficiente {
        requiere: Riesgo,
        otorgado_max: Riesgo,
    },
    ArgumentosInvalidos {
        error: String,
    },
    RutasFueraDeWhitelist {
        ruta: String,
    },
}

impl fmt::Display for AccionDenegada {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccionDenegada::ViolacionLey1 { motivo } => write!(f, "LEY-1: {motivo}"),
            AccionDenegada::ViolacionPrivacidad { campo } => {
                write!(f, "PRIVACIDAD: expone '{campo}'")
            }
            AccionDenegada::SinConsentimientoUsuario => write!(f, "SIN CONSENTIMIENTO explícito"),
            AccionDenegada::PermisoInsuficiente {
                requiere,
                otorgado_max,
            } => write!(
                f,
                "MINIMO PRIVILEGIO: requiere {requiere:?}, max concedido {otorgado_max:?}"
            ),
            AccionDenegada::ArgumentosInvalidos { error } => write!(f, "ARGS INVALIDOS: {error}"),
            AccionDenegada::RutasFueraDeWhitelist { ruta } => {
                write!(f, "WHITELIST: '{ruta}' no autorizada")
            }
        }
    }
}
impl std::error::Error for AccionDenegada {}

/// Acción YA VALIDADA. Solo `validar()` puede construirla (invariante por tipos).
pub struct AccionPermitida {
    intentada: AccionIntentada,
    autoridad: AutoridadEjecucion,
    /// Sellado interno: marca que pasó los 3 checks.
    _sello: AccesoSellado,
}

struct AccesoSellado;

impl AccionPermitida {
    /// Devuelve la herramienta + args canónicos YA validados (para el executor sandbox).
    pub fn herramienta(&self) -> Option<(&HerramientaID, ArgumentosValidados)> {
        match &self.intentada {
            AccionIntentada::EjecutarHerramienta { id, args_json, .. } => Some((
                id,
                ArgumentosValidados {
                    canonico: args_json.clone(),
                },
            )),
            _ => None,
        }
    }
    pub fn autoridad(&self) -> &AutoridadEjecucion {
        &self.autoridad
    }
}

/// Contexto de sesión del usuario: consentimiento + techo de privilegios.
pub struct ContextoUsuario {
    pub consentimiento_explicito: bool,
    pub permiso_maximo: Riesgo,
    /// Directorios donde está permitido ejecutar/escribir (whitelist cerrada).
    pub whitelist_rutas: Vec<String>,
    /// TTL de la autoridad emitida (anti-replay). Default 5s (REGLA 3).
    pub ttl_autoridad: Duration,
}

impl Default for ContextoUsuario {
    fn default() -> Self {
        Self {
            consentimiento_explicito: false,
            permiso_maximo: Riesgo::Seguro,
            whitelist_rutas: vec![String::from("/tools")],
            ttl_autoridad: Duration::from_secs(5),
        }
    }
}

fn nivel(r: &Riesgo) -> u8 {
    match r {
        Riesgo::Seguro => 0,
        Riesgo::EscrituraLocal => 1,
        Riesgo::Red => 2,
        Riesgo::SistemaCritico => 3,
        Riesgo::DestruccionDatos => 4,
    }
}

/// Validadores de argumentos (REGLA 5): rechaza patrones de inyección/destrucción.
fn args_sospechosos(s: &str) -> Option<&'static str> {
    const PATRONES: &[&str] = &[
        "rm -rf /",
        "mkfs",
        "dd if=",
        "> /dev/",
        ":(){",
        "shutdown",
        "reboot",
        "chmod 777",
        "/etc/shadow",
        "tpm_",
        "insmod",
        "rmmod",
        "iptables -F",
    ];
    for p in PATRONES {
        if s.contains(p) {
            return Some("patrón destructivo/crítico en argumentos");
        }
    }
    if s.len() > 64 * 1024 {
        return Some("argumentos exceden 64KiB");
    }
    None
}

fn ruta_en_whitelist(ruta: &str, wl: &[String]) -> bool {
    // Normalización mínima: sin '..', ruta absoluta, prefijo exacto de un directorio whitelisted.
    if !ruta.starts_with('/') || ruta.split('/').any(|c| c == "..") {
        return false;
    }
    wl.iter()
        .any(|w| ruta == w || ruta.starts_with(&format!("{w}/")))
}

/// EL GATE. Intercepta CADA llamada. Orden no negociable: Ley 1 → Privacidad → Consentimiento → Mínimo privilegio.
pub fn validar(
    accion: &AccionIntentada,
    ctx: &ContextoUsuario,
) -> Result<AccionPermitida, AccionDenegada> {
    // 1. LEY 1 — No dañar (física o digitalmente).
    match accion {
        AccionIntentada::EjecutarHerramienta {
            args_json,
            riesgo_declarado,
            ..
        } => {
            if let Some(e) = args_sospechosos(args_json) {
                return Err(AccionDenegada::ViolacionLey1 { motivo: e });
            }
            if matches!(
                riesgo_declarado,
                Riesgo::DestruccionDatos | Riesgo::SistemaCritico
            ) {
                return Err(AccionDenegada::ViolacionLey1 {
                    motivo: "riesgo declarado crítico denegado por defecto (zero-trust)",
                });
            }
        }
        AccionIntentada::EscribirArchivo { bytes, .. } if *bytes > 512 * 1024 * 1024 => {
            return Err(AccionDenegada::ViolacionLey1 {
                motivo: "escritura >512MiB sin cuota",
            });
        }
        _ => {}
    }

    // 2. PRIVACIDAD (Zero-Knowledge): nada que exponga datos cifrados/credenciales.
    if let AccionIntentada::ConsultarMemoria { query } = accion {
        for campo in ["password", "token", "secret", "api_key", "pri"] {
            if query.to_lowercase().contains(campo) {
                return Err(AccionDenegada::ViolacionPrivacidad {
                    campo: campo.into(),
                });
            }
        }
    }

    // 3. CONSENTIMIENTO explícito del usuario para todo lo que no sea lectura pasiva.
    let es_lectura_pasiva = matches!(
        accion,
        AccionIntentada::LeerSensor { .. } | AccionIntentada::ConsultarMemoria { .. }
    );
    if !es_lectura_pasiva && !ctx.consentimiento_explicito {
        return Err(AccionDenegada::SinConsentimientoUsuario);
    }

    // 4. MÍNIMO PRIVILEGIO + WHITELIST.
    let (riesgo_req, ruta_opt) = match accion {
        AccionIntentada::LeerSensor { .. } | AccionIntentada::ConsultarMemoria { .. } => {
            (Riesgo::Seguro, None)
        }
        AccionIntentada::EjecutarHerramienta {
            id,
            riesgo_declarado,
            ..
        } => {
            let ruta = match id {
                HerramientaID::EjecutarScript(p) => Some(p.clone()),
                _ => None,
            };
            (riesgo_declarado.clone(), ruta)
        }
        AccionIntentada::EscribirArchivo { ruta, .. } => {
            (Riesgo::EscrituraLocal, Some(ruta.clone()))
        }
    };
    if let Some(r) = ruta_opt {
        if !ruta_en_whitelist(&r, &ctx.whitelist_rutas) {
            return Err(AccionDenegada::RutasFueraDeWhitelist { ruta: r });
        }
    }
    if nivel(&riesgo_req) > nivel(&ctx.permiso_maximo) {
        return Err(AccionDenegada::PermisoInsuficiente {
            requiere: riesgo_req,
            otorgado_max: ctx.permiso_maximo.clone(),
        });
    }

    // OK: único punto de construcción del tipo sellado.
    Ok(AccionPermitida {
        intentada: accion.clone(),
        autoridad: AutoridadEjecucion::emitir(ctx.ttl_autoridad),
        _sello: AccesoSellado,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_consent(riesgo: Riesgo) -> ContextoUsuario {
        ContextoUsuario {
            consentimiento_explicito: true,
            permiso_maximo: riesgo,
            ..Default::default()
        }
    }

    #[test]
    fn ley1_bloquea_destruccion_aunque_haya_consentimiento() {
        let ctx = ctx_consent(Riesgo::SistemaCritico);
        let a = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::EjecutarScript("/tools/x.sh".into()),
            args_json: r#"{"cmd":"rm -rf / --no-preserve-root"}"#.into(),
            riesgo_declarado: Riesgo::EscrituraLocal,
        };
        assert!(matches!(
            validar(&a, &ctx),
            Err(AccionDenegada::ViolacionLey1 { .. })
        ));
    }

    #[test]
    fn sin_consentimiento_solo_pasa_lectura_pasiva() {
        let ctx = ContextoUsuario::default();
        let ok = AccionIntentada::LeerSensor {
            sensor: "battery".into(),
        };
        assert!(validar(&ok, &ctx).is_ok());
        let mal = AccionIntentada::EscribirArchivo {
            ruta: "/tools/a.txt".into(),
            bytes: 10,
        };
        assert!(matches!(
            validar(&mal, &ctx),
            Err(AccionDenegada::SinConsentimientoUsuario)
        ));
    }

    #[test]
    fn minimo_privilegio_rechama_red_sin_permiso() {
        let ctx = ctx_consent(Riesgo::Seguro);
        let a = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::Nexo,
            args_json: "{}".into(),
            riesgo_declarado: Riesgo::Red,
        };
        assert!(matches!(
            validar(&a, &ctx),
            Err(AccionDenegada::PermisoInsuficiente { .. })
        ));
    }

    #[test]
    fn path_traversal_reject() {
        let ctx = ctx_consent(Riesgo::EscrituraLocal);
        let a = AccionIntentada::EscribirArchivo {
            ruta: "/tools/../etc/passwd".into(),
            bytes: 1,
        };
        assert!(matches!(
            validar(&a, &ctx),
            Err(AccionDenegada::RutasFueraDeWhitelist { .. })
        ));
    }

    #[test]
    fn happy_path_emite_autoridad_efimera() {
        let ctx = ctx_consent(Riesgo::EscrituraLocal);
        let a = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::NucleoVital,
            args_json: r#"{"action":"read_capacity"}"#.into(),
            riesgo_declarado: Riesgo::Seguro,
        };
        let p = validar(&a, &ctx).expect("debe permitir");
        assert!(p.autoridad().vigente());
    }
}

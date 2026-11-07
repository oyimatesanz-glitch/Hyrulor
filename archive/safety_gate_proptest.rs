//! Tests property-based (proptest) del Safety Gate: el invariante duro debe
//! sostenerse para CUALQUIER entrada generada adversarialmente.
//! Invariante: validar(a) = Ok(p)  =>  nivel(riesgo_req) <= nivel(permiso_max)
//!            ∧ ruta solicitada ∈ whitelist ∧ ¬patrón destructivo en args.

use hyory::foundation::puerta_seguridad::*;
use proptest::prelude::*;
use std::time::Duration;

fn arb_texto(max_len: usize) -> impl Strategy<Value = String> {
    let rango = 1..max_len;
    ".{0,40}".prop_map(move |s: String| s.chars().take(rango.end).collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn ningun_input_genera_permiso_con_ruta_malformada(
        ruta in arb_texto(80),
        args in arb_texto(200),
        consentimiento in any::<bool>(),
        permiso in prop::sample::select(&[Riesgo::Seguro, Riesgo::EscrituraLocal, Riesgo::Red][..]),
    ) {
        let ctx = ContextoUsuario {
            consentimiento_explicito: consentimiento,
            permiso_maximo: permiso.clone(),
            whitelist_rutas: vec!["/tools".into()],
            ttl_autoridad: Duration::from_secs(5),
        };
        let accion = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::EjecutarScript(ruta.clone()),
            args_json: args,
            riesgo_declarado: Riesgo::Seguro,
        };
        if let Ok(_permitida) = validar(&accion, &ctx) {
            // SI se permitió, la ruta DEBE ser absoluta, sin '..', bajo /tools/.
            prop_assert!(ruta.starts_with('/'));
            prop_assert!(!ruta.split('/').any(|c| c == ".."));
            prop_assert!(ruta.starts_with("/tools/"));
        }
    }

    #[test]
    fn ley1_siempre_gana_a_la_fuerza_bruta(cmd_fragmentos in prop::collection::vec(
        prop::sample::select(&["rm -rf /", "mkfs", "dd if=", "; reboot", "insmod"][..]),
        1..4,
    )) {
        let ctx = ContextoUsuario {
            consentimiento_explicito: true,
            permiso_maximo: Riesgo::DestruccionDatos, // aunque el usuario conceda TODO
            whitelist_rutas: vec!["/tools".into()],
            ttl_autoridad: Duration::from_secs(5),
        };
        let joined = cmd_fragmentos.join(" ");
        let accion = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::NucleoVital,
            args_json: joined.clone(),
            riesgo_declarado: Riesgo::Seguro,
        };
        let r = validar(&accion, &ctx);
        if joined.contains("rm -rf /") || joined.contains("mkfs") || joined.contains("dd if=")
            || joined.contains("reboot") || joined.contains("insmod") {
            prop_assert!(matches!(r, Err(AccionDenegada::ViolacionLey1 { .. })),
                "LEY-1 violada: se permitió {:?}", joined);
        }
    }

    #[test]
    fn sin_consentimiento_nunca_se_ejecuta_herramientas(
        args in arb_texto(50),
        permiso in prop::sample::select(&[Riesgo::Seguro, Riesgo::EscrituraLocal, Riesgo::Red, Riesgo::SistemaCritico][..]),
    ) {
        let ctx = ContextoUsuario {
            consentimiento_explicito: false,
            permiso_maximo: permiso,
            whitelist_rutas: vec!["/tools".into()],
            ttl_autoridad: Duration::from_secs(5),
        };
        let accion = AccionIntentada::EjecutarHerramienta {
            id: HerramientaID::SistemaDiagnostico,
            args_json: args,
            riesgo_declarado: Riesgo::Seguro,
        };
        prop_assert!(matches!(validar(&accion, &ctx), Err(AccionDenegada::SinConsentimientoUsuario)));
    }
}

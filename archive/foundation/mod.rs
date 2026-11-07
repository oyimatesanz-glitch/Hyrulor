//! Componentes reutilizables importados del núcleo Hyrulor Genesis.
//!
//! El daemon de fase 2 sigue usando sus módulos actuales; estos componentes
//! aportan detección de hardware, validación de acciones, memoria síncrona
//! auditable y un executor sandbox reforzado para integración progresiva.

pub mod inicializacion;
pub mod memoria;
pub mod puerta_seguridad;
pub mod sandbox;

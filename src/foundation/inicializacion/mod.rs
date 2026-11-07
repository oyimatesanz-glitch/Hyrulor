//! Hito 0.2 — Detección universal de hardware vía sysfs/procfs.
//! Sin dependencias externas: parseo directo de /proc y /sys.
//! Mismo binario en ThinkPad X220 (Intel), Yoga Slim (Vega gfx90c), NVIDIA, RPi.

pub mod detector_hw;

pub use detector_hw::{ConfiguracionOmni, DetectorError, PerfilHardware};

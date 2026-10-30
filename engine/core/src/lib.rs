//! `engine_core`: base del motor.
//!
//! Matemáticas, tiempo en ticks enteros, handles con generación, serialización
//! y perfilado. No conoce nada del juego ni de los gráficos.
//!
//! Spec: `specs/architecture/engine_core.md`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod collections;
pub mod handle;
pub mod math;
pub mod time;
pub mod version;

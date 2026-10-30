//! Tipos matemáticos del motor: reexport completo de `glam`.
//!
//! `engine_sim` y `game_rules` los toman de aquí y no declaran `glam`. El
//! determinismo exigido es en la misma máquina (regla 13), así que la ruta SSE2
//! de `glam` vale; `-C target-cpu` y `-C target-feature` quedan prohibidos en
//! `.cargo/config.toml` para que el FMA no cambie los resultados entre builds.

/// Tipos matemáticos del motor. `engine_sim` y `game_rules` los toman de aquí.
pub use glam::*;

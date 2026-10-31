//! `engine_sim`: simulación determinista a tick fijo.
//!
//! Tablas SoA con handles comunes, grid espacial de orden estable, navegación
//! por sectores, scheduler de fases paralelas y pipeline de eventos ordenados.
//! No conoce ni toca gráficos (regla 2 de la constitución).
//!
//! Spec: `specs/architecture/engine_sim.md`.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod buffer;
pub mod desync;
pub mod events;
pub mod grid;
pub mod orders;
pub mod parallel;
pub mod replay;
pub mod save;
pub mod sim;
pub mod table;
pub mod world;

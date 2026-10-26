//! `engine_sim`: simulación determinista a tick fijo.
//!
//! Tablas SoA con handles comunes, grid espacial de orden estable, navegación
//! por sectores, scheduler de fases paralelas y pipeline de eventos ordenados.
//! No conoce ni toca gráficos (regla 2 de la constitución).
//!
//! Spec: `specs/architecture/engine_sim.md`.
#![forbid(unsafe_code)]

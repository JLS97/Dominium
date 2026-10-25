//! `engine_render`: render GPU-driven con `wgpu`.
//!
//! Instancing, skinning en GPU con esqueleto horneado, impostores, terreno por
//! chunks, culling y LOD en GPU. Interpola en el vertex shader entre los dos
//! últimos snapshots de la simulación. Cero trabajo por unidad en la CPU.
//!
//! Único crate con permiso para `unsafe` (regla 3 de la constitución), solo
//! donde el mapeo de buffers lo exija y siempre con un comentario `SAFETY:`.
//!
//! Spec: `specs/architecture/engine_render.md` (pendiente).
#![allow(unsafe_code)]

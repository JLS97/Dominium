//! Handle común a todas las tablas del motor (ADR-0004): slot más generación.
//!
//! Este archivo no contiene ningún flotante: CA-2 lee su texto.
// Contrato A: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T02 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T02")]

use core::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// Identidad de cualquier entidad de cualquier tabla (ADR-0004): slot más generación.
/// Garantiza: ocupa 8 bytes, `Option<Handle>` también ocupa 8 bytes, el orden es por (slot, generación)
/// y coincide con el orden numérico de `to_raw()`. La generación nunca es 0, así que `to_raw()` nunca es 0.
/// La tabla que emite el handle es la única que puede validarlo: este tipo no sabe de qué tabla viene.
///
/// La codificación `serde` es la de sus dos campos en orden, `slot` y `generation`, los dos como `u32`.
/// Deserializar una generación 0 es un error del deserializador.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Handle {
    slot: u32,
    generation: NonZeroU32,
}

impl Handle {
    /// Generación con la que nace un slot: 1.
    pub const FIRST_GENERATION: NonZeroU32 = NonZeroU32::MIN;

    /// Construye un handle a partir de su slot y su generación.
    #[must_use]
    pub const fn new(slot: u32, generation: NonZeroU32) -> Handle {
        panic!("P01-T02: pendiente de implementar")
    }

    /// El slot del handle.
    #[must_use]
    pub const fn slot(self) -> u32 {
        panic!("P01-T02: pendiente de implementar")
    }

    /// `slot` como `usize`. Qué indexa lo fija la spec de `engine_sim`.
    #[must_use]
    pub const fn index(self) -> usize {
        panic!("P01-T02: pendiente de implementar")
    }

    /// La generación del handle. Nunca es 0.
    #[must_use]
    pub const fn generation(self) -> NonZeroU32 {
        panic!("P01-T02: pendiente de implementar")
    }

    /// `(slot << 32) | generation`. Es la forma que entra en el hash de estado y en la aleatoriedad sin estado.
    #[must_use]
    pub const fn to_raw(self) -> u64 {
        panic!("P01-T02: pendiente de implementar")
    }

    /// Inversa de `to_raw`. `None` si los 32 bits bajos son 0.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Option<Handle> {
        panic!("P01-T02: pendiente de implementar")
    }

    /// La generación que sigue a `generation` cuando un slot se libera: suma 1 y de `u32::MAX` vuelve a 1, nunca a 0.
    #[must_use]
    pub const fn next_generation(generation: NonZeroU32) -> NonZeroU32 {
        panic!("P01-T02: pendiente de implementar")
    }
}

/// `handle.to_raw()`, o 0 si no hay handle. Para el sujeto de `rng::random_u64` cuando la extracción no es de una entidad.
#[must_use]
pub const fn raw_or_zero(handle: Option<Handle>) -> u64 {
    panic!("P01-T02: pendiente de implementar")
}

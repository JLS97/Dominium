//! Handle común a todas las tablas del motor (ADR-0004): slot más generación.
//!
//! Este archivo no contiene ningún flotante: CA-2 lee su texto.

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
        Handle { slot, generation }
    }

    /// El slot del handle.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// `slot` como `usize`. Qué indexa lo fija la spec de `engine_sim`.
    #[must_use]
    pub const fn index(self) -> usize {
        self.slot as usize
    }

    /// La generación del handle. Nunca es 0.
    #[must_use]
    pub const fn generation(self) -> NonZeroU32 {
        self.generation
    }

    /// `(slot << 32) | generation`. Es la forma que entra en el hash de estado y en la aleatoriedad sin estado.
    #[must_use]
    pub const fn to_raw(self) -> u64 {
        ((self.slot as u64) << 32) | (self.generation.get() as u64)
    }

    /// Inversa de `to_raw`. `None` si los 32 bits bajos son 0.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Option<Handle> {
        // El desplazamiento y la máscara dejan cada mitad dentro de 32 bits antes del cast.
        let slot = (raw >> 32) as u32;
        match NonZeroU32::new((raw & 0xFFFF_FFFF) as u32) {
            Some(generation) => Some(Handle { slot, generation }),
            None => None,
        }
    }

    /// La generación que sigue a `generation` cuando un slot se libera: suma 1 y de `u32::MAX` vuelve a 1, nunca a 0.
    #[must_use]
    pub const fn next_generation(generation: NonZeroU32) -> NonZeroU32 {
        match generation.checked_add(1) {
            Some(next) => next,
            None => Handle::FIRST_GENERATION,
        }
    }
}

/// `handle.to_raw()`, o 0 si no hay handle. Es lo que `rng::random_u64` hashea como sujeto, reciba `Some` o `None`.
#[must_use]
pub const fn raw_or_zero(handle: Option<Handle>) -> u64 {
    match handle {
        Some(handle) => handle.to_raw(),
        None => 0,
    }
}

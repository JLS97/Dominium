//! Aleatoriedad sin estado (regla 13, ADR-0009): `hash(semilla, tick, sujeto, propósito)`.
//!
//! No hay generador que avanzar ni estado canónico que guardar: cada extracción
//! es una función pura de la semilla de la partida, el tick, el sujeto y el
//! propósito, idéntica en cualquier hilo y en cualquier perfil de compilación.
//! Los ids de `Purpose` los fija quien extrae aleatoriedad, nunca este crate.
// Contrato B1: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T06 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T06")]

use serde::{Deserialize, Serialize};

use crate::time::Tick;

/// Semilla de la partida. Es canónica: va en el guardado y en el replay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Seed(pub u64);

/// Para qué se extrae aleatoriedad: un id de 32 bits que fija quien la usa, y un índice para la n-ésima extracción
/// del mismo propósito en el mismo tick y sujeto (Q-009). La aleatoriedad no tiene estado canónico.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Purpose(pub u64);

impl Purpose {
    /// `(id << 32)`, índice 0.
    #[must_use]
    pub const fn new(id: u32) -> Purpose {
        panic!("P01-T06: pendiente de implementar")
    }

    /// Mismo id, índice `index` en los 32 bits bajos.
    #[must_use]
    pub const fn nth(self, index: u32) -> Purpose {
        panic!("P01-T06: pendiente de implementar")
    }

    /// El id del propósito: los 32 bits altos.
    #[must_use]
    pub const fn id(self) -> u32 {
        panic!("P01-T06: pendiente de implementar")
    }

    /// El índice de la extracción: los 32 bits bajos.
    #[must_use]
    pub const fn index(self) -> u32 {
        panic!("P01-T06: pendiente de implementar")
    }
}

/// `xxh3_64` con semilla 0 sobre 32 bytes: `seed.0`, `tick.0`, `subject` y `purpose.0`, cada uno como `u64`
/// little-endian y en ese orden (ADR-0009). Pura: mismas entradas, mismo valor, en cualquier hilo. No asigna.
/// `subject` es `Handle::to_raw()` para una entidad, o `handle::raw_or_zero(None)` para una extracción global.
#[must_use]
pub fn random_u64(seed: Seed, tick: Tick, subject: u64, purpose: Purpose) -> u64 {
    todo!("P01-T06")
}

/// Los 64 bits de una extracción, con las conversiones habituales. Todas son deterministas y no asignan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draw(u64);

impl Draw {
    /// Envuelve los 64 bits de una extracción.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Draw {
        panic!("P01-T06: pendiente de implementar")
    }

    /// Los 64 bits tal cual.
    #[must_use]
    pub const fn bits(self) -> u64 {
        panic!("P01-T06: pendiente de implementar")
    }

    /// En `[0, 1)`: los 24 bits altos divididos por 2^24. Exacto en `f32`.
    #[must_use]
    pub fn unit_f32(self) -> f32 {
        todo!("P01-T06")
    }

    /// En `[0, 1)`: los 53 bits altos divididos por 2^53. Exacto en `f64`.
    #[must_use]
    pub fn unit_f64(self) -> f64 {
        todo!("P01-T06")
    }

    /// En `[0, n)`: `((bits as u128 * n as u128) >> 64) as u32`. Exige `n > 0`; si no, pánico `rango vacío`.
    #[must_use]
    pub const fn below(self, n: u32) -> u32 {
        panic!("P01-T06: pendiente de implementar")
    }

    /// `below(denominator) < numerator`. Exige `denominator > 0`; si no, pánico `rango vacío`.
    #[must_use]
    pub const fn chance(self, numerator: u32, denominator: u32) -> bool {
        panic!("P01-T06: pendiente de implementar")
    }

    /// `(bits >> 32, bits & 0xFFFF_FFFF)`, para quien necesite dos valores de 32 bits de una extracción.
    #[must_use]
    pub const fn split(self) -> (u32, u32) {
        panic!("P01-T06: pendiente de implementar")
    }
}

/// `Draw::from_bits(random_u64(seed, tick, subject, purpose))`.
#[must_use]
pub fn draw(seed: Seed, tick: Tick, subject: u64, purpose: Purpose) -> Draw {
    todo!("P01-T06")
}

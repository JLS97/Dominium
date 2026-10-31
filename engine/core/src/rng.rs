//! Aleatoriedad sin estado (regla 13, ADR-0009): `hash(semilla, tick, sujeto, propósito)`.
//!
//! No hay generador que avanzar ni estado canónico que guardar: cada extracción
//! es una función pura de la semilla de la partida, el tick, el sujeto y el
//! propósito, idéntica en cualquier hilo y en cualquier perfil de compilación.
//! Los ids de `Purpose` los fija quien extrae aleatoriedad, nunca este crate.

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_64;

use crate::handle::{Handle, raw_or_zero};
use crate::time::Tick;

/// Semilla de la partida. Es canónica: va en el guardado y en el replay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Seed(pub u64);

/// Para qué se extrae aleatoriedad: un id de 32 bits que fija quien la usa, y un índice para la n-ésima extracción
/// del mismo propósito en el mismo tick y sujeto (Q-009). La aleatoriedad no tiene estado canónico. Sin `Default`:
/// el propósito se declara siempre con `new`, para que el id 0 no se obtenga por omisión (Q-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Purpose(pub u64);

/// Máscara de los 32 bits bajos de un `u64`: el índice de `Purpose` y la mitad baja de `Draw::split`.
const LOW_32: u64 = 0xFFFF_FFFF;

impl Purpose {
    /// `(id << 32)`, índice 0.
    #[must_use]
    pub const fn new(id: u32) -> Purpose {
        Purpose((id as u64) << 32)
    }

    /// Mismo id, índice `index` en los 32 bits bajos. `index` es la posición de la extracción dentro del bucle del
    /// tick (la n-ésima del mismo propósito y sujeto), nunca un contador guardado en la entidad, que sería estado no
    /// canónico (regla 6, Q-019). Se cuenta en el orden canónico de la fase, nunca por la posición densa de una
    /// entidad en su tabla, que cambia tras un swap-remove, ni por el orden de llegada de los hilos; una extracción
    /// por entidad lleva su handle como sujeto, no `None` con el índice de la entidad (Q-022).
    #[must_use]
    pub const fn nth(self, index: u32) -> Purpose {
        // Sustituye los 32 bits bajos: `nth` sobre `nth` se queda con el último índice, no con su OR.
        Purpose((self.0 & !LOW_32) | (index as u64))
    }

    /// El id del propósito: los 32 bits altos.
    #[must_use]
    pub const fn id(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// El índice de la extracción: los 32 bits bajos.
    #[must_use]
    pub const fn index(self) -> u32 {
        (self.0 & LOW_32) as u32
    }
}

/// `xxh3_64` con semilla 0 sobre 32 bytes: `seed.0`, `tick.0`, `handle::raw_or_zero(subject)` y `purpose.0`, cada uno
/// como `u64` little-endian y en ese orden (ADR-0009). Pura: mismas entradas, mismo valor, en cualquier hilo. No asigna.
/// `subject` es el handle de la entidad, o `None` para una extracción global. El tipo obliga a que el sujeto sea un
/// handle emitido por su tabla, no un índice denso de slot, que cambia tras un swap-remove y haría que la misma
/// entidad extrajera valores distintos según la historia de borrados (Q-018, Q-022).
#[must_use]
pub fn random_u64(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> u64 {
    // Los 32 bytes viven en la pila: ninguna asignación (regla 19).
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&seed.0.to_le_bytes());
    bytes[8..16].copy_from_slice(&tick.0.to_le_bytes());
    bytes[16..24].copy_from_slice(&raw_or_zero(subject).to_le_bytes());
    bytes[24..32].copy_from_slice(&purpose.0.to_le_bytes());
    xxh3_64(&bytes)
}

/// Los 64 bits de una extracción, con las conversiones habituales. Todas son deterministas y no asignan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draw(u64);

/// 2^24, divisor de `unit_f32`. Es potencia de dos, así que la división es exacta.
const TWO_POW_24: f32 = 16_777_216.0;
/// 2^53, divisor de `unit_f64`. Es potencia de dos, así que la división es exacta.
const TWO_POW_53: f64 = 9_007_199_254_740_992.0;

impl Draw {
    /// Envuelve los 64 bits de una extracción.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Draw {
        Draw(bits)
    }

    /// Los 64 bits tal cual.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// En `[0, 1)`: los 24 bits altos divididos por 2^24. Exacto en `f32`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "los 24 bits altos caben exactos en la mantisa de f32"
    )]
    pub fn unit_f32(self) -> f32 {
        (self.0 >> 40) as f32 / TWO_POW_24
    }

    /// En `[0, 1)`: los 53 bits altos divididos por 2^53. Exacto en `f64`.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "los 53 bits altos caben exactos en la mantisa de f64"
    )]
    pub fn unit_f64(self) -> f64 {
        (self.0 >> 11) as f64 / TWO_POW_53
    }

    /// En `[0, n)`: `((bits as u128 * n as u128) >> 64) as u32`. Exige `n > 0`; si no, pánico `rango vacío`.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "el producto de 64 por 32 bits desplazado 64 es menor que n, y cabe en u32 por construcción"
    )]
    pub const fn below(self, n: u32) -> u32 {
        assert!(n > 0, "rango vacío");
        ((self.0 as u128 * n as u128) >> 64) as u32
    }

    /// `below(denominator) < numerator`. Exige `denominator > 0`; si no, pánico `rango vacío`.
    #[must_use]
    pub const fn chance(self, numerator: u32, denominator: u32) -> bool {
        self.below(denominator) < numerator
    }

    /// `(bits >> 32, bits & 0xFFFF_FFFF)`, para quien necesite dos valores de 32 bits de una extracción.
    #[must_use]
    pub const fn split(self) -> (u32, u32) {
        ((self.0 >> 32) as u32, (self.0 & LOW_32) as u32)
    }
}

/// `Draw::from_bits(random_u64(seed, tick, subject, purpose))`.
#[must_use]
pub fn draw(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> Draw {
    Draw::from_bits(random_u64(seed, tick, subject, purpose))
}

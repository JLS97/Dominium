//! Tiempo de la simulación en ticks enteros (reglas 4 y 17, ADR-0006).
//!
//! `Tick` es un instante y `Ticks` una duración. Toda la aritmética hace
//! comprobaciones explícitas (`checked_*` y `assert!`) con mensajes fijos, de
//! modo que dev y release se comportan igual. Este archivo no contiene ningún
//! flotante: CA-2 lee su texto.
// Contrato A: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T02 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T02")]

use core::ops::{Add, AddAssign, Mul, Sub};
use core::time::Duration;

use serde::{Deserialize, Serialize};

/// Ticks de simulación por segundo. Es el único lugar del código donde aparece el 20 (ADR-0006).
pub const TICK_RATE_HZ: u32 = 20;
/// Ticks que caben en un segundo de juego, para expresar duraciones largas en tests y datos.
pub const TICKS_PER_SECOND: Ticks = Ticks(TICK_RATE_HZ as u64);
/// Duración de un tick en tiempo de pared, exacta: 50 ms. Solo la usan los crates que tocan el reloj real.
pub const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE_HZ as u64);

/// Instante de la simulación: número de ticks completados desde el inicio de la partida.
/// Garantiza orden total y codificación `postcard` como `u64` varint.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(transparent)]
pub struct Tick(pub u64);

/// Duración en ticks: la diferencia entre dos `Tick` (regla 17). Nunca negativa.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(transparent)]
pub struct Ticks(pub u64);

impl Tick {
    /// Estado inicial, antes del primer tick.
    pub const ZERO: Tick = Tick(0);

    /// El tick siguiente. Pánico `desbordamiento de ticks` si `self.0 == u64::MAX`.
    #[must_use]
    pub const fn next(self) -> Tick {
        panic!("P01-T02: pendiente de implementar")
    }

    /// Ticks transcurridos desde `earlier`. Exige `earlier <= self`; si no, pánico `retroceso de ticks`.
    #[must_use]
    pub const fn since(self, earlier: Tick) -> Ticks {
        panic!("P01-T02: pendiente de implementar")
    }

    /// Como `since`, pero devuelve `None` en vez de pánico cuando `earlier > self`.
    #[must_use]
    pub const fn checked_since(self, earlier: Tick) -> Option<Ticks> {
        panic!("P01-T02: pendiente de implementar")
    }

    /// Cierto si `self` es múltiplo de `period` (el tick 0 lo es de cualquier periodo).
    /// Exige `period > 0`; si no, pánico `periodo cero`.
    #[must_use]
    pub const fn is_multiple_of(self, period: Ticks) -> bool {
        panic!("P01-T02: pendiente de implementar")
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Tick {
    type Output = Tick;

    fn add(self, rhs: Ticks) -> Tick {
        todo!("P01-T02")
    }
}

impl AddAssign<Ticks> for Tick {
    fn add_assign(&mut self, rhs: Ticks) {
        todo!("P01-T02")
    }
}

/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Tick {
    type Output = Tick;

    fn sub(self, rhs: Ticks) -> Tick {
        todo!("P01-T02")
    }
}

/// Igual que `since`.
impl Sub<Tick> for Tick {
    type Output = Ticks;

    fn sub(self, rhs: Tick) -> Ticks {
        todo!("P01-T02")
    }
}

impl Ticks {
    /// Duración nula.
    pub const ZERO: Ticks = Ticks(0);
    /// Un solo tick.
    pub const ONE: Ticks = Ticks(1);

    /// `self` menos `rhs`, o `None` si `rhs > self`.
    #[must_use]
    pub const fn checked_sub(self, rhs: Ticks) -> Option<Ticks> {
        panic!("P01-T02: pendiente de implementar")
    }

    /// Duración en tiempo de pared: `self * TICK_DURATION`. Para la app y las herramientas;
    /// nunca para la simulación. No desborda: `u64::MAX` ticks son menos de 2^60 segundos y
    /// `Duration` llega a 2^64 segundos.
    #[must_use]
    pub const fn to_wall_duration(self) -> Duration {
        panic!("P01-T02: pendiente de implementar")
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Ticks {
    type Output = Ticks;

    fn add(self, rhs: Ticks) -> Ticks {
        todo!("P01-T02")
    }
}

impl AddAssign<Ticks> for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        todo!("P01-T02")
    }
}

/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Ticks {
    type Output = Ticks;

    fn sub(self, rhs: Ticks) -> Ticks {
        todo!("P01-T02")
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Mul<u64> for Ticks {
    type Output = Ticks;

    fn mul(self, rhs: u64) -> Ticks {
        todo!("P01-T02")
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Mul<Ticks> for u64 {
    type Output = Ticks;

    fn mul(self, rhs: Ticks) -> Ticks {
        todo!("P01-T02")
    }
}

//! Tiempo de la simulación en ticks enteros (reglas 4 y 17, ADR-0006).
//!
//! `Tick` es un instante y `Ticks` una duración. Toda la aritmética hace
//! comprobaciones explícitas (`checked_*` y `assert!`) con mensajes fijos, de
//! modo que dev y release se comportan igual. Este archivo no contiene ningún
//! flotante: CA-2 lee su texto.

use core::ops::{Add, AddAssign, Mul, Sub};
use core::time::Duration;

use serde::{Deserialize, Serialize};

/// Ticks de simulación por segundo. Es el único lugar del código donde aparece el 20 (ADR-0006).
pub const TICK_RATE_HZ: u32 = 20;
/// Ticks que caben en un segundo de juego, para expresar duraciones largas en tests y datos.
pub const TICKS_PER_SECOND: Ticks = Ticks(TICK_RATE_HZ as u64);
/// Duración de un tick en tiempo de pared, exacta: 50 ms. Solo la usan los crates que tocan el reloj real.
pub const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE_HZ as u64);

// `to_wall_duration` reparte los ticks en segundos enteros más un resto menor que un segundo;
// eso solo es exacto si la frecuencia divide al número de nanosegundos de un segundo.
const _: () = assert!(
    1_000_000_000 % TICK_RATE_HZ == 0,
    "TICK_RATE_HZ debe dividir a 1_000_000_000 para que TICK_DURATION sea exacta"
);

/// Mensaje de pánico cuando una suma o un producto de ticks no cabe en `u64`.
const OVERFLOW_MSG: &str = "desbordamiento de ticks";
/// Mensaje de pánico cuando se resta más de lo transcurrido.
const UNDERFLOW_MSG: &str = "retroceso de ticks";
/// Mensaje de pánico cuando `is_multiple_of` recibe un periodo nulo.
const ZERO_PERIOD_MSG: &str = "periodo cero";

/// Suma comprobada; pánico `desbordamiento de ticks` si no cabe en `u64`.
const fn add_or_panic(lhs: u64, rhs: u64) -> u64 {
    match lhs.checked_add(rhs) {
        Some(sum) => sum,
        None => panic!("{}", OVERFLOW_MSG),
    }
}

/// Resta comprobada; pánico `retroceso de ticks` si `rhs > lhs`.
const fn sub_or_panic(lhs: u64, rhs: u64) -> u64 {
    match lhs.checked_sub(rhs) {
        Some(difference) => difference,
        None => panic!("{}", UNDERFLOW_MSG),
    }
}

/// Producto comprobado; pánico `desbordamiento de ticks` si no cabe en `u64`.
const fn mul_or_panic(lhs: u64, rhs: u64) -> u64 {
    match lhs.checked_mul(rhs) {
        Some(product) => product,
        None => panic!("{}", OVERFLOW_MSG),
    }
}

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
        Tick(add_or_panic(self.0, 1))
    }

    /// Ticks transcurridos desde `earlier`. Exige `earlier <= self`; si no, pánico `retroceso de ticks`.
    #[must_use]
    pub const fn since(self, earlier: Tick) -> Ticks {
        Ticks(sub_or_panic(self.0, earlier.0))
    }

    /// Como `since`, pero devuelve `None` en vez de pánico cuando `earlier > self`.
    #[must_use]
    pub const fn checked_since(self, earlier: Tick) -> Option<Ticks> {
        match self.0.checked_sub(earlier.0) {
            Some(elapsed) => Some(Ticks(elapsed)),
            None => None,
        }
    }

    /// Cierto si `self` es múltiplo de `period` (el tick 0 lo es de cualquier periodo).
    /// Exige `period > 0`; si no, pánico `periodo cero`.
    #[must_use]
    pub const fn is_multiple_of(self, period: Ticks) -> bool {
        assert!(period.0 != 0, "{}", ZERO_PERIOD_MSG);
        self.0.is_multiple_of(period.0)
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Tick {
    type Output = Tick;

    fn add(self, rhs: Ticks) -> Tick {
        Tick(add_or_panic(self.0, rhs.0))
    }
}

impl AddAssign<Ticks> for Tick {
    fn add_assign(&mut self, rhs: Ticks) {
        *self = *self + rhs;
    }
}

/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Tick {
    type Output = Tick;

    fn sub(self, rhs: Ticks) -> Tick {
        Tick(sub_or_panic(self.0, rhs.0))
    }
}

/// Igual que `since`.
impl Sub<Tick> for Tick {
    type Output = Ticks;

    fn sub(self, rhs: Tick) -> Ticks {
        self.since(rhs)
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
        match self.0.checked_sub(rhs.0) {
            Some(difference) => Some(Ticks(difference)),
            None => None,
        }
    }

    /// Duración en tiempo de pared: `self * TICK_DURATION`. Para la app y las herramientas;
    /// nunca para la simulación. No desborda: `u64::MAX` ticks son menos de 2^60 segundos y
    /// `Duration` llega a 2^64 segundos.
    #[must_use]
    pub const fn to_wall_duration(self) -> Duration {
        // Segundos enteros más el resto de ticks. El resto es menor que `TICK_RATE_HZ`, así que
        // su parte en nanosegundos queda por debajo de un segundo y la suma nunca desborda. Así
        // se evita el producto `self.0 * nanos`, que sí desbordaría `u64` para valores grandes.
        let whole_seconds = Duration::from_secs(self.0 / TICK_RATE_HZ as u64);
        let remainder_ticks = self.0 % TICK_RATE_HZ as u64;
        let remainder = Duration::from_nanos(remainder_ticks * TICK_DURATION.subsec_nanos() as u64);
        match whole_seconds.checked_add(remainder) {
            Some(wall) => wall,
            // Inalcanzable: el resto es menor que un segundo y los segundos caben de sobra.
            None => panic!("el resto de ticks es menor que un segundo"),
        }
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Ticks {
    type Output = Ticks;

    fn add(self, rhs: Ticks) -> Ticks {
        Ticks(add_or_panic(self.0, rhs.0))
    }
}

impl AddAssign<Ticks> for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        *self = *self + rhs;
    }
}

/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Ticks {
    type Output = Ticks;

    fn sub(self, rhs: Ticks) -> Ticks {
        Ticks(sub_or_panic(self.0, rhs.0))
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Mul<u64> for Ticks {
    type Output = Ticks;

    fn mul(self, rhs: u64) -> Ticks {
        Ticks(mul_or_panic(self.0, rhs))
    }
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Mul<Ticks> for u64 {
    type Output = Ticks;

    fn mul(self, rhs: Ticks) -> Ticks {
        rhs * self
    }
}

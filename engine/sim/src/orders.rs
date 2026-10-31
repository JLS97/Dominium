//! La cola canónica de órdenes selladas por tick (reglas 6 y 15): `Sealed<O>`,
//! `QueueError` y `OrderQueue<O>`. El jugador y la IA entran por la misma
//! cola, y las pendientes se guardan y se reproducen. El tipo de orden lo
//! define `game_rules` (`World::Order`); aquí solo está el mecanismo.
// Contrato A: los cuerpos son marcadores pendientes, no usan sus parámetros
// y no leen los campos privados. `expect` obliga a P02-T05 a retirar estas
// líneas al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P02-T05")]
#![expect(
    clippy::needless_pass_by_value,
    reason = "`push` no consume `sealed` hasta P02-T05"
)]
#![expect(dead_code, reason = "campos privados sin leer hasta P02-T05")]

use engine_core::hash::{HashState, StateHasher};
use engine_core::time::Tick;
use serde::{Deserialize, Serialize};

use crate::buffer::FixedVec;

/// Una orden y el tick en que se aplica.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed<O> {
    /// El tick en que se aplica.
    pub tick: Tick,
    /// La orden.
    pub order: O,
}

/// El tick y después la orden.
impl<O: HashState> HashState for Sealed<O> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        todo!("P02-T05")
    }
}

/// Por qué una orden no entra en la cola.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueueError {
    /// No cabe: la cola tiene `capacity` órdenes.
    #[error("cola de órdenes llena: capacidad {capacity}")]
    Full {
        /// Capacidad de la cola.
        capacity: usize,
    },
    /// Sellada para un tick anterior al de la última orden encolada.
    #[error("orden sellada para el tick {tick:?}, anterior al último encolado {last:?}")]
    OutOfOrder {
        /// El tick de la orden rechazada.
        tick: Tick,
        /// El tick de la última orden encolada.
        last: Tick,
    },
    /// La simulación reproduce un replay y las órdenes salen de él.
    #[error("la simulación está reproduciendo un replay y no admite órdenes externas")]
    Replaying,
}

/// La cola canónica de órdenes selladas (regla 6): las pendientes se guardan (`SaveBody::pending`) y se reproducen.
/// Orden de aplicación: por tick sellado y, dentro del tick, por orden de entrada. Capacidad fija. No se serializa ella
/// misma: `SaveBody` lleva `pending()` y `load` la reconstruye con la capacidad de `SimConfig`.
pub struct OrderQueue<O> {
    /// Las órdenes selladas, en orden de entrada, que es el de tick no decreciente.
    items: FixedVec<Sealed<O>>,
}

/// Conserva la capacidad.
impl<O: Clone> Clone for OrderQueue<O> {
    fn clone(&self) -> OrderQueue<O> {
        todo!("P02-T05")
    }
}

impl<O> OrderQueue<O> {
    /// Vacía, con sitio para `capacity` órdenes. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> OrderQueue<O> {
        todo!("P02-T05")
    }

    /// Órdenes pendientes.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T05")
    }

    /// Cierto si no hay ninguna orden pendiente.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T05")
    }

    /// Capacidad congelada.
    #[must_use]
    pub fn capacity(&self) -> usize {
        todo!("P02-T05")
    }

    /// Comprueba primero el orden y después el sitio: `OutOfOrder` si `sealed.tick` es anterior al tick de la última orden
    /// encolada, y si no, `Full` si no cabe. No asigna.
    pub fn push(&mut self, sealed: Sealed<O>) -> Result<(), QueueError> {
        todo!("P02-T05")
    }

    /// Vacía `out` y mueve a él, en orden de entrada, las órdenes selladas exactamente para `tick`. Si queda alguna sellada
    /// para un tick anterior, pánico `orden del pasado`: es un invariante roto. `out` necesita capacidad; si no, pánico
    /// `columna llena`.
    ///
    /// # Panics
    ///
    /// Pánico `orden del pasado` si queda una orden sellada para un tick anterior a `tick`, y `columna llena` si
    /// las del tick no caben en `out`.
    pub fn drain_for(&mut self, tick: Tick, out: &mut FixedVec<O>) {
        todo!("P02-T05")
    }

    /// Las pendientes, en orden de aplicación.
    #[must_use]
    pub fn pending(&self) -> &[Sealed<O>] {
        todo!("P02-T05")
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        todo!("P02-T05")
    }
}

/// `pending()` como slice: la longitud y después cada `Sealed` en orden.
impl<O: HashState> HashState for OrderQueue<O> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        todo!("P02-T05")
    }
}

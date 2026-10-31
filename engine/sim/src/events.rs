//! Pipeline de eventos ordenados (regla 13): `EventKey` y `SortedEvents<E>`.
//! Los eventos de cada chunk se concatenan en orden de chunk y se ordenan por
//! (objetivo, origen, tipo) con la secuencia de emisión como desempate, sin
//! asignar, para que el resultado no dependa del número de hilos.
// Contrato A: los cuerpos son marcadores pendientes, no usan sus parámetros,
// no leen los campos privados y lo que sigue a cada `todo!` solo fija el
// tipo oculto del `impl Iterator`. `expect` obliga a P02-T04 a retirar
// estas líneas al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P02-T04")]
#![expect(dead_code, reason = "campos privados sin leer hasta P02-T04")]
#![expect(
    unreachable_code,
    reason = "tipo oculto de `impl Iterator` fijado tras `todo!` hasta P02-T04"
)]

use crate::buffer::FixedVec;
use crate::parallel::ChunkScratch;

/// Clave de orden de un evento: (objetivo, origen, tipo). Objetivo y origen son `Handle::to_raw()`, o 0 si no hay;
/// el tipo lo numera quien define los eventos. Los desempates de juego (dos matan al mismo en el mismo tick) salen de
/// este orden y se fijan en la spec de combate.
pub trait EventKey {
    /// La clave (objetivo, origen, tipo) por la que se ordena el evento.
    fn key(&self) -> (u64, u64, u32);
}

/// Los eventos de todos los chunks, concatenados en orden de chunk y ordenados por clave; los empates conservan el orden
/// de chunk y de emisión. Sin asignar: `sort_unstable` con la secuencia de emisión como último criterio, porque el `sort`
/// estable de la biblioteca estándar reserva un buffer temporal.
pub struct SortedEvents<E> {
    /// Cada evento con su secuencia de emisión, que desempata la clave.
    items: FixedVec<(E, u32)>,
}

/// Conserva la capacidad.
impl<E: Clone> Clone for SortedEvents<E> {
    fn clone(&self) -> SortedEvents<E> {
        todo!("P02-T04")
    }
}

impl<E: EventKey + Clone> SortedEvents<E> {
    /// Vacío, con sitio para `capacity` eventos. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> SortedEvents<E> {
        todo!("P02-T04")
    }

    /// Vacía, concatena los eventos de cada chunk en orden de chunk y ordena. Exige capacidad para todos; si no,
    /// pánico `eventos llenos`. No asigna.
    ///
    /// # Panics
    ///
    /// Pánico `eventos llenos` si los eventos de todos los chunks no caben en la capacidad.
    pub fn collect<S>(&mut self, scratch: &ChunkScratch<S>, events_of: impl Fn(&S) -> &[E]) {
        todo!("P02-T04")
    }

    /// Eventos del último `collect`.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T04")
    }

    /// Cierto si no hay ningún evento.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T04")
    }

    /// Los eventos en el orden final.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        todo!("P02-T04");
        core::iter::empty()
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        todo!("P02-T04")
    }
}

//! Pipeline de eventos ordenados (regla 13): `EventKey` y `SortedEvents<E>`.
//! Los eventos de cada chunk se concatenan en orden de chunk y se ordenan por
//! (objetivo, origen, tipo) con la secuencia de emisión como desempate, sin
//! asignar, para que el resultado no dependa del número de hilos.

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
        SortedEvents {
            items: self.items.clone(),
        }
    }
}

impl<E: EventKey + Clone> SortedEvents<E> {
    /// Vacío, con sitio para `capacity` eventos. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> SortedEvents<E> {
        SortedEvents {
            items: FixedVec::with_capacity(capacity),
        }
    }

    /// Vacía, concatena los eventos de cada chunk en orden de chunk y ordena. Exige capacidad para todos; si no,
    /// pánico `eventos llenos`. No asigna.
    ///
    /// # Panics
    ///
    /// Pánico `eventos llenos` si los eventos de todos los chunks no caben en la capacidad.
    pub fn collect<S>(&mut self, scratch: &ChunkScratch<S>, events_of: impl Fn(&S) -> &[E]) {
        self.items.clear();
        // Se comprueba el total antes de copiar nada, para no dejar un buffer a medias.
        let total: usize = scratch
            .iter()
            .map(|(_, chunk)| events_of(chunk).len())
            .sum();
        assert!(total <= self.items.capacity(), "eventos llenos");
        // La secuencia de emisión crece con el orden de chunk y, dentro del chunk, con el de emisión; es el desempate
        // que hace que `sort_unstable` dé el mismo resultado que un orden estable.
        let mut sequence: u32 = 0;
        for (_, chunk) in scratch.iter() {
            for event in events_of(chunk) {
                self.items.push((event.clone(), sequence));
                sequence += 1;
            }
        }
        self.items
            .as_mut_slice()
            .sort_unstable_by_key(|(event, sequence)| (event.key(), *sequence));
    }

    /// Eventos del último `collect`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Cierto si no hay ningún evento.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Los eventos en el orden final.
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.items.as_slice().iter().map(|(event, _)| event)
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

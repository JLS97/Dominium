//! `FixedVec<T>`: vector de capacidad fija que nunca crece. Es la única forma
//! de buffer en el camino caliente (regla 19): reserva una vez, en
//! `with_capacity` o `reserve`, y a partir de ahí agotar la capacidad es un
//! pánico con mensaje fijo o un `Err`, nunca una asignación.
// Contrato A: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P02-T02 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P02-T02")]
#![expect(
    clippy::needless_pass_by_value,
    reason = "`push` y `try_push` no consumen `value` hasta P02-T02"
)]

use core::ops::{Index, IndexMut};

use engine_core::hash::{HashState, StateHasher};
use serde::{Deserialize, Serialize};

/// Vector de capacidad fija. Reserva una vez y nunca crece: `push` con `len() == capacity()` hace pánico `columna llena`.
/// Es la única forma de buffer en el camino caliente (regla 19). Se serializa como una secuencia, igual que un `Vec`;
/// al deserializar, la capacidad queda igual a la longitud hasta que alguien llame a `reserve`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FixedVec<T> {
    /// Los elementos, en orden. Solo este campo entra en `serde`: la secuencia.
    items: Vec<T>,
    /// Capacidad congelada en `with_capacity` o `reserve`. No se serializa: el derivado la deja a 0 y P02-T02
    /// la iguala a la longitud al deserializar, como fija la spec.
    #[serde(skip)]
    capacity: usize,
}

/// Conserva la capacidad: el clon reserva `capacity()`, no `len()`.
impl<T: Clone> Clone for FixedVec<T> {
    fn clone(&self) -> FixedVec<T> {
        todo!("P02-T02")
    }
}

impl<T> FixedVec<T> {
    /// Vacío, con sitio para `capacity` elementos. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> FixedVec<T> {
        todo!("P02-T02")
    }

    /// Amplía la capacidad hasta `capacity` si es menor. Asigna. Solo tras deserializar o al construir.
    pub fn reserve(&mut self, capacity: usize) {
        todo!("P02-T02")
    }

    /// Elementos guardados.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T02")
    }

    /// Cierto si no hay ningún elemento.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T02")
    }

    /// Capacidad congelada: el máximo de elementos sin asignar.
    #[must_use]
    pub fn capacity(&self) -> usize {
        todo!("P02-T02")
    }

    /// Los elementos como slice, en orden.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        todo!("P02-T02")
    }

    /// Los elementos como slice mutable, en orden.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        todo!("P02-T02")
    }

    /// Añade `value` al final.
    ///
    /// # Panics
    ///
    /// Pánico `columna llena` si `len() == capacity()`.
    pub fn push(&mut self, value: T) {
        todo!("P02-T02")
    }

    /// `Err(value)` si estaba lleno, sin pánico, con el valor intacto, para quien prefiera decidir.
    pub fn try_push(&mut self, value: T) -> Result<(), T> {
        todo!("P02-T02")
    }

    /// Saca el último elemento, o `None` si está vacío.
    pub fn pop(&mut self) -> Option<T> {
        todo!("P02-T02")
    }

    /// Intercambia los elementos `a` y `b`.
    ///
    /// # Panics
    ///
    /// Pánico `índice fuera del buffer` si alguno es `>= len()`.
    pub fn swap(&mut self, a: usize, b: usize) {
        todo!("P02-T02")
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        todo!("P02-T02")
    }

    /// Recorta a `len` elementos si tiene más; si no, no hace nada. No toca la capacidad.
    pub fn truncate(&mut self, len: usize) {
        todo!("P02-T02")
    }
}

impl<T> Index<usize> for FixedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        todo!("P02-T02")
    }
}

impl<T> IndexMut<usize> for FixedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        todo!("P02-T02")
    }
}

/// Como `[T]`: la longitud como `u64` y después los elementos, con la vía en bloque de `T`.
impl<T: HashState> HashState for FixedVec<T> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        todo!("P02-T02")
    }
}

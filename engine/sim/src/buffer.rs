//! `FixedVec<T>`: vector de capacidad fija que nunca crece. Es la única forma
//! de buffer en el camino caliente (regla 19): reserva una vez, en
//! `with_capacity` o `reserve`, y a partir de ahí agotar la capacidad es un
//! pánico con mensaje fijo o un `Err`, nunca una asignación.

use core::ops::{Index, IndexMut};

use engine_core::hash::{HashState, StateHasher};
use serde::{Deserialize, Deserializer, Serialize};

/// Vector de capacidad fija. Reserva una vez y nunca crece: `push` con `len() >= capacity()` hace pánico `columna llena`.
/// Es la única forma de buffer en el camino caliente (regla 19). Se serializa como una secuencia, igual que un `Vec`
/// (`#[serde(transparent)]` sobre los elementos); al deserializar, la capacidad queda igual a la longitud hasta que
/// alguien llame a `reserve`. La igualdad compara solo los elementos: la capacidad no es canónica y no entra en `==`,
/// en `serde` ni en `HashState` (fijado en P02-T01).
///
/// Invariante interno: `items.capacity() >= capacity`, así que ningún `push` por debajo de `capacity` reasigna.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct FixedVec<T> {
    /// Los elementos, en orden. Solo este campo entra en `serde`: la secuencia.
    items: Vec<T>,
    /// Capacidad congelada en `with_capacity` o `reserve`. No es canónica: no entra en `serde`, en `==` ni en
    /// `HashState`.
    #[serde(skip)]
    capacity: usize,
}

/// Conserva la capacidad: el clon reserva `capacity()`, no `len()`.
impl<T: Clone> Clone for FixedVec<T> {
    fn clone(&self) -> FixedVec<T> {
        let mut items = Vec::with_capacity(self.capacity);
        items.extend_from_slice(&self.items);
        FixedVec {
            items,
            capacity: self.capacity,
        }
    }
}

/// Implementación propia: compara solo los elementos, sin la capacidad.
impl<T: PartialEq> PartialEq for FixedVec<T> {
    fn eq(&self, other: &FixedVec<T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for FixedVec<T> {}

/// Implementación propia: deserializa la secuencia (la misma que escribe `Serialize`) y deja `capacity() == len()`,
/// nunca 0 (regla 19: un buffer deserializado no puede crecer en silencio al primer `push`).
impl<'de, T: Deserialize<'de>> Deserialize<'de> for FixedVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<FixedVec<T>, D::Error> {
        let items = Vec::<T>::deserialize(deserializer)?;
        let capacity = items.len();
        Ok(FixedVec { items, capacity })
    }
}

impl<T> FixedVec<T> {
    /// Vacío, con sitio para `capacity` elementos. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> FixedVec<T> {
        FixedVec {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Amplía la capacidad hasta `capacity` si es menor. Asigna. Solo tras deserializar o al construir.
    pub fn reserve(&mut self, capacity: usize) {
        if capacity > self.capacity {
            // `len() <= self.capacity < capacity`, así que la resta no desborda.
            self.items.reserve_exact(capacity - self.items.len());
            self.capacity = capacity;
        }
    }

    /// Elementos guardados.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Cierto si no hay ningún elemento.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Capacidad congelada: el máximo de elementos sin asignar.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Los elementos como slice, en orden.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.items.as_slice()
    }

    /// Los elementos como slice mutable, en orden.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.items.as_mut_slice()
    }

    /// Añade `value` al final.
    ///
    /// # Panics
    ///
    /// Pánico `columna llena` si `len() == capacity()`.
    pub fn push(&mut self, value: T) {
        assert!(self.items.len() < self.capacity, "columna llena");
        self.items.push(value);
    }

    /// `Err(value)` si estaba lleno, sin pánico, con el valor intacto, para quien prefiera decidir.
    pub fn try_push(&mut self, value: T) -> Result<(), T> {
        if self.items.len() < self.capacity {
            self.items.push(value);
            Ok(())
        } else {
            Err(value)
        }
    }

    /// Saca el último elemento, o `None` si está vacío.
    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    /// Intercambia los elementos `a` y `b`.
    ///
    /// # Panics
    ///
    /// Pánico `índice fuera del buffer` si alguno es `>= len()`.
    pub fn swap(&mut self, a: usize, b: usize) {
        let len = self.items.len();
        assert!(a < len && b < len, "índice fuera del buffer");
        self.items.swap(a, b);
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Recorta a `len` elementos si tiene más; si no, no hace nada. No toca la capacidad.
    pub fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
    }
}

impl<T> Index<usize> for FixedVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.items[index]
    }
}

impl<T> IndexMut<usize> for FixedVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.items[index]
    }
}

/// Como `[T]`: la longitud como `u64` y después los elementos, con la vía en bloque de `T`.
impl<T: HashState> HashState for FixedVec<T> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        // Delegar en `[T]` garantiza byte a byte la misma secuencia que `hash_of(buf.as_slice())`.
        self.as_slice().hash_state(hasher);
    }
}

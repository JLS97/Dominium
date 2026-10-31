//! Tablas SoA con el handle común (regla 5, ADR-0004): `Row`, `Column<T>`, el
//! trait `Columns` y `Table<C>`. Una columna por campo, filas densas con las
//! activas delante, directorio por slot con generación, borrado diferido por
//! fila descendente y reordenación por permutación. Ninguna operación asigna
//! después de `with_capacity`.
// Contrato A: los cuerpos son marcadores pendientes, no usan sus parámetros
// y no leen los campos privados. `expect` obliga a P02-T02 y P02-T03 a
// retirar estas líneas al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P02-T02 y P02-T03")]
#![expect(
    clippy::needless_pass_by_value,
    reason = "`push` e `insert` no consumen su fila hasta P02-T02"
)]
#![expect(dead_code, reason = "campos privados sin leer hasta P02-T02")]

use core::num::NonZeroU32;
use core::ops::{Index, IndexMut};

use engine_core::handle::Handle;
use engine_core::hash::{HashState, StateHasher};
use serde::{Deserialize, Deserializer, Serialize};

use crate::buffer::FixedVec;

/// Fila densa de una tabla. Cambia con borrados, activaciones y reordenaciones; nunca se guarda de un tick a otro.
/// Lo estable es el `Handle`; `Handle::slot()` indexa el directorio de la tabla, no la fila.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Row(pub u32);

impl Row {
    /// La fila como índice de slice.
    #[must_use]
    pub const fn index(self) -> usize {
        panic!("P02-T02: pendiente de implementar")
    }
}

/// Una columna SoA: un `FixedVec<T>` indexado por `Row`. Se serializa igual que el `FixedVec` (`#[serde(transparent)]`).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Column<T> {
    /// Los valores, uno por fila.
    items: FixedVec<T>,
}

/// Conserva la capacidad.
impl<T: Clone> Clone for Column<T> {
    fn clone(&self) -> Column<T> {
        todo!("P02-T02")
    }
}

impl<T> Column<T> {
    /// Vacía, con sitio para `capacity` filas. Asigna.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Column<T> {
        todo!("P02-T02")
    }

    /// Amplía la capacidad hasta `capacity` si es menor. Asigna. Solo tras deserializar o al construir.
    pub fn reserve(&mut self, capacity: usize) {
        todo!("P02-T02")
    }

    /// Filas guardadas.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T02")
    }

    /// Cierto si no hay ninguna fila.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T02")
    }

    /// Capacidad congelada.
    #[must_use]
    pub fn capacity(&self) -> usize {
        todo!("P02-T02")
    }

    /// Los valores como slice, en orden de fila.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        todo!("P02-T02")
    }

    /// Los valores como slice mutable, en orden de fila.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        todo!("P02-T02")
    }

    /// Añade una fila al final.
    ///
    /// # Panics
    ///
    /// Pánico `columna llena` si `len() == capacity()`.
    pub fn push(&mut self, value: T) {
        todo!("P02-T02")
    }

    /// Intercambia los valores de dos filas.
    ///
    /// # Panics
    ///
    /// Pánico `fila fuera de la tabla` si alguna es `>= len()`.
    pub fn swap(&mut self, a: Row, b: Row) {
        todo!("P02-T02")
    }

    /// Saca la última fila, o `None` si está vacía.
    pub fn pop(&mut self) -> Option<T> {
        todo!("P02-T02")
    }

    /// Vacía sin tocar la capacidad.
    pub fn clear(&mut self) {
        todo!("P02-T02")
    }
}

impl<T> Index<Row> for Column<T> {
    type Output = T;

    fn index(&self, row: Row) -> &T {
        todo!("P02-T02")
    }
}

impl<T> IndexMut<Row> for Column<T> {
    fn index_mut(&mut self, row: Row) -> &mut T {
        todo!("P02-T02")
    }
}

/// Como `[T]`: longitud y elementos, con la vía en bloque de `T`.
impl<T: HashState> HashState for Column<T> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        todo!("P02-T02")
    }
}

/// Las columnas de una tabla concreta. Lo implementa `game_rules` a mano, una línea por columna y método
/// (ADR-0004: sin macro antes del hito 3). Toda operación estructural pasa por aquí, así ninguna columna se desalinea.
/// `len` devuelve la longitud de las columnas, que deben coincidir entre sí: la tabla solo compara ese valor con el suyo,
/// así que una desalineación entre dos columnas la detecta `len` si `game_rules` las compara (recomendado,
/// con `debug_assert_eq!`), y la tabla no.
pub trait Columns: Sized {
    /// Los valores de una entidad al insertarla. Es el único struct por entidad que existe.
    type Row;

    /// Todas las columnas vacías, cada una con sitio para `capacity` filas. Asigna.
    fn with_capacity(capacity: usize) -> Self;

    /// Amplía la capacidad de todas las columnas hasta `capacity` si es menor. Asigna.
    fn reserve(&mut self, capacity: usize);

    /// La longitud de las columnas, que deben coincidir entre sí.
    fn len(&self) -> usize;

    /// Cierto si no hay ninguna fila.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Descompone `row` y añade cada valor al final de su columna.
    fn push(&mut self, row: Self::Row);

    /// Intercambia dos filas en todas las columnas.
    fn swap(&mut self, a: Row, b: Row);

    /// Quita la última fila de todas las columnas.
    fn pop(&mut self);

    /// Vacía todas las columnas sin tocar su capacidad.
    fn clear(&mut self);
}

/// Tabla densa con directorio: filas `0..len()`, las activas delante (`0..active_len()`), las inactivas detrás.
/// Garantiza: `handle_at(row_of(h)) == h` para todo handle vivo; un handle muerto no vuelve a estar vivo (la generación
/// del directorio sube al liberar el slot, `Handle::next_generation`), salvo tras 2^32 - 1 reutilizaciones del mismo slot;
/// los borrados se aplican solo en `apply_removals`, por fila descendente; ninguna operación asigna tras
/// `with_capacity`, e `insert` falla con `TableFull` en vez de crecer. Tras cada operación estructural comprueba
/// `C::len() == len()` y, si no cuadra porque alguien tocó las columnas por `columns_mut`, hace pánico `columnas desalineadas`.
///
/// La codificación `serde` es la de "Datos y formatos" de la spec: los campos en este orden, sin el scratch de
/// permutación ni la bandera de pendiente por slot.
#[derive(Serialize)]
pub struct Table<C: Columns> {
    /// Slots del directorio. Menor que `u32::MAX`.
    capacity: u32,
    /// Fila por slot, `u32::MAX` si el slot está libre. `capacity` entradas.
    dir_rows: FixedVec<u32>,
    /// Generación por slot. `capacity` entradas, nunca 0.
    dir_generations: FixedVec<NonZeroU32>,
    /// Pila de slots libres, del fondo a la cima: el último entra el primero.
    free: FixedVec<u32>,
    /// La columna densa fila → handle.
    handles: FixedVec<Handle>,
    /// Frontera del conjunto activo: filas `0..active_len` activas.
    active_len: u32,
    /// Borrados pendientes de `apply_removals`, en orden de marcado.
    pending: FixedVec<Handle>,
    /// Las columnas de `game_rules`.
    columns: C,
    /// Bandera de pendiente por slot, para que `mark_removed` sea de tiempo constante. Derivada: no se serializa.
    #[serde(skip)]
    pending_flags: FixedVec<bool>,
    /// Scratch de `permute_active`. Derivado: no se serializa.
    #[serde(skip)]
    permutation_scratch: FixedVec<Row>,
}

/// Conserva la capacidad.
impl<C: Columns + Clone> Clone for Table<C> {
    fn clone(&self) -> Table<C> {
        todo!("P02-T02")
    }
}

/// Implementación propia: valida los invariantes y vuelve a reservar la capacidad (ver "Datos y formatos").
impl<'de, C: Columns + Deserialize<'de>> Deserialize<'de> for Table<C> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Table<C>, D::Error> {
        todo!("P02-T03")
    }
}

/// `insert` sobre una tabla sin slots libres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("tabla llena: capacidad {capacity}")]
pub struct TableFull {
    /// Capacidad de la tabla que no tenía sitio.
    pub capacity: usize,
}

impl<C: Columns> Table<C> {
    /// Exige `capacity < u32::MAX`; si no, pánico `capacidad de tabla excesiva`. Asigna una vez: directorio de `capacity`
    /// slots, pila de libres con todos los slots (el 0 sale primero), handles, pendientes, bandera de pendiente por slot
    /// y scratch de permutación, y `C::with_capacity`.
    ///
    /// # Panics
    ///
    /// Pánico `capacidad de tabla excesiva` si `capacity >= u32::MAX`.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Table<C> {
        todo!("P02-T02")
    }

    /// Slots del directorio: el máximo de entidades vivas.
    #[must_use]
    pub fn capacity(&self) -> usize {
        todo!("P02-T02")
    }

    /// Entidades vivas: filas `0..len()`.
    #[must_use]
    pub fn len(&self) -> usize {
        todo!("P02-T02")
    }

    /// Cierto si no hay ninguna entidad viva.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        todo!("P02-T02")
    }

    /// Entidades activas: filas `0..active_len()`.
    #[must_use]
    pub fn active_len(&self) -> usize {
        todo!("P02-T02")
    }

    /// Entra activa, en la fila `active_len()` (la inactiva que estaba ahí pasa al final). El slot del directorio sale de la
    /// pila de libres: el último liberado primero y, al principio, 0, 1, 2… La generación es la que tenga ese slot.
    pub fn insert(&mut self, row: C::Row) -> Result<Handle, TableFull> {
        todo!("P02-T02")
    }

    /// Cierto si el slot de `handle` está ocupado con su misma generación.
    #[must_use]
    pub fn is_alive(&self, handle: Handle) -> bool {
        todo!("P02-T02")
    }

    /// La fila de `handle`, o `None` si está muerto.
    #[must_use]
    pub fn row_of(&self, handle: Handle) -> Option<Row> {
        todo!("P02-T02")
    }

    /// El handle de la fila `row`.
    ///
    /// # Panics
    ///
    /// Pánico `fila fuera de la tabla` si `row.index() >= len()`.
    #[must_use]
    pub fn handle_at(&self, row: Row) -> Handle {
        todo!("P02-T02")
    }

    /// La columna densa fila → handle, en orden de fila.
    #[must_use]
    pub fn handles(&self) -> &[Handle] {
        todo!("P02-T02")
    }

    /// Cierto si `handle` está vivo y su fila es `< active_len()`.
    #[must_use]
    pub fn is_active(&self, handle: Handle) -> bool {
        todo!("P02-T02")
    }

    /// Mueve la fila a la frontera del conjunto activo por intercambio con la fila `active_len() - 1` (al desactivar) o
    /// `active_len()` (al activar). Devuelve `false` sin cambiar nada si el handle está muerto o ya estaba así.
    pub fn set_active(&mut self, handle: Handle, active: bool) -> bool {
        todo!("P02-T03")
    }

    /// Apunta el borrado para `apply_removals`, en tiempo constante (bandera por slot). `false` si el handle está muerto
    /// o ya estaba apuntado. No asigna.
    pub fn mark_removed(&mut self, handle: Handle) -> bool {
        todo!("P02-T02")
    }

    /// Los handles apuntados para borrar, en orden de marcado.
    #[must_use]
    pub fn pending_removals(&self) -> &[Handle] {
        todo!("P02-T02")
    }

    /// Aplica los borrados pendientes así: toma la fila de cada pendiente al empezar, y las recorre de mayor a menor (el
    /// "slot descendente" de ADR-0004, que aquí es la fila densa); para cada fila `r`, si `r < active_len()` intercambia `r`
    /// con `active_len() - 1` y baja `active_len`, y después intercambia la fila en la que ha quedado con `len() - 1` y hace
    /// `pop`. Cada slot liberado sube de generación y entra en la pila de libres, así que el último borrado es el primero
    /// que se reutiliza. Vacía `removed` y deja en él los handles borrados en ese orden; `removed` tiene que tener
    /// capacidad para `pending_removals().len()`, si no, pánico `columna llena`. Después, `pending_removals()` está vacío.
    ///
    /// # Panics
    ///
    /// Pánico `columna llena` si `removed` no tiene capacidad para `pending_removals().len()`.
    pub fn apply_removals(&mut self, removed: &mut FixedVec<Handle>) {
        todo!("P02-T02")
    }

    /// Reordena las filas activas: la fila `i` pasa a tener lo que tenía `order[i]`. Exige que `order` sea una permutación
    /// de `0..active_len()`; si no, pánico `permutación inválida`. Mantiene el directorio. No asigna.
    ///
    /// # Panics
    ///
    /// Pánico `permutación inválida` si `order` no es una permutación de `0..active_len()`.
    pub fn permute_active(&mut self, order: &[Row]) {
        todo!("P02-T03")
    }

    /// Las columnas, para leer.
    #[must_use]
    pub fn columns(&self) -> &C {
        todo!("P02-T02")
    }

    /// Solo para leer y escribir valores. Las operaciones estructurales de `Columns` las hace la tabla; hacerlas aquí
    /// acaba en pánico `columnas desalineadas` en la siguiente operación.
    #[must_use]
    pub fn columns_mut(&mut self) -> &mut C {
        todo!("P02-T02")
    }

    /// Borra todas las entidades, por fila descendente: las generaciones de sus slots suben y los slots vuelven a la pila
    /// en ese orden, así que el slot de la fila 0 es el primero que se reutiliza. Nada vuelve a estar vivo.
    pub fn clear(&mut self) {
        todo!("P02-T02")
    }
}

/// La secuencia de bytes de "Datos y formatos": `len` y `active_len` como `u64`; `handles()` como `[Handle]`; el
/// directorio como dos slices de `capacity` entradas, fila por slot (`u32`, `u32::MAX` si libre) y generación por slot
/// (`u32`); la pila de slots libres (`[u32]`, del fondo a la cima); los borrados pendientes (`[Handle]`); y después
/// `C::hash_state`. El scratch de permutación y la bandera de pendiente por slot no entran.
impl<C: Columns + HashState> HashState for Table<C> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        todo!("P02-T03")
    }
}

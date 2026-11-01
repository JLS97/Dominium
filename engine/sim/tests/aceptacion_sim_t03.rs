//! Tests de aceptación de P02-T03: `set_active`, `permute_active`, `HashState` y
//! `serde` de `table`.
//!
//! Cubren CA-3, CA-4, CA-5 y CA-6 de `specs/architecture/engine_sim.md`, más dos
//! huecos que señaló la verificación de P02-T02 (el orden de `apply_removals` y el de
//! la pila tras `clear`). Solo usan la API pública de `engine_sim` y `engine_core` y no
//! los modifica el implementador. Las columnas de prueba (`TestColumns`, dos columnas;
//! `ByteColumns`, una sola `Column<u8>` para el vector de 20 bytes) se definen aquí a
//! mano, una línea por columna y método, como hará `game_rules` (ADR-0004).
//!
//! Nada sale de la aleatoriedad del sistema: `proptest` corre con semilla fija y sin
//! persistencia de fallos, y las permutaciones salen de un xorshift sembrado por caso.
//! El modelo de referencia de CA-3 (`Model`) sigue las reglas de la documentación de
//! `Table`: fila de entrada de una inserción con inactivas, intercambios de
//! `set_active`, borrado por fila descendente y orden de la pila de slots libres.

use core::any::Any;
use core::num::NonZeroU32;
use std::panic::{AssertUnwindSafe, catch_unwind};

use engine_core::handle::Handle;
use engine_core::hash::{HashState, StateHash, StateHasher, bytemuck, hash_of};
use engine_sim::buffer::FixedVec;
use engine_sim::table::{Column, Columns, Row, Table, TableFull};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestError, TestRunner};
use serde::{Deserialize, Serialize};

/// Operaciones por caso de CA-3.
const OPS_PER_CASE: usize = 2_000;
/// Capacidad de la tabla grande de CA-3.
const PROPERTY_CAPACITY: u32 = 256;
/// Casos de `proptest` en CA-3. Cada caso son 2.000 operaciones sobre dos tablas (capacidad
/// 256 y capacidad 0), con las invariantes comprobadas tras cada operación.
const PROPERTY_CASES: u32 = 48;
/// Tope de iteraciones de reducción en CA-3: la secuencia tiene longitud fija, así que
/// `proptest` solo puede simplificar operación a operación y el caso mínimo aporta poco.
const PROPERTY_SHRINK_ITERS: u32 = 64;
/// Semilla fija de `proptest`. Cualquier valor vale; lo importante es que no sea aleatoria.
const PROPERTY_SEED: u64 = 0x5EED_0000_0203_0003;
/// Filas de la permutación sin asignaciones de CA-5.
const BIG_PERMUTATION_ROWS: usize = 4_096;
/// Entidades de la tabla grande de CA-6.
const BIG_TABLE_ROWS: u32 = 1_000;
/// Capacidad de la tabla grande de CA-6: deja 24 huecos para los `insert` tras cargar.
const BIG_TABLE_CAPACITY: u32 = 1_024;
/// Capacidad de la tabla "igual" de CA-6 que tiene que dar otro hash.
const OTHER_CAPACITY: u32 = 2_000;
/// Los 20 bytes fijados en "Datos y formatos" para la tabla mínima.
const FIXED_TABLE_BYTES: [u8; 20] = [
    0x02, 0x02, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x01,
    0x01, 0x00, 0x01, 0x07,
];

// ---------------------------------------------------------------------------------------
// Columnas de prueba
// ---------------------------------------------------------------------------------------

/// Valores de una entidad de prueba al insertarla: el único struct por entidad.
#[derive(Clone, Copy, Debug)]
struct TestRow {
    id: u32,
    value: u64,
}

/// Columnas de prueba: una `Column` por campo de `TestRow`. Sin flotantes, para comparar con
/// igualdad exacta. Deriva lo que necesita `Table`: `Clone` (conserva la capacidad),
/// `Serialize` y `Deserialize` (cada columna como su secuencia) y `PartialEq`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct TestColumns {
    ids: Column<u32>,
    values: Column<u64>,
}

impl Columns for TestColumns {
    type Row = TestRow;

    fn with_capacity(capacity: usize) -> TestColumns {
        TestColumns {
            ids: Column::with_capacity(capacity),
            values: Column::with_capacity(capacity),
        }
    }

    fn reserve(&mut self, capacity: usize) {
        self.ids.reserve(capacity);
        self.values.reserve(capacity);
    }

    fn len(&self) -> usize {
        debug_assert_eq!(
            self.ids.len(),
            self.values.len(),
            "columnas de prueba desalineadas"
        );
        self.ids.len()
    }

    fn push(&mut self, row: TestRow) {
        self.ids.push(row.id);
        self.values.push(row.value);
    }

    fn swap(&mut self, a: Row, b: Row) {
        self.ids.swap(a, b);
        self.values.swap(a, b);
    }

    fn pop(&mut self) {
        self.ids.pop();
        self.values.pop();
    }

    fn clear(&mut self) {
        self.ids.clear();
        self.values.clear();
    }
}

/// Cada columna en orden, como `[T]`.
impl HashState for TestColumns {
    fn hash_state(&self, hasher: &mut StateHasher) {
        self.ids.hash_state(hasher);
        self.values.hash_state(hasher);
    }
}

/// Columnas de un solo campo `Column<u8>`: la struct que fija el vector de 20 bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct ByteColumns {
    bytes: Column<u8>,
}

impl Columns for ByteColumns {
    type Row = u8;

    fn with_capacity(capacity: usize) -> ByteColumns {
        ByteColumns {
            bytes: Column::with_capacity(capacity),
        }
    }

    fn reserve(&mut self, capacity: usize) {
        self.bytes.reserve(capacity);
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn push(&mut self, row: u8) {
        self.bytes.push(row);
    }

    fn swap(&mut self, a: Row, b: Row) {
        self.bytes.swap(a, b);
    }

    fn pop(&mut self) {
        self.bytes.pop();
    }

    fn clear(&mut self) {
        self.bytes.clear();
    }
}

impl HashState for ByteColumns {
    fn hash_state(&self, hasher: &mut StateHasher) {
        self.bytes.hash_state(hasher);
    }
}

// ---------------------------------------------------------------------------------------
// Ayudas
// ---------------------------------------------------------------------------------------

/// Generación no nula a partir de un entero; los tests solo la piden con valores válidos.
fn nz(generation: u32) -> NonZeroU32 {
    NonZeroU32::new(generation).expect("la generación de un handle nunca es 0")
}

/// Handle a partir de dos enteros, para abreviar los tests.
fn handle(slot: u32, generation: u32) -> Handle {
    Handle::new(slot, nz(generation))
}

/// Fila de prueba con un identificador y un valor.
fn row(id: u32, value: u64) -> TestRow {
    TestRow { id, value }
}

/// `usize` como `u32`; los tests solo lo piden con valores que caben.
fn u32_of(value: usize) -> u32 {
    u32::try_from(value).expect("el valor cabe en u32")
}

/// `u32` como `usize`; en los objetivos de 64 bits nunca falla.
fn usize_of(value: u32) -> usize {
    usize::try_from(value).expect("u32 cabe en usize")
}

/// Generador xorshift64 con semilla fija: permutaciones reproducibles sin tocar el sistema.
struct XorShift64(u64);

impl XorShift64 {
    /// Estado inicial; la semilla no puede ser 0 porque xorshift se quedaría en 0.
    fn new(seed: u64) -> XorShift64 {
        assert_ne!(seed, 0, "la semilla de xorshift no puede ser 0");
        XorShift64(seed)
    }

    /// Siguiente palabra de 64 bits.
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Un índice uniforme en `0..bound`; `bound` tiene que ser mayor que 0.
    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("el límite cabe en u64");
        usize::try_from(self.next_u64() % bound).expect("el índice cabe en usize")
    }
}

/// Una permutación de `0..len` barajada con Fisher-Yates a partir de `seed`.
fn shuffled_rows(len: usize, seed: u64) -> Vec<Row> {
    let mut order: Vec<Row> = (0..len).map(|i| Row(u32_of(i))).collect();
    let mut rng = XorShift64::new(seed);
    for i in (1..len).rev() {
        let j = rng.below(i + 1);
        order.swap(i, j);
    }
    order
}

/// Foto de todo lo observable de una tabla de prueba, para comparar dos tablas o comprobar
/// que una operación "no cambia nada".
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    capacity: usize,
    len: usize,
    active_len: usize,
    handles: Vec<Handle>,
    pending: Vec<Handle>,
    ids: Vec<u32>,
    values: Vec<u64>,
}

fn snapshot(table: &Table<TestColumns>) -> Snapshot {
    Snapshot {
        capacity: table.capacity(),
        len: table.len(),
        active_len: table.active_len(),
        handles: table.handles().to_vec(),
        pending: table.pending_removals().to_vec(),
        ids: table.columns().ids.as_slice().to_vec(),
        values: table.columns().values.as_slice().to_vec(),
    }
}

/// Comprueba que `handle` está vivo en la fila `row`, que `row_of` y `handle_at` son inversas
/// la una de la otra y que `is_active` refleja si la fila está delante de `active_len()`.
fn assert_alive_at<C: Columns>(table: &Table<C>, handle: Handle, row: Row, active: bool) {
    assert!(table.is_alive(handle), "{handle:?} debería estar vivo");
    assert_eq!(table.row_of(handle), Some(row), "fila de {handle:?}");
    assert_eq!(table.handle_at(row), handle, "handle de {row:?}");
    assert_eq!(table.handles()[row.index()], handle, "handles()[{row:?}]");
    assert_eq!(
        table.is_active(handle),
        active,
        "is_active({handle:?}) en {row:?} con active_len {}",
        table.active_len()
    );
    assert_eq!(
        row.index() < table.active_len(),
        active,
        "fila de {handle:?}"
    );
}

/// Comprueba que `handle` está muerto: ni vivo, ni activo, ni con fila.
fn assert_dead<C: Columns>(table: &Table<C>, handle: Handle) {
    assert!(!table.is_alive(handle), "{handle:?} debería estar muerto");
    assert!(
        !table.is_active(handle),
        "{handle:?} muerto no puede estar activo"
    );
    assert_eq!(
        table.row_of(handle),
        None,
        "{handle:?} muerto no tiene fila"
    );
}

/// Comprueba que `handle_at` y `row_of` son inversas para todas las filas de la tabla.
fn assert_directory_consistent<C: Columns>(table: &Table<C>) {
    for (index, handle) in table.handles().iter().enumerate() {
        let row = Row(u32_of(index));
        assert_eq!(table.row_of(*handle), Some(row), "row_of({handle:?})");
        assert_eq!(table.handle_at(row), *handle, "handle_at({row:?})");
        assert_eq!(
            table.is_active(*handle),
            index < table.active_len(),
            "is_active({handle:?})"
        );
    }
}

/// El mensaje de texto de un pánico capturado con `catch_unwind`.
fn panic_message(payload: &Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "pánico sin mensaje de texto".to_string())
}

/// Ejecuta `f` y exige que haga pánico con un mensaje que contenga `expected`.
fn assert_panics_with<F: FnOnce()>(label: &str, expected: &str, f: F) {
    let result = catch_unwind(AssertUnwindSafe(f));
    let payload = match result {
        Ok(()) => panic!("{label}: no hizo pánico; se esperaba `{expected}`"),
        Err(payload) => payload,
    };
    let message = panic_message(&payload);
    assert!(
        message.contains(expected),
        "{label}: pánico con `{message}`; se esperaba `{expected}`"
    );
}

/// La secuencia de bytes de `Table<C>` en el hash de estado, escrita a mano según el párrafo
/// final de la sección `table` de la spec: `len` y `active_len` como `u64`; `handles()` como
/// `[Handle]` (longitud `u64` y `write_handles`); el directorio como dos slices `[u32]` de
/// `capacity` entradas (fila por slot, `u32::MAX` si libre; generación por slot); la pila de
/// libres `[u32]` del fondo a la cima; los pendientes `[Handle]`; y después las columnas.
/// Cada slice entra como `[T]` de `engine_core`: longitud `u64` y elementos en bloque.
fn handwritten_table_hash(
    handles: &[Handle],
    active_len: usize,
    dir_rows: &[u32],
    dir_generations: &[u32],
    free: &[u32],
    pending: &[Handle],
    columns: impl FnOnce(&mut StateHasher),
) -> StateHash {
    assert_eq!(dir_rows.len(), dir_generations.len(), "directorio");
    let mut hasher = StateHasher::new();
    hasher.write_u64(u64::try_from(handles.len()).expect("len cabe en u64"));
    hasher.write_u64(u64::try_from(active_len).expect("active_len cabe en u64"));
    hasher.write_u64(u64::try_from(handles.len()).expect("len cabe en u64"));
    hasher.write_handles(handles);
    hasher.write_u64(u64::try_from(dir_rows.len()).expect("capacity cabe en u64"));
    hasher.write(bytemuck::cast_slice(dir_rows));
    hasher.write_u64(u64::try_from(dir_generations.len()).expect("capacity cabe en u64"));
    hasher.write(bytemuck::cast_slice(dir_generations));
    hasher.write_u64(u64::try_from(free.len()).expect("free cabe en u64"));
    hasher.write(bytemuck::cast_slice(free));
    hasher.write_u64(u64::try_from(pending.len()).expect("pending cabe en u64"));
    hasher.write_handles(pending);
    columns(&mut hasher);
    hasher.finish()
}

/// Las columnas de prueba como entran en el hash: `ids` y después `values`, cada una como `[T]`.
fn hash_test_columns<'a>(ids: &'a [u32], values: &'a [u64]) -> impl FnOnce(&mut StateHasher) + 'a {
    move |hasher| {
        hasher.write_u64(u64::try_from(ids.len()).expect("len cabe en u64"));
        hasher.write(bytemuck::cast_slice(ids));
        hasher.write_u64(u64::try_from(values.len()).expect("len cabe en u64"));
        hasher.write(bytemuck::cast_slice(values));
    }
}

// ---------------------------------------------------------------------------------------
// Modelo de referencia de CA-3
// ---------------------------------------------------------------------------------------

/// Entidad viva del modelo: su handle y los valores de sus columnas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entity {
    handle: Handle,
    id: u32,
    value: u64,
}

/// Modelo de referencia de `Table`: un `Vec` de entidades vivas en orden de fila, con las
/// reglas de la documentación de `Table` (fila de entrada de una inserción, intercambios de
/// `set_active`, borrado por fila descendente, generaciones y orden de la pila de libres).
struct Model {
    capacity: u32,
    /// Las filas densas, en orden.
    rows: Vec<Entity>,
    active_len: usize,
    /// Generación por slot.
    generations: Vec<u32>,
    /// Pila de slots libres, del fondo a la cima.
    free: Vec<u32>,
    /// Borrados pendientes en orden de marcado.
    pending: Vec<Handle>,
    /// Handles que han muerto, para comprobarlos y para elegirlos como objetivo.
    dead: Vec<Handle>,
}

impl Model {
    fn new(capacity: u32) -> Model {
        Model {
            capacity,
            rows: Vec::new(),
            active_len: 0,
            generations: vec![1; usize_of(capacity)],
            // La cima es el final: el slot 0 sale primero.
            free: (0..capacity).rev().collect(),
            pending: Vec::new(),
            dead: Vec::new(),
        }
    }

    fn row_of(&self, handle: Handle) -> Option<usize> {
        self.rows.iter().position(|entity| entity.handle == handle)
    }

    /// Elige un handle objetivo a partir de un entero del generador: casi siempre uno vivo,
    /// a veces uno muerto, a veces uno arbitrario (con slot posiblemente fuera del directorio).
    fn pick(&self, selector: u32) -> Handle {
        let kind = selector % 8;
        let index = usize_of(selector / 8);
        if kind == 7 && !self.dead.is_empty() {
            self.dead[index % self.dead.len()]
        } else if kind == 6 {
            let slot = u32_of(index % (usize_of(self.capacity) + 4));
            handle(slot, u32_of(1 + index % 3))
        } else if !self.rows.is_empty() {
            self.rows[index % self.rows.len()].handle
        } else {
            handle(u32_of(index % 300), 1)
        }
    }

    fn insert(&mut self, id: u32, value: u64) -> Result<Handle, TableFull> {
        let Some(slot) = self.free.pop() else {
            return Err(TableFull {
                capacity: usize_of(self.capacity),
            });
        };
        let handle = handle(slot, self.generations[usize_of(slot)]);
        self.rows.push(Entity { handle, id, value });
        let last = self.rows.len() - 1;
        if self.active_len != last {
            self.rows.swap(self.active_len, last);
        }
        self.active_len += 1;
        Ok(handle)
    }

    fn mark_removed(&mut self, handle: Handle) -> bool {
        if self.row_of(handle).is_none() || self.pending.contains(&handle) {
            return false;
        }
        self.pending.push(handle);
        true
    }

    fn set_active(&mut self, handle: Handle, active: bool) -> bool {
        let Some(row) = self.row_of(handle) else {
            return false;
        };
        if active {
            if row < self.active_len {
                return false;
            }
            self.rows.swap(row, self.active_len);
            self.active_len += 1;
        } else {
            if row >= self.active_len {
                return false;
            }
            self.rows.swap(row, self.active_len - 1);
            self.active_len -= 1;
        }
        true
    }

    fn release(&mut self, handle: Handle) {
        let slot = handle.index();
        self.generations[slot] = Handle::next_generation(handle.generation()).get();
        self.free.push(handle.slot());
        self.dead.push(handle);
    }

    fn remove_row(&mut self, row: usize) -> Handle {
        let mut row = row;
        if row < self.active_len {
            self.rows.swap(row, self.active_len - 1);
            row = self.active_len - 1;
            self.active_len -= 1;
        }
        let last = self.rows.len() - 1;
        self.rows.swap(row, last);
        let entity = self.rows.pop().expect("la fila existe");
        self.release(entity.handle);
        entity.handle
    }

    fn apply_removals(&mut self) -> Vec<Handle> {
        let mut rows: Vec<usize> = self
            .pending
            .iter()
            .map(|handle| self.row_of(*handle).expect("los pendientes están vivos"))
            .collect();
        rows.sort_unstable_by(|a, b| b.cmp(a));
        let removed = rows.into_iter().map(|row| self.remove_row(row)).collect();
        self.pending.clear();
        removed
    }

    fn permute_active(&mut self, order: &[Row]) {
        assert_eq!(
            order.len(),
            self.active_len,
            "el modelo solo permuta activas"
        );
        let old: Vec<Entity> = self.rows[..self.active_len].to_vec();
        for (target, source) in self.rows[..self.active_len].iter_mut().zip(order) {
            *target = old[source.index()];
        }
    }

    fn clear(&mut self) {
        while let Some(entity) = self.rows.pop() {
            self.release(entity.handle);
        }
        self.pending.clear();
        self.active_len = 0;
    }

    /// El hash que debería dar la tabla, con la secuencia escrita a mano.
    fn expected_hash(&self) -> StateHash {
        let handles: Vec<Handle> = self.rows.iter().map(|entity| entity.handle).collect();
        let mut dir_rows = vec![u32::MAX; usize_of(self.capacity)];
        for (index, entity) in self.rows.iter().enumerate() {
            dir_rows[entity.handle.index()] = u32_of(index);
        }
        let ids: Vec<u32> = self.rows.iter().map(|entity| entity.id).collect();
        let values: Vec<u64> = self.rows.iter().map(|entity| entity.value).collect();
        handwritten_table_hash(
            &handles,
            self.active_len,
            &dir_rows,
            &self.generations,
            &self.free,
            &self.pending,
            hash_test_columns(&ids, &values),
        )
    }
}

/// Una operación aleatoria de CA-3.
#[derive(Clone, Copy, Debug)]
enum Op {
    Insert {
        id: u32,
        value: u64,
    },
    MarkRemoved(u32),
    ApplyRemovals,
    SetActive {
        target: u32,
        active: bool,
    },
    /// Semilla del xorshift que baraja `0..active_len()`.
    PermuteActive(u64),
    Clear,
}

/// Estrategia de una operación. Los pesos favorecen las inserciones para que la tabla de
/// capacidad 256 llegue a llenarse en una parte de los casos, y hacen `clear` raro para que
/// el estado se acumule entre operaciones.
fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        120 => (any::<u32>(), any::<u64>()).prop_map(|(id, value)| Op::Insert { id, value }),
        36 => any::<u32>().prop_map(Op::MarkRemoved),
        15 => Just(Op::ApplyRemovals),
        60 => (any::<u32>(), any::<bool>())
            .prop_map(|(target, active)| Op::SetActive { target, active }),
        25 => (1..u64::MAX).prop_map(Op::PermuteActive),
        1 => Just(Op::Clear),
    ]
}

/// Comprueba todas las invariantes de CA-3 de la tabla contra el modelo.
fn assert_matches_model(table: &Table<TestColumns>, model: &Model, step: usize, op: Op) {
    let context = format!("paso {step} ({op:?})");
    assert_eq!(
        table.capacity(),
        usize_of(model.capacity),
        "{context}: capacity"
    );
    assert_eq!(table.len(), model.rows.len(), "{context}: len");
    assert_eq!(
        table.is_empty(),
        model.rows.is_empty(),
        "{context}: is_empty"
    );
    assert_eq!(
        table.active_len(),
        model.active_len,
        "{context}: active_len"
    );
    assert!(
        table.active_len() <= table.len(),
        "{context}: active_len > len"
    );
    // La longitud de cada columna es `len()`.
    assert_eq!(table.columns().len(), table.len(), "{context}: columnas");
    assert_eq!(table.columns().ids.len(), table.len(), "{context}: ids");
    assert_eq!(
        table.columns().values.len(),
        table.len(),
        "{context}: values"
    );
    // `handles()` es exactamente la lista del modelo (y, por tanto, las filas
    // `0..active_len()` son exactamente las activas del modelo, en su orden).
    let expected: Vec<Handle> = model.rows.iter().map(|entity| entity.handle).collect();
    assert_eq!(table.handles(), &expected[..], "{context}: handles()");
    // `handles()` no repite handles.
    let mut sorted = table.handles().to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), table.len(), "{context}: handles repetidos");
    // Cada handle vivo resuelve a su fila, con sus valores.
    for (index, entity) in model.rows.iter().enumerate() {
        let row = Row(u32_of(index));
        assert_alive_at(table, entity.handle, row, index < model.active_len);
        assert_eq!(
            table.columns().ids[row],
            entity.id,
            "{context}: id de {:?}",
            entity.handle
        );
        assert_eq!(
            table.columns().values[row],
            entity.value,
            "{context}: valor de {:?}",
            entity.handle
        );
    }
    // Cada handle muerto da `row_of == None` e `is_alive == false`.
    for dead in &model.dead {
        assert_dead(table, *dead);
    }
    assert_eq!(
        table.pending_removals(),
        &model.pending[..],
        "{context}: pendientes"
    );
}

/// Aplica las 2.000 operaciones a una tabla de `capacity` y al modelo, comprobando las
/// invariantes tras cada una y el hash al final.
fn run_property_case(ops: &[Op], capacity: u32) {
    let mut table: Table<TestColumns> = Table::with_capacity(usize_of(capacity));
    let mut model = Model::new(capacity);
    let mut removed = FixedVec::<Handle>::with_capacity(usize_of(capacity));
    for (step, op) in ops.iter().copied().enumerate() {
        match op {
            Op::Insert { id, value } => {
                let expected = model.insert(id, value);
                let got = table.insert(row(id, value));
                assert_eq!(got, expected, "paso {step}: insert");
                if capacity == 0 {
                    assert_eq!(got, Err(TableFull { capacity: 0 }), "paso {step}");
                }
            }
            Op::MarkRemoved(target) => {
                let handle = model.pick(target);
                let expected = model.mark_removed(handle);
                assert_eq!(
                    table.mark_removed(handle),
                    expected,
                    "paso {step}: mark_removed({handle:?})"
                );
            }
            Op::ApplyRemovals => {
                let expected = model.apply_removals();
                table.apply_removals(&mut removed);
                assert_eq!(
                    removed.as_slice(),
                    &expected[..],
                    "paso {step}: apply_removals"
                );
            }
            Op::SetActive { target, active } => {
                let handle = model.pick(target);
                let expected = model.set_active(handle, active);
                assert_eq!(
                    table.set_active(handle, active),
                    expected,
                    "paso {step}: set_active({handle:?}, {active})"
                );
            }
            Op::PermuteActive(seed) => {
                let order = shuffled_rows(model.active_len, seed);
                model.permute_active(&order);
                table.permute_active(&order);
            }
            Op::Clear => {
                model.clear();
                table.clear();
            }
        }
        assert_matches_model(&table, &model, step, op);
    }
    assert_eq!(
        hash_of(&table),
        model.expected_hash(),
        "hash final de la tabla de capacidad {capacity}"
    );
}

/// Configuración de `proptest` con semilla fija y sin persistencia: ninguna aleatoriedad
/// del sistema y ningún archivo de regresiones.
fn property_config() -> Config {
    Config {
        cases: PROPERTY_CASES,
        max_shrink_iters: PROPERTY_SHRINK_ITERS,
        rng_seed: RngSeed::Fixed(PROPERTY_SEED),
        failure_persistence: None,
        ..Config::default()
    }
}

// CA-3: Propiedad con `proptest`: 2.000 operaciones aleatorias de `insert`, `mark_removed`,
// `apply_removals`, `set_active`, `permute_active` (con permutaciones válidas) y `clear`
// sobre una tabla de capacidad 256, y una variante con capacidad 0 en la que todo `insert`
// da `TableFull` y el resto no hace nada, comparadas con un modelo de referencia (un `Vec`
// de entidades vivas con su valor): tras cada operación, `len()` y `active_len()` coinciden
// con el modelo, cada handle vivo del modelo resuelve a una fila cuyo valor en las columnas
// es el suyo, cada handle muerto da `row_of == None` e `is_alive == false`, `handles()` no
// repite handles, las filas `0..active_len()` son exactamente las activas del modelo, y la
// longitud de cada columna es `len()`. Lo demuestra: test `ca_03_tabla_invariantes_propiedad`.
#[test]
fn ca_03_tabla_invariantes_propiedad() {
    let mut runner = TestRunner::new(property_config());
    let ops = proptest::collection::vec(op_strategy(), OPS_PER_CASE);
    let result = runner.run(&ops, |ops| {
        run_property_case(&ops, PROPERTY_CAPACITY);
        run_property_case(&ops, 0);
        Ok(())
    });
    match result {
        Ok(()) => {}
        Err(TestError::Fail(reason, ops)) => {
            let shown = ops.len().min(24);
            panic!(
                "CA-3 falla: {reason}\nprimeras {shown} de {} operaciones: {:?}",
                ops.len(),
                &ops[..shown]
            );
        }
        Err(TestError::Abort(reason)) => panic!("CA-3 abortado: {reason}"),
    }
}

// CA-3 (hueco señalado en la verificación de P02-T02): `apply_removals` ordena por fila
// descendente y no por orden inverso de marcado. Marcar `a` (fila 0), `c` (fila 2) y `b`
// (fila 1) da `removed == [c, b, a]`, y después la pila reutiliza los slots 0, 1 y 2 con
// generación 2.
#[test]
fn ca_03_apply_removals_ordena_por_fila_no_por_marcado() {
    let mut table: Table<TestColumns> = Table::with_capacity(8);
    let a = table.insert(row(10, 100)).expect("insert a");
    let b = table.insert(row(20, 200)).expect("insert b");
    let c = table.insert(row(30, 300)).expect("insert c");
    assert_eq!([a, b, c], [handle(0, 1), handle(1, 1), handle(2, 1)]);

    // Orden de marcado a, c, b; orden de borrado por fila descendente: c, b, a.
    assert!(table.mark_removed(a));
    assert!(table.mark_removed(c));
    assert!(table.mark_removed(b));
    assert_eq!(table.pending_removals(), &[a, c, b]);
    let mut removed = FixedVec::<Handle>::with_capacity(8);
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[c, b, a]);
    assert_eq!(table.len(), 0);
    assert_eq!(table.active_len(), 0);
    assert_eq!(table.pending_removals(), []);
    assert_dead(&table, a);
    assert_dead(&table, b);
    assert_dead(&table, c);

    // El último liberado (`a`, slot 0) es el primero que se reutiliza, y después 1 y 2.
    let a2 = table.insert(row(11, 101)).expect("insert tras borrar");
    let b2 = table.insert(row(21, 201)).expect("insert tras borrar");
    let c2 = table.insert(row(31, 301)).expect("insert tras borrar");
    assert_eq!([a2, b2, c2], [handle(0, 2), handle(1, 2), handle(2, 2)]);
    let d = table.insert(row(41, 401)).expect("insert de un slot nuevo");
    assert_eq!(d, handle(3, 1));
    assert_dead(&table, a);
    assert_dead(&table, b);
    assert_dead(&table, c);
    assert_eq!(table.handles(), &[a2, b2, c2, d]);
    assert_eq!(table.columns().ids.as_slice(), &[11, 21, 31, 41]);
}

// CA-3 (hueco señalado en la verificación de P02-T02): `clear` apila por fila descendente
// aunque los slots no estén en orden de fila. Con `a, b, c, d` insertadas y `a` borrada,
// `handles()` tiene los slots `[3, 1, 2]`; tras `clear`, los siguientes `insert` dan
// `(3,2), (1,2), (2,2), (0,2), (4,1)`.
#[test]
fn ca_03_clear_apila_por_fila_descendente() {
    let mut table: Table<TestColumns> = Table::with_capacity(5);
    let a = table.insert(row(10, 100)).expect("insert a");
    let b = table.insert(row(20, 200)).expect("insert b");
    let c = table.insert(row(30, 300)).expect("insert c");
    let d = table.insert(row(40, 400)).expect("insert d");
    assert!(table.mark_removed(a));
    let mut removed = FixedVec::<Handle>::with_capacity(5);
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[a]);
    // `a` estaba en la fila 0: `d` baja a su sitio y los slots quedan `[3, 1, 2]`.
    assert_eq!(table.handles(), &[d, b, c]);
    assert_eq!(
        table
            .handles()
            .iter()
            .map(|h| h.slot())
            .collect::<Vec<u32>>(),
        [3, 1, 2]
    );

    table.clear();
    assert_eq!(table.len(), 0);
    assert_eq!(table.active_len(), 0);
    assert_eq!(table.capacity(), 5);
    assert_dead(&table, b);
    assert_dead(&table, c);
    assert_dead(&table, d);

    // Por fila descendente: se liberan c (2), b (1) y d (3), así que el slot 3 (fila 0) sale
    // primero, después 1, 2, el 0 que ya estaba libre y por último el 4 nunca usado.
    let expected = [
        handle(3, 2),
        handle(1, 2),
        handle(2, 2),
        handle(0, 2),
        handle(4, 1),
    ];
    for (i, want) in expected.iter().enumerate() {
        let got = table.insert(row(u32_of(i), 0)).expect("caben cinco");
        assert_eq!(got, *want, "insert {i} tras clear");
    }
    assert_eq!(table.insert(row(9, 9)), Err(TableFull { capacity: 5 }));
    assert_dead(&table, a);
    assert_dead(&table, b);
    assert_dead(&table, c);
    assert_dead(&table, d);
}

// CA-4: Con cinco entidades activas: `set_active(c, false)` devuelve `true`,
// `active_len() == 4`, `c` queda en la fila 4 y la que estaba en la fila 4 pasa a la de `c`;
// `set_active(c, false)` otra vez devuelve `false`; `set_active(c, true)` devuelve `true` y
// `active_len() == 5`; `is_active` refleja cada paso; una entidad insertada con dos
// inactivas detrás entra en la fila `active_len()` y las inactivas siguen detrás;
// `set_active` de un handle muerto devuelve `false` sin cambiar `active_len()`;
// `set_active(x, false)` de una fila con borrado pendiente y después `apply_removals` la
// borra igual; y con `[a b c d | e f]` (`active_len` 4), `mark_removed(b)` y
// `apply_removals` dejan `handles() == [a, d, c, f, e]` y `active_len() == 3`. Lo
// demuestra: test `ca_04_conjunto_activo_como_prefijo`.
#[expect(
    clippy::too_many_lines,
    reason = "un test por criterio, como fija la spec"
)]
#[expect(
    clippy::many_single_char_names,
    reason = "los nombres a..f son los del criterio"
)]
#[test]
fn ca_04_conjunto_activo_como_prefijo() {
    let mut table: Table<TestColumns> = Table::with_capacity(8);
    let ents: Vec<Handle> = (0..5_u32)
        .map(|i| {
            table
                .insert(row((i + 1) * 10, u64::from(i + 1) * 100))
                .expect("caben cinco")
        })
        .collect();
    let [a, b, c, d, e] = ents[..] else {
        panic!("cinco handles")
    };
    assert_eq!(table.handles(), &[a, b, c, d, e]);
    assert_eq!(table.active_len(), 5);
    for h in [a, b, c, d, e] {
        assert!(table.is_active(h), "{h:?} empieza activo");
    }

    // Desactivar `c` (fila 2): intercambio con la fila `active_len() - 1 == 4`, donde estaba `e`.
    assert!(table.set_active(c, false));
    assert_eq!(table.active_len(), 4);
    assert_eq!(table.len(), 5);
    assert_eq!(table.handles(), &[a, b, e, d, c]);
    assert_alive_at(&table, c, Row(4), false);
    assert_alive_at(&table, e, Row(2), true);
    assert_eq!(table.columns().ids.as_slice(), &[10, 20, 50, 40, 30]);
    assert_eq!(
        table.columns().values.as_slice(),
        &[100, 200, 500, 400, 300]
    );
    assert!(!table.is_active(c));
    assert!(table.is_alive(c));
    assert_directory_consistent(&table);

    // Otra vez: ya estaba inactiva, `false` y nada cambia.
    let before = snapshot(&table);
    assert!(!table.set_active(c, false));
    assert_eq!(snapshot(&table), before);
    assert!(!table.is_active(c));

    // Reactivar: intercambio con la fila `active_len() == 4`, que es la suya.
    assert!(table.set_active(c, true));
    assert_eq!(table.active_len(), 5);
    assert!(table.is_active(c));
    assert_eq!(table.handles(), &[a, b, e, d, c]);
    assert_alive_at(&table, c, Row(4), true);
    // Ya activa: `false` y nada cambia.
    let before = snapshot(&table);
    assert!(!table.set_active(c, true));
    assert_eq!(snapshot(&table), before);
    assert_directory_consistent(&table);

    // Dos inactivas detrás (`d` y `e`) y una inserción: entra en la fila `active_len()` y las
    // inactivas siguen detrás.
    assert!(table.set_active(d, false)); // d (fila 3) <-> fila 4 (c)
    assert_eq!(table.handles(), &[a, b, e, c, d]);
    assert_eq!(table.active_len(), 4);
    assert!(table.set_active(e, false)); // e (fila 2) <-> fila 3 (c)
    assert_eq!(table.handles(), &[a, b, c, e, d]);
    assert_eq!(table.active_len(), 3);
    assert!(!table.is_active(d));
    assert!(!table.is_active(e));
    assert!(table.is_active(c));
    let f = table.insert(row(60, 600)).expect("insert f");
    assert_eq!(table.active_len(), 4);
    assert_eq!(table.len(), 6);
    assert_alive_at(&table, f, Row(3), true);
    assert_eq!(table.handles(), &[a, b, c, f, d, e]);
    assert_eq!(table.columns().ids.as_slice(), &[10, 20, 30, 60, 40, 50]);
    assert_eq!(
        table.columns().values.as_slice(),
        &[100, 200, 300, 600, 400, 500]
    );
    assert_alive_at(&table, d, Row(4), false);
    assert_alive_at(&table, e, Row(5), false);
    assert_directory_consistent(&table);

    // `set_active` de un handle muerto: `false` sin cambiar `active_len()` ni nada más. Vale
    // una generación equivocada, un slot nunca usado y un slot fuera del directorio.
    let before = snapshot(&table);
    for dead in [handle(0, 2), handle(7, 1), handle(8, 1), handle(300, 3)] {
        assert_dead(&table, dead);
        assert!(!table.set_active(dead, false), "{dead:?} muerto");
        assert!(!table.set_active(dead, true), "{dead:?} muerto");
        assert_eq!(table.active_len(), 4, "{dead:?} muerto");
    }
    assert_eq!(snapshot(&table), before);

    // Activar una inactiva: entra en la fila `active_len() == 4` por intercambio con la que
    // está ahí (`d`), y después `e` también.
    assert!(table.set_active(e, true));
    assert_eq!(table.handles(), &[a, b, c, f, e, d]);
    assert_eq!(table.active_len(), 5);
    assert!(table.is_active(e));
    assert!(!table.is_active(d));
    assert!(table.set_active(d, true));
    assert_eq!(table.handles(), &[a, b, c, f, e, d]);
    assert_eq!(table.active_len(), 6);
    assert_directory_consistent(&table);

    // Una fila con borrado pendiente se desactiva igual, y `apply_removals` la borra igual.
    assert!(table.mark_removed(b));
    assert!(table.set_active(b, false)); // b (fila 1) <-> fila 5 (d)
    assert_eq!(table.handles(), &[a, d, c, f, e, b]);
    assert_eq!(table.active_len(), 5);
    assert!(!table.is_active(b));
    assert!(table.is_alive(b));
    assert_eq!(table.pending_removals(), &[b]);
    let mut removed = FixedVec::<Handle>::with_capacity(8);
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[b]);
    assert_dead(&table, b);
    assert_eq!(table.handles(), &[a, d, c, f, e]);
    assert_eq!(table.len(), 5);
    assert_eq!(table.active_len(), 5);
    assert_eq!(table.pending_removals(), []);
    assert_eq!(table.columns().ids.as_slice(), &[10, 40, 30, 60, 50]);
    assert_directory_consistent(&table);
    // Y un pendiente inactivo que se reactiva también se borra.
    assert!(table.set_active(f, false));
    assert!(table.mark_removed(f));
    assert!(table.set_active(f, true));
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[f]);
    assert_dead(&table, f);
    assert_eq!(table.len(), 4);
    assert_eq!(table.active_len(), 4);
    assert_directory_consistent(&table);

    // `[a b c d | e f]` con `active_len` 4: `mark_removed(b)` y `apply_removals` dejan
    // `[a, d, c, f, e]` y `active_len() == 3`.
    let mut six: Table<TestColumns> = Table::with_capacity(8);
    let ents: Vec<Handle> = (0..6_u32)
        .map(|i| {
            six.insert(row((i + 1) * 10, u64::from(i + 1) * 100))
                .expect("caben seis")
        })
        .collect();
    let [a, b, c, d, e, f] = ents[..] else {
        panic!("seis handles")
    };
    assert!(six.set_active(f, false));
    assert!(six.set_active(e, false));
    assert_eq!(six.handles(), &[a, b, c, d, e, f]);
    assert_eq!(six.active_len(), 4);
    assert!(six.mark_removed(b));
    six.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[b]);
    assert_eq!(six.handles(), &[a, d, c, f, e]);
    assert_eq!(six.active_len(), 3);
    assert_eq!(six.len(), 5);
    assert_eq!(six.columns().ids.as_slice(), &[10, 40, 30, 60, 50]);
    assert_eq!(six.columns().values.as_slice(), &[100, 400, 300, 600, 500]);
    assert_alive_at(&six, a, Row(0), true);
    assert_alive_at(&six, d, Row(1), true);
    assert_alive_at(&six, c, Row(2), true);
    assert_alive_at(&six, f, Row(3), false);
    assert_alive_at(&six, e, Row(4), false);
    assert_dead(&six, b);
    assert_directory_consistent(&six);
}

// CA-4 (apoyo, regla 19): `set_active` no asigna en ninguno de sus caminos.
#[test]
fn ca_04_set_active_no_asigna() {
    let mut table: Table<TestColumns> = Table::with_capacity(64);
    let ents: Vec<Handle> = (0..64_u32)
        .map(|i| table.insert(row(i, u64::from(i))).expect("caben 64"))
        .collect();
    let dead = handle(0, 2);
    let mut results = [false; 4];
    let info = allocation_counter::measure(|| {
        results[0] = table.set_active(ents[5], false);
        results[1] = table.set_active(ents[5], false);
        results[2] = table.set_active(ents[5], true);
        results[3] = table.set_active(dead, false);
    });
    assert_eq!(
        info.count_total, 0,
        "set_active asignó {} veces ({} bytes)",
        info.count_total, info.bytes_total
    );
    assert_eq!(results, [true, false, true, false]);
    assert_eq!(table.active_len(), 64);
}

/// Seis entidades con ids 10..=60, las dos últimas inactivas: `[a b c d | e f]`.
fn six_with_two_inactive() -> (Table<TestColumns>, [Handle; 6]) {
    let mut table: Table<TestColumns> = Table::with_capacity(8);
    let ents: [Handle; 6] = core::array::from_fn(|i| {
        let i = u32_of(i);
        table
            .insert(row((i + 1) * 10, u64::from(i + 1) * 100))
            .expect("caben seis")
    });
    assert!(table.set_active(ents[5], false));
    assert!(table.set_active(ents[4], false));
    assert_eq!(table.handles(), &ents[..]);
    assert_eq!(table.active_len(), 4);
    (table, ents)
}

// CA-5: Con seis entidades, dos inactivas: `permute_active(&[Row(3), Row(0), Row(2),
// Row(1)])` deja en la fila `i` los valores que tenía `order[i]`, `handle_at` coherente con
// `row_of` para las seis, las dos inactivas intactas en las filas 4 y 5; `permute_active`
// con una fila repetida, con una fila ≥ `active_len()` o con longitud distinta de
// `active_len()` hace pánico `permutación inválida`; `permute_active(&[])` sobre una tabla
// sin activas no hace nada; `allocation_counter::measure` cuenta 0 asignaciones en una
// permutación válida de 4.096 filas; `Column::swap` con una fila ≥ `len` hace pánico `fila
// fuera de la tabla`; un `pop` directo en `columns_mut()` seguido de `insert` hace pánico
// `columnas desalineadas`. Lo demuestran: tests `ca_05_permutacion_de_activas`,
// `ca_05_permutacion_invalida_panica`, `ca_05_swap_fuera_de_rango_panica` y
// `ca_05_columnas_desalineadas_panica`.
#[expect(
    clippy::many_single_char_names,
    reason = "los nombres a..f son los del criterio"
)]
#[test]
fn ca_05_permutacion_de_activas() {
    let (mut table, ents) = six_with_two_inactive();
    let [a, b, c, d, e, f] = ents;
    // Un borrado pendiente sobrevive a la permutación (caso límite "permute_active con
    // borrados pendientes").
    assert!(table.mark_removed(c));

    let order = [Row(3), Row(0), Row(2), Row(1)];
    table.permute_active(&order);
    // La fila `i` tiene lo que tenía `order[i]`: d, a, c, b.
    assert_eq!(table.handles(), &[d, a, c, b, e, f]);
    assert_eq!(table.columns().ids.as_slice(), &[40, 10, 30, 20, 50, 60]);
    assert_eq!(
        table.columns().values.as_slice(),
        &[400, 100, 300, 200, 500, 600]
    );
    assert_eq!(table.len(), 6);
    assert_eq!(table.active_len(), 4);
    assert_alive_at(&table, d, Row(0), true);
    assert_alive_at(&table, a, Row(1), true);
    assert_alive_at(&table, c, Row(2), true);
    assert_alive_at(&table, b, Row(3), true);
    assert_alive_at(&table, e, Row(4), false);
    assert_alive_at(&table, f, Row(5), false);
    assert_directory_consistent(&table);
    assert_eq!(table.pending_removals(), &[c]);

    // La identidad no cambia nada; la inversa deshace la permutación.
    let before = snapshot(&table);
    table.permute_active(&[Row(0), Row(1), Row(2), Row(3)]);
    assert_eq!(snapshot(&table), before);
    table.permute_active(&[Row(1), Row(3), Row(2), Row(0)]);
    assert_eq!(table.handles(), &[a, b, c, d, e, f]);
    assert_eq!(table.columns().ids.as_slice(), &[10, 20, 30, 40, 50, 60]);
    assert_directory_consistent(&table);

    // El pendiente se borra después por su fila actual.
    let mut removed = FixedVec::<Handle>::with_capacity(8);
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[c]);
    assert_eq!(table.handles(), &[a, b, d, f, e]);
    assert_eq!(table.active_len(), 3);
    assert_directory_consistent(&table);

    // `permute_active(&[])` sobre una tabla sin activas no hace nada: vacía y toda inactiva.
    let mut empty: Table<TestColumns> = Table::with_capacity(4);
    let before = snapshot(&empty);
    empty.permute_active(&[]);
    assert_eq!(snapshot(&empty), before);
    let mut all_inactive: Table<TestColumns> = Table::with_capacity(4);
    let x = all_inactive.insert(row(1, 1)).expect("insert");
    let y = all_inactive.insert(row(2, 2)).expect("insert");
    assert!(all_inactive.set_active(x, false));
    assert!(all_inactive.set_active(y, false));
    assert_eq!(all_inactive.active_len(), 0);
    let before = snapshot(&all_inactive);
    all_inactive.permute_active(&[]);
    assert_eq!(snapshot(&all_inactive), before);
    assert_directory_consistent(&all_inactive);

    // Una permutación válida de 4.096 filas no asigna nada (regla 19).
    let mut big: Table<TestColumns> = Table::with_capacity(BIG_PERMUTATION_ROWS);
    for i in 0..BIG_PERMUTATION_ROWS {
        let i = u32_of(i);
        big.insert(row(i, u64::from(i) * 3)).expect("caben 4.096");
    }
    assert_eq!(big.active_len(), BIG_PERMUTATION_ROWS);
    let order = shuffled_rows(BIG_PERMUTATION_ROWS, 0x0C0F_FEE0_0000_4096);
    assert!(
        order.iter().enumerate().any(|(i, r)| r.index() != i),
        "la permutación de 4.096 filas no es la identidad"
    );
    let old_handles = big.handles().to_vec();
    let old_ids = big.columns().ids.as_slice().to_vec();
    let info = allocation_counter::measure(|| big.permute_active(&order));
    assert_eq!(
        info.count_total, 0,
        "permute_active asignó {} veces ({} bytes)",
        info.count_total, info.bytes_total
    );
    assert_eq!(big.len(), BIG_PERMUTATION_ROWS);
    assert_eq!(big.active_len(), BIG_PERMUTATION_ROWS);
    for (i, source) in order.iter().enumerate() {
        assert_eq!(big.handles()[i], old_handles[source.index()], "fila {i}");
        assert_eq!(big.columns().ids[Row(u32_of(i))], old_ids[source.index()]);
    }
    assert_directory_consistent(&big);
}

// CA-5 (continuación): `permute_active` con una fila repetida, con una fila
// `>= active_len()` o con longitud distinta de `active_len()` hace pánico `permutación
// inválida`. Cada caso sobre una tabla recién construida, porque el estado tras un pánico no
// está fijado.
#[test]
fn ca_05_permutacion_invalida_panica() {
    let cases: [(&str, &[Row]); 5] = [
        ("fila repetida", &[Row(3), Row(0), Row(0), Row(1)]),
        ("fila >= active_len", &[Row(3), Row(0), Row(2), Row(4)]),
        ("fila muy lejos", &[Row(3), Row(0), Row(2), Row(u32::MAX)]),
        ("longitud menor", &[Row(2), Row(0), Row(1)]),
        ("longitud mayor", &[Row(3), Row(0), Row(2), Row(1), Row(4)]),
    ];
    for (label, order) in cases {
        let (mut table, _) = six_with_two_inactive();
        assert_panics_with(label, "permutación inválida", || {
            table.permute_active(order);
        });
    }
    // Y sobre una tabla sin activas, cualquier fila sobra.
    let mut empty: Table<TestColumns> = Table::with_capacity(4);
    assert_panics_with("sin activas", "permutación inválida", || {
        empty.permute_active(&[Row(0)]);
    });
}

// CA-5 (continuación): `Column::swap` con una fila `>= len` hace pánico `fila fuera de la
// tabla`.
#[test]
#[should_panic(expected = "fila fuera de la tabla")]
fn ca_05_swap_fuera_de_rango_panica() {
    let mut column = Column::<u32>::with_capacity(4);
    column.push(1);
    column.push(2);
    assert_eq!(column.len(), 2);
    column.swap(Row(0), Row(2));
}

// CA-5 (continuación): un `pop` directo en `columns_mut()` seguido de `insert` hace pánico
// `columnas desalineadas`.
#[test]
#[should_panic(expected = "columnas desalineadas")]
fn ca_05_columnas_desalineadas_panica() {
    let mut table: Table<TestColumns> = Table::with_capacity(4);
    table.insert(row(1, 1)).expect("insert");
    table.insert(row(2, 2)).expect("insert");
    // Operación estructural por la puerta de atrás: las columnas pierden una fila y `handles`
    // no se entera.
    Columns::pop(table.columns_mut());
    assert_eq!(table.columns().len(), 1);
    let _ = table.insert(row(3, 3));
}

// ---------------------------------------------------------------------------------------
// CA-6
// ---------------------------------------------------------------------------------------

/// La tabla grande de CA-6: 1.000 entidades (id `i`, valor `i * 7 + 1`), 100 inactivas (las
/// de id múltiplo de 10) y 50 borrados pendientes (25 inactivas, id múltiplo de 40, y 25
/// activas, id `≡ 5 mod 40`), en una tabla de capacidad `capacity`.
fn build_big_table(capacity: u32) -> Table<TestColumns> {
    assert!(capacity >= BIG_TABLE_ROWS);
    let mut table: Table<TestColumns> = Table::with_capacity(usize_of(capacity));
    let ents: Vec<Handle> = (0..BIG_TABLE_ROWS)
        .map(|i| {
            table
                .insert(row(i, u64::from(i) * 7 + 1))
                .expect("caben 1.000")
        })
        .collect();
    for (i, handle) in ents.iter().enumerate() {
        if i % 10 == 0 {
            assert!(table.set_active(*handle, false));
        }
    }
    for (i, handle) in ents.iter().enumerate() {
        if i % 40 == 0 || i % 40 == 5 {
            assert!(table.mark_removed(*handle));
        }
    }
    assert_eq!(table.len(), 1_000);
    assert_eq!(table.active_len(), 900);
    assert_eq!(table.pending_removals().len(), 50);
    table
}

// CA-6: La tabla mínima de "Datos y formatos" codifica con `postcard::to_allocvec`
// exactamente a los 20 bytes fijados y vuelve igual con `from_bytes`. [...] Lo demuestran:
// tests `ca_06_tabla_bytes_fijados`, `ca_06_tabla_serde_ida_y_vuelta_con_capacidad`,
// `ca_06_tabla_corrupta_no_deserializa` y `ca_06_tabla_hash_segun_la_secuencia`.
#[test]
fn ca_06_tabla_bytes_fijados() {
    // Capacidad 2, una entidad en el slot 0 con generación 1, activa, una columna `u8` con 7.
    let mut table: Table<ByteColumns> = Table::with_capacity(2);
    let a = table.insert(7).expect("insert");
    assert_eq!(a, handle(0, 1));
    let bytes = postcard::to_allocvec(&table).expect("serializar la tabla");
    assert_eq!(bytes, FIXED_TABLE_BYTES, "los 20 bytes fijados");

    let mut back: Table<ByteColumns> =
        postcard::from_bytes(&FIXED_TABLE_BYTES).expect("deserializar los 20 bytes");
    assert_eq!(back.capacity(), 2);
    assert_eq!(back.len(), 1);
    assert_eq!(back.active_len(), 1);
    assert_eq!(back.handles(), &[a]);
    assert_eq!(back.pending_removals(), []);
    assert_eq!(back.columns().bytes.as_slice(), &[7]);
    assert_eq!(back.columns(), table.columns());
    assert_alive_at(&back, a, Row(0), true);
    assert_eq!(hash_of(&back), hash_of(&table));
    // Vuelve a codificar a los mismos bytes.
    assert_eq!(
        postcard::to_allocvec(&back).expect("serializar la cargada"),
        FIXED_TABLE_BYTES
    );
    // La capacidad se ha reservado: cabe una más (slot 1, generación 1) y después `TableFull`.
    assert_eq!(back.columns().bytes.capacity(), 2);
    assert_eq!(back.insert(9), Ok(handle(1, 1)));
    assert_eq!(back.insert(9), Err(TableFull { capacity: 2 }));
    assert_eq!(table.insert(9), Ok(handle(1, 1)));
    assert_eq!(hash_of(&back), hash_of(&table));

    // La secuencia del hash de la tabla mínima, escrita a mano.
    let minimal: Table<ByteColumns> =
        postcard::from_bytes(&FIXED_TABLE_BYTES).expect("deserializar los 20 bytes");
    let expected = handwritten_table_hash(
        &[handle(0, 1)],
        1,
        &[0, u32::MAX],
        &[1, 1],
        &[1],
        &[],
        |hasher| {
            hasher.write_u64(1);
            hasher.write(&[7]);
        },
    );
    assert_eq!(hash_of(&minimal), expected);
}

// CA-6 (continuación): Una tabla con 1.000 entidades, 100 inactivas y 50 borrados
// pendientes: ida y vuelta por `postcard` devuelve una tabla con el mismo `hash_of`, los
// mismos `handles()`, `active_len()` y `pending_removals()`, y los mismos valores de
// columnas; tras cargar, `insert` hasta la capacidad original no asigna y el siguiente da
// `TableFull`; `apply_removals` en las dos tablas da los mismos `removed`. Además, tras
// cargar, `mark_removed` de un pendiente ya cargado devuelve `false` (la bandera por slot se
// reconstruye).
#[test]
fn ca_06_tabla_serde_ida_y_vuelta_con_capacidad() {
    let mut original = build_big_table(BIG_TABLE_CAPACITY);
    let bytes = postcard::to_allocvec(&original).expect("serializar la tabla grande");
    let mut loaded: Table<TestColumns> =
        postcard::from_bytes(&bytes).expect("deserializar la tabla grande");

    assert_eq!(hash_of(&loaded), hash_of(&original));
    assert_eq!(snapshot(&loaded), snapshot(&original));
    assert_eq!(loaded.capacity(), usize_of(BIG_TABLE_CAPACITY));
    assert_eq!(loaded.len(), 1_000);
    assert_eq!(loaded.active_len(), 900);
    assert_eq!(loaded.handles(), original.handles());
    assert_eq!(loaded.pending_removals(), original.pending_removals());
    assert_eq!(loaded.columns(), original.columns());
    assert_directory_consistent(&loaded);
    for handle in original.handles() {
        assert_eq!(loaded.row_of(*handle), original.row_of(*handle));
        assert_eq!(loaded.is_active(*handle), original.is_active(*handle));
    }
    // Las columnas han recuperado la capacidad, no solo la longitud.
    assert_eq!(
        loaded.columns().ids.capacity(),
        usize_of(BIG_TABLE_CAPACITY)
    );
    assert_eq!(
        loaded.columns().values.capacity(),
        usize_of(BIG_TABLE_CAPACITY)
    );

    // La bandera de pendiente por slot se reconstruye: marcar un pendiente cargado da `false`
    // y no lo duplica; marcar uno no pendiente da `true` en las dos tablas.
    let pending = loaded.pending_removals().to_vec();
    for handle in &pending {
        assert!(!loaded.mark_removed(*handle), "{handle:?} ya era pendiente");
    }
    assert_eq!(loaded.pending_removals(), &pending[..]);
    let not_pending = loaded.handle_at(Row(1));
    assert!(!pending.contains(&not_pending));
    assert!(loaded.mark_removed(not_pending));
    assert!(original.mark_removed(not_pending));
    assert_eq!(loaded.pending_removals(), original.pending_removals());
    assert_eq!(hash_of(&loaded), hash_of(&original));

    // `insert` hasta la capacidad original no asigna y el siguiente da `TableFull`.
    let gap = usize_of(BIG_TABLE_CAPACITY - BIG_TABLE_ROWS);
    assert_eq!(gap, 24);
    let rows: [TestRow; 24] = core::array::from_fn(|i| row(2_000 + u32_of(i), 5));
    let mut results = [Err(TableFull { capacity: 0 }); 24];
    let info = allocation_counter::measure(|| {
        for (out, test_row) in results.iter_mut().zip(rows) {
            *out = loaded.insert(test_row);
        }
    });
    assert_eq!(
        info.count_total, 0,
        "los insert tras cargar asignaron {} veces ({} bytes)",
        info.count_total, info.bytes_total
    );
    for (i, (result, test_row)) in results.iter().zip(rows).enumerate() {
        let expected = original.insert(test_row).expect("el original tiene hueco");
        assert_eq!(*result, Ok(expected), "insert {i} tras cargar");
    }
    assert_eq!(loaded.len(), usize_of(BIG_TABLE_CAPACITY));
    assert_eq!(
        loaded.insert(row(9, 9)),
        Err(TableFull {
            capacity: usize_of(BIG_TABLE_CAPACITY)
        })
    );
    assert_eq!(
        original.insert(row(9, 9)),
        Err(TableFull {
            capacity: usize_of(BIG_TABLE_CAPACITY)
        })
    );
    assert_eq!(hash_of(&loaded), hash_of(&original));

    // `apply_removals` en las dos tablas da los mismos `removed`, y después siguen iguales.
    let mut removed_loaded = FixedVec::<Handle>::with_capacity(64);
    let mut removed_original = FixedVec::<Handle>::with_capacity(64);
    loaded.apply_removals(&mut removed_loaded);
    original.apply_removals(&mut removed_original);
    assert_eq!(removed_loaded.len(), 51);
    assert_eq!(removed_loaded.as_slice(), removed_original.as_slice());
    assert_eq!(snapshot(&loaded), snapshot(&original));
    assert_eq!(hash_of(&loaded), hash_of(&original));
    assert_directory_consistent(&loaded);
    // Y la pila de libres reconstruida reutiliza los mismos slots con las mismas generaciones.
    for i in 0..51_u32 {
        assert_eq!(loaded.insert(row(i, 0)), original.insert(row(i, 0)));
    }
    assert_eq!(hash_of(&loaded), hash_of(&original));

    // Ida y vuelta de la tabla cargada, ya mutada: sigue siendo la misma.
    let again = postcard::to_allocvec(&loaded).expect("serializar la cargada");
    let reloaded: Table<TestColumns> = postcard::from_bytes(&again).expect("deserializar");
    assert_eq!(hash_of(&reloaded), hash_of(&loaded));
    assert_eq!(snapshot(&reloaded), snapshot(&loaded));
}

/// Espejo de la codificación `serde` de `Table<TestColumns>` de "Datos y formatos", campo a
/// campo y en el mismo orden, para fabricar bytes corruptos sin manipular varints a mano.
/// Antes de corromper nada, el test comprueba que el espejo válido codifica a los mismos
/// bytes que la tabla real.
#[derive(Clone, Serialize)]
struct RawTable {
    capacity: u32,
    dir_rows: Vec<u32>,
    dir_generations: Vec<u32>,
    free: Vec<u32>,
    handles: Vec<RawHandle>,
    active_len: u32,
    pending: Vec<RawHandle>,
    columns: RawColumns,
}

/// `Handle` como sus dos campos `u32`.
#[derive(Clone, Copy, Serialize)]
struct RawHandle {
    slot: u32,
    generation: u32,
}

/// `TestColumns` como dos secuencias.
#[derive(Clone, Serialize)]
struct RawColumns {
    ids: Vec<u32>,
    values: Vec<u64>,
}

fn raw(slot: u32, generation: u32) -> RawHandle {
    RawHandle { slot, generation }
}

/// La tabla válida de la que salen las corrupciones: capacidad 4, `a` (slot 0) y `b` (slot 1)
/// activas, `b` pendiente, slots 2 y 3 libres (pila `[3, 2]`, el 2 en la cima).
fn valid_raw_table() -> RawTable {
    RawTable {
        capacity: 4,
        dir_rows: vec![0, 1, u32::MAX, u32::MAX],
        dir_generations: vec![1, 1, 1, 1],
        free: vec![3, 2],
        handles: vec![raw(0, 1), raw(1, 1)],
        active_len: 2,
        pending: vec![raw(1, 1)],
        columns: RawColumns {
            ids: vec![10, 20],
            values: vec![100, 200],
        },
    }
}

/// La misma tabla que `valid_raw_table`, construida por la API.
fn valid_table() -> Table<TestColumns> {
    let mut table: Table<TestColumns> = Table::with_capacity(4);
    let a = table.insert(row(10, 100)).expect("insert a");
    let b = table.insert(row(20, 200)).expect("insert b");
    assert_eq!([a, b], [handle(0, 1), handle(1, 1)]);
    assert!(table.mark_removed(b));
    table
}

// CA-6 (continuación): fallan al deserializar, cada uno con su caso, unos bytes con una
// columna de longitud distinta de `handles()`, con `capacity` menor que `len`, con una
// generación 0, con un slot libre repetido, con `free` de longitud distinta de
// `capacity - len`, con un slot `>= capacity` en `handles` y con `active_len` mayor que
// `len`, ninguno con pánico. Los bytes corruptos salen del espejo `RawTable` de los bytes
// válidos, cambiando solo el campo que toque.
#[expect(
    clippy::too_many_lines,
    reason = "un caso por invariante de \"Datos y formatos\""
)]
#[test]
fn ca_06_tabla_corrupta_no_deserializa() {
    // El espejo válido codifica a los mismos bytes que la tabla real y se deserializa bien.
    let valid = valid_raw_table();
    let valid_bytes = postcard::to_allocvec(&valid).expect("serializar el espejo");
    let table = valid_table();
    assert_eq!(
        valid_bytes,
        postcard::to_allocvec(&table).expect("serializar la tabla"),
        "el espejo y la tabla codifican igual"
    );
    let loaded: Table<TestColumns> =
        postcard::from_bytes(&valid_bytes).expect("el espejo válido se deserializa");
    assert_eq!(snapshot(&loaded), snapshot(&table));
    assert_eq!(hash_of(&loaded), hash_of(&table));

    // Cada corrupción: el campo que cambia y nada más.
    let corrupt = |label: &'static str, mutate: fn(&mut RawTable)| {
        let mut raw = valid_raw_table();
        mutate(&mut raw);
        (label, raw)
    };
    let cases: Vec<(&str, RawTable)> = vec![
        // Los siete casos del criterio.
        corrupt("columna de longitud distinta de handles", |t| {
            t.columns.ids.push(30);
            t.columns.values.push(300);
        }),
        corrupt("capacity menor que len", |t| {
            t.capacity = 1;
            t.dir_rows = vec![0];
            t.dir_generations = vec![1];
            t.free = vec![];
            t.handles = vec![raw(0, 1), raw(0, 1)];
            t.pending = vec![];
        }),
        corrupt("generación 0", |t| t.dir_generations[2] = 0),
        corrupt("slot libre repetido", |t| t.free = vec![2, 2]),
        corrupt("free de longitud distinta de capacity - len", |t| {
            t.free = vec![3];
        }),
        corrupt("slot >= capacity en handles", |t| t.handles[1] = raw(7, 1)),
        corrupt("active_len mayor que len", |t| t.active_len = 3),
        // Más invariantes de "Datos y formatos".
        corrupt("capacity == u32::MAX", |t| {
            t.capacity = u32::MAX;
            t.dir_rows = vec![];
            t.dir_generations = vec![];
            t.free = vec![];
            t.handles = vec![];
            t.active_len = 0;
            t.pending = vec![];
            t.columns = RawColumns {
                ids: vec![],
                values: vec![],
            };
        }),
        corrupt("dir_rows de longitud distinta de capacity", |t| {
            t.dir_rows.push(u32::MAX);
        }),
        corrupt("dir_generations de longitud distinta de capacity", |t| {
            t.dir_generations.pop();
        }),
        corrupt("fila del directorio fuera de la tabla", |t| {
            t.dir_rows[0] = 9;
        }),
        corrupt("fila del directorio que no es la del handle", |t| {
            t.dir_rows[0] = 1;
            t.dir_rows[1] = 0;
        }),
        corrupt("generación del handle distinta del directorio", |t| {
            t.handles[0] = raw(0, 2);
        }),
        corrupt("slot libre que está en handles", |t| t.free = vec![3, 1]),
        corrupt("slot libre con fila en el directorio", |t| {
            t.dir_rows[2] = 1;
        }),
        corrupt("slot >= capacity en free", |t| t.free = vec![3, 9]),
        corrupt("slot >= capacity en pending", |t| {
            t.pending = vec![raw(9, 1)];
        }),
        corrupt("pendiente muerto", |t| t.pending = vec![raw(2, 1)]),
        corrupt("pendiente con otra generación", |t| {
            t.pending = vec![raw(1, 2)];
        }),
        corrupt("pendiente repetido", |t| {
            t.pending = vec![raw(1, 1), raw(1, 1)];
        }),
        corrupt("handle repetido en handles", |t| {
            t.handles[1] = raw(0, 1);
            t.dir_rows[1] = u32::MAX;
            t.free = vec![3, 2, 1];
        }),
    ];
    for (label, raw_table) in &cases {
        let bytes = postcard::to_allocvec(raw_table).expect("serializar el espejo corrupto");
        assert_ne!(
            bytes, valid_bytes,
            "{label}: la corrupción cambia los bytes"
        );
        let result = catch_unwind(|| postcard::from_bytes::<Table<TestColumns>>(&bytes));
        match result {
            Ok(Ok(table)) => panic!(
                "{label}: se deserializó una tabla con {} filas y capacidad {}",
                table.len(),
                table.capacity()
            ),
            Ok(Err(_)) => {}
            Err(payload) => panic!(
                "{label}: pánico al deserializar en vez de error: {}",
                panic_message(&payload)
            ),
        }
    }
}

// CA-6 (continuación): el `hash_of` de la tabla es igual a la secuencia de la sección
// `table` escrita a mano con un `StateHasher`, y distinto del de una tabla igual con
// capacidad 2.000.
#[test]
fn ca_06_tabla_hash_segun_la_secuencia() {
    let table = build_big_table(BIG_TABLE_CAPACITY);
    // La tabla no ha aplicado ningún borrado: todos los slots tienen generación 1, los slots
    // 0..1000 están vivos y la pila de libres va de 1023 (fondo) a 1000 (cima).
    let capacity = usize_of(BIG_TABLE_CAPACITY);
    let dir_rows: Vec<u32> = (0..BIG_TABLE_CAPACITY)
        .map(|slot| table.row_of(handle(slot, 1)).map_or(u32::MAX, |row| row.0))
        .collect();
    assert_eq!(dir_rows.iter().filter(|r| **r == u32::MAX).count(), 24);
    let dir_generations = vec![1_u32; capacity];
    let free: Vec<u32> = (BIG_TABLE_ROWS..BIG_TABLE_CAPACITY).rev().collect();
    assert_eq!(free.first(), Some(&1_023));
    assert_eq!(free.last(), Some(&1_000));
    let expected = handwritten_table_hash(
        table.handles(),
        table.active_len(),
        &dir_rows,
        &dir_generations,
        &free,
        table.pending_removals(),
        hash_test_columns(
            table.columns().ids.as_slice(),
            table.columns().values.as_slice(),
        ),
    );
    assert_eq!(hash_of(&table), expected, "la secuencia escrita a mano");
    // La misma secuencia por `write_value` sobre la tabla.
    let mut hasher = StateHasher::new();
    hasher.write_value(&table);
    assert_eq!(hasher.finish(), expected);

    // Una tabla igual con capacidad 2.000 da otro hash: el directorio y la pila son más largos.
    let other = build_big_table(OTHER_CAPACITY);
    assert_eq!(other.handles(), table.handles());
    assert_eq!(other.active_len(), table.active_len());
    assert_eq!(other.pending_removals(), table.pending_removals());
    assert_eq!(other.columns(), table.columns());
    assert_ne!(hash_of(&other), hash_of(&table));
    let other_dir_rows: Vec<u32> = (0..OTHER_CAPACITY)
        .map(|slot| other.row_of(handle(slot, 1)).map_or(u32::MAX, |row| row.0))
        .collect();
    let other_free: Vec<u32> = (BIG_TABLE_ROWS..OTHER_CAPACITY).rev().collect();
    let other_expected = handwritten_table_hash(
        other.handles(),
        other.active_len(),
        &other_dir_rows,
        &vec![1_u32; usize_of(OTHER_CAPACITY)],
        &other_free,
        other.pending_removals(),
        hash_test_columns(
            other.columns().ids.as_slice(),
            other.columns().values.as_slice(),
        ),
    );
    assert_eq!(hash_of(&other), other_expected);

    // Lo derivado no entra: aplicar y volver a marcar no cambia el hash si el estado canónico
    // es el mismo, y sí cambia cuando cambia un valor, el conjunto activo o un pendiente.
    let mut mutated = table.clone();
    assert_eq!(hash_of(&mutated), hash_of(&table));
    mutated.columns_mut().values[Row(0)] += 1;
    assert_ne!(hash_of(&mutated), hash_of(&table));
    mutated.columns_mut().values[Row(0)] -= 1;
    assert_eq!(hash_of(&mutated), hash_of(&table));
    let first = mutated.handle_at(Row(0));
    assert!(mutated.set_active(first, false));
    assert_ne!(hash_of(&mutated), hash_of(&table));
    assert!(mutated.set_active(first, true));
    // Reactivar deja `first` en la fila `active_len() - 1`: mismo contenido, otro orden de
    // filas, otro hash. Las permutaciones también entran en el hash: deshacer el intercambio
    // con `permute_active` devuelve el orden original y el mismo hash.
    assert_ne!(hash_of(&mutated), hash_of(&table));
    let mut order: Vec<Row> = (0..u32_of(mutated.active_len())).map(Row).collect();
    let moved = mutated.row_of(first).expect("first vive").0;
    assert_eq!(usize_of(moved), mutated.active_len() - 1);
    order.swap(0, usize_of(moved));
    mutated.permute_active(&order);
    assert_eq!(mutated.handles(), table.handles());
    assert_eq!(hash_of(&mutated), hash_of(&table));
}

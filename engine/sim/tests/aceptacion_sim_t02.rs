//! Tests de aceptación de P02-T02: `buffer`, `Column`, `Columns` y `Table` hasta
//! `apply_removals`.
//!
//! Cubren CA-1, CA-2 y CA-22 de `specs/architecture/engine_sim.md`. Solo usan la API
//! pública de `engine_sim` y `engine_core` y no los modifica el implementador. Las
//! columnas de prueba (`TestColumns`) se definen aquí a mano, una línea por columna y
//! método, como hará `game_rules` (ADR-0004). Cada pánico con mensaje fijo tiene su
//! propio test con `#[should_panic(expected = ...)]`, porque un test solo puede
//! comprobar un pánico.

use core::num::NonZeroU32;

use engine_core::handle::Handle;
use engine_core::hash::hash_of;
use engine_sim::buffer::FixedVec;
use engine_sim::table::{Column, Columns, Row, Table, TableFull};

/// Valores de una entidad de prueba al insertarla: el único struct por entidad.
#[derive(Clone, Copy, Debug)]
struct TestRow {
    id: u32,
    weight: f32,
}

/// Columnas de prueba: una `Column` por campo de `TestRow`. `Clone` derivado: cada `Column`
/// conserva su capacidad, así que `Table::clone` también.
#[derive(Clone, Debug)]
struct TestColumns {
    ids: Column<u32>,
    weights: Column<f32>,
}

impl Columns for TestColumns {
    type Row = TestRow;

    fn with_capacity(capacity: usize) -> TestColumns {
        TestColumns {
            ids: Column::with_capacity(capacity),
            weights: Column::with_capacity(capacity),
        }
    }

    fn reserve(&mut self, capacity: usize) {
        self.ids.reserve(capacity);
        self.weights.reserve(capacity);
    }

    fn len(&self) -> usize {
        // Lo que recomienda la spec: la tabla solo compara este valor con el suyo, así que la
        // desalineación entre dos columnas la detecta quien implementa `Columns`.
        debug_assert_eq!(
            self.ids.len(),
            self.weights.len(),
            "columnas de prueba desalineadas"
        );
        self.ids.len()
    }

    fn push(&mut self, row: TestRow) {
        self.ids.push(row.id);
        self.weights.push(row.weight);
    }

    fn swap(&mut self, a: Row, b: Row) {
        self.ids.swap(a, b);
        self.weights.swap(a, b);
    }

    fn pop(&mut self) {
        self.ids.pop();
        self.weights.pop();
    }

    fn clear(&mut self) {
        self.ids.clear();
        self.weights.clear();
    }
}

/// Generación no nula a partir de un entero; los tests solo la piden con valores válidos.
fn nz(generation: u32) -> NonZeroU32 {
    NonZeroU32::new(generation).expect("la generación de un handle nunca es 0")
}

/// Handle a partir de dos enteros, para abreviar los tests.
fn handle(slot: u32, generation: u32) -> Handle {
    Handle::new(slot, nz(generation))
}

/// Fila de prueba con un identificador y un peso.
fn row(id: u32, weight: f32) -> TestRow {
    TestRow { id, weight }
}

/// Los pesos como bits, para compararlos con igualdad exacta sin comparar flotantes.
fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

/// Foto de todo lo observable de una tabla, para comprobar que una operación "no cambia nada".
#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    capacity: usize,
    len: usize,
    active_len: usize,
    handles: Vec<Handle>,
    pending: Vec<Handle>,
    ids: Vec<u32>,
    weight_bits: Vec<u32>,
}

fn snapshot(table: &Table<TestColumns>) -> Snapshot {
    Snapshot {
        capacity: table.capacity(),
        len: table.len(),
        active_len: table.active_len(),
        handles: table.handles().to_vec(),
        pending: table.pending_removals().to_vec(),
        ids: table.columns().ids.as_slice().to_vec(),
        weight_bits: bits(table.columns().weights.as_slice()),
    }
}

/// Comprueba que `handle` está vivo y activo en la fila `row`, y que `row_of` y `handle_at`
/// son inversas la una de la otra.
fn assert_alive_at(table: &Table<TestColumns>, handle: Handle, row: Row) {
    assert!(table.is_alive(handle), "{handle:?} debería estar vivo");
    assert!(table.is_active(handle), "{handle:?} debería estar activo");
    assert_eq!(table.row_of(handle), Some(row), "fila de {handle:?}");
    assert_eq!(table.handle_at(row), handle, "handle de {row:?}");
    assert_eq!(table.handles()[row.index()], handle, "handles()[{row:?}]");
}

/// Comprueba que `handle` está muerto: ni vivo, ni activo, ni con fila.
fn assert_dead(table: &Table<TestColumns>, handle: Handle) {
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

// CA-1: `Table::with_capacity(8)` con columnas de prueba: tres `insert` devuelven handles con
// slots 0, 1 y 2 y generación 1, en las filas 0, 1 y 2, `len() == 3`, `active_len() == 3`,
// `row_of`, `handle_at` e `is_alive` coherentes, `handles()` igual a los tres en orden; cinco
// `insert` más llenan la tabla y el noveno devuelve `Err(TableFull { capacity: 8 })` sin
// cambiar nada; `allocation_counter::measure` cuenta 0 asignaciones en los ocho `insert`;
// `clone()` conserva `capacity() == 8`, los mismos `handles()` y valores, y admite `insert`
// hasta la capacidad sin pánico tras `clear()`; tras `clear()` de tres entidades, los tres
// handles están muertos y el siguiente `insert` reutiliza el slot de la antigua fila 0 con
// generación 2, el siguiente el de la fila 1; `with_capacity(u32::MAX as usize)` hace pánico
// `capacidad de tabla excesiva`; `handle_at(Row(3))` con tres filas hace pánico `fila fuera
// de la tabla`. Lo demuestran: tests `ca_01_tabla_insercion_llena_clon_y_clear`,
// `ca_01_tabla_capacidad_excesiva_panica` y `ca_01_handle_at_fuera_panica`.
#[expect(
    clippy::too_many_lines,
    reason = "un test por criterio, como fija la spec"
)]
#[test]
fn ca_01_tabla_insercion_llena_clon_y_clear() {
    let mut table: Table<TestColumns> = Table::with_capacity(8);
    assert_eq!(table.capacity(), 8);
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
    assert_eq!(table.active_len(), 0);
    assert_eq!(table.handles(), []);
    assert_eq!(table.pending_removals(), []);
    assert_eq!(table.columns().len(), 0);
    assert!(table.columns().is_empty());
    assert_eq!(table.columns().ids.capacity(), 8);
    assert_eq!(table.columns().weights.capacity(), 8);

    // Tres inserciones: slots 0, 1 y 2 con generación 1, en las filas 0, 1 y 2.
    let a = table.insert(row(10, 1.5)).expect("primer insert");
    let b = table.insert(row(20, 2.5)).expect("segundo insert");
    let c = table.insert(row(30, 3.5)).expect("tercer insert");
    assert_eq!(a, handle(0, 1));
    assert_eq!(b, handle(1, 1));
    assert_eq!(c, handle(2, 1));
    assert_eq!(a.generation(), Handle::FIRST_GENERATION);
    assert_eq!(table.len(), 3);
    assert!(!table.is_empty());
    assert_eq!(table.active_len(), 3);
    assert_alive_at(&table, a, Row(0));
    assert_alive_at(&table, b, Row(1));
    assert_alive_at(&table, c, Row(2));
    assert_eq!(table.handles(), &[a, b, c]);
    assert_eq!(Row(2).index(), 2);
    assert_eq!(table.columns().len(), 3);
    assert_eq!(table.columns().ids.as_slice(), &[10, 20, 30]);
    assert_eq!(
        bits(table.columns().weights.as_slice()),
        bits(&[1.5, 2.5, 3.5])
    );
    assert_eq!(table.columns().ids[Row(1)], 20);
    assert_eq!(table.columns().weights[Row(2)].to_bits(), 3.5_f32.to_bits());
    // Un handle con otra generación en un slot ocupado no está vivo.
    assert_dead(&table, handle(0, 2));
    // Ni uno de un slot que nunca se ha usado.
    assert_dead(&table, handle(7, 1));

    // `columns_mut` sirve para escribir valores sin tocar la estructura.
    table.columns_mut().ids[Row(1)] = 21;
    assert_eq!(table.columns().ids.as_slice(), &[10, 21, 30]);
    table.columns_mut().ids[Row(1)] = 20;

    // Un clon de la tabla con tres entidades admite cinco inserciones más sin pánico ni
    // `TableFull`: conserva la capacidad.
    let mut clone_of_three = table.clone();
    assert_eq!(clone_of_three.capacity(), 8);
    assert_eq!(clone_of_three.handles(), &[a, b, c]);
    for i in 3..8_u32 {
        let inserted = clone_of_three
            .insert(row((i + 1) * 10, 0.5))
            .expect("el clon tiene capacidad para ocho");
        assert_eq!(inserted, handle(i, 1));
    }
    assert_eq!(clone_of_three.len(), 8);
    assert_eq!(
        clone_of_three.insert(row(90, 9.5)),
        Err(TableFull { capacity: 8 })
    );
    // El original no se ha enterado.
    assert_eq!(table.len(), 3);
    assert_eq!(table.handles(), &[a, b, c]);

    // Cinco inserciones más llenan la tabla original.
    let more: Vec<Handle> = (3..8_u32)
        .map(|i| {
            table
                .insert(row((i + 1) * 10, 0.25))
                .expect("caben ocho entidades")
        })
        .collect();
    for (i, inserted) in (3..8_u32).zip(&more) {
        assert_eq!(*inserted, handle(i, 1));
        assert_alive_at(&table, *inserted, Row(i));
    }
    assert_eq!(table.len(), 8);
    assert_eq!(table.active_len(), 8);
    assert_eq!(table.columns().len(), 8);
    assert_eq!(
        table.columns().ids.as_slice(),
        &[10, 20, 30, 40, 50, 60, 70, 80]
    );

    // El noveno `insert` devuelve `TableFull` y no cambia nada observable.
    let before = snapshot(&table);
    assert_eq!(table.insert(row(90, 9.5)), Err(TableFull { capacity: 8 }));
    assert_eq!(snapshot(&table), before);
    assert_eq!(
        TableFull { capacity: 8 }.to_string(),
        "tabla llena: capacidad 8"
    );

    // Los ocho `insert` en una tabla recién creada no asignan nada (regla 19).
    let mut fresh: Table<TestColumns> = Table::with_capacity(8);
    let rows: [TestRow; 8] = core::array::from_fn(|i| {
        let id = u32::try_from(i).expect("ocho filas caben en u32") * 10;
        row(id, 0.5)
    });
    let mut results = [Err(TableFull { capacity: 0 }); 8];
    let info = allocation_counter::measure(|| {
        for (out, test_row) in results.iter_mut().zip(rows) {
            *out = fresh.insert(test_row);
        }
    });
    assert_eq!(
        info.count_total, 0,
        "los ocho insert asignaron {} veces ({} bytes)",
        info.count_total, info.bytes_total
    );
    for (i, result) in (0..8_u32).zip(results) {
        assert_eq!(result, Ok(handle(i, 1)));
    }
    assert_eq!(fresh.len(), 8);
    assert_eq!(fresh.handles(), table.handles());

    // El clon de la tabla llena conserva capacidad, handles y valores, y tras `clear()` admite
    // `insert` hasta la capacidad sin pánico.
    let mut cloned = table.clone();
    assert_eq!(cloned.capacity(), 8);
    assert_eq!(cloned.len(), 8);
    assert_eq!(cloned.active_len(), 8);
    assert_eq!(cloned.handles(), table.handles());
    assert_eq!(
        cloned.columns().ids.as_slice(),
        table.columns().ids.as_slice()
    );
    assert_eq!(
        bits(cloned.columns().weights.as_slice()),
        bits(table.columns().weights.as_slice())
    );
    assert_eq!(snapshot(&cloned), snapshot(&table));
    cloned.clear();
    assert_eq!(cloned.len(), 0);
    assert_eq!(cloned.active_len(), 0);
    assert_eq!(cloned.capacity(), 8);
    assert_eq!(cloned.handles(), []);
    assert_eq!(cloned.columns().len(), 0);
    for i in 0..8_u32 {
        let reinserted = cloned
            .insert(row(i, 0.5))
            .expect("el clon vaciado admite ocho insert");
        assert_eq!(reinserted.slot(), i);
        assert_eq!(reinserted, handle(i, 2));
    }
    assert_eq!(cloned.insert(row(99, 0.5)), Err(TableFull { capacity: 8 }));
    // El original sigue intacto tras vaciar el clon.
    assert_eq!(snapshot(&table), before);
    assert_alive_at(&table, a, Row(0));

    // `clear()` de tres entidades: los tres handles mueren y los slots se reutilizan en el orden
    // de la antigua fila 0, 1 y 2, con generación 2; el cuarto `insert` toma un slot nunca usado.
    let mut three: Table<TestColumns> = Table::with_capacity(8);
    let a3 = three.insert(row(1, 1.0)).expect("insert");
    let b3 = three.insert(row(2, 2.0)).expect("insert");
    let c3 = three.insert(row(3, 3.0)).expect("insert");
    assert_eq!([a3, b3, c3], [handle(0, 1), handle(1, 1), handle(2, 1)]);
    three.clear();
    assert_eq!(three.len(), 0);
    assert!(three.is_empty());
    assert_eq!(three.active_len(), 0);
    assert_eq!(three.capacity(), 8);
    assert_eq!(three.handles(), []);
    assert_eq!(three.pending_removals(), []);
    assert_eq!(three.columns().len(), 0);
    assert_dead(&three, a3);
    assert_dead(&three, b3);
    assert_dead(&three, c3);
    let a4 = three.insert(row(4, 4.0)).expect("insert tras clear");
    assert_eq!(a4, handle(0, 2));
    assert_eq!(a4.generation(), Handle::next_generation(a3.generation()));
    assert_alive_at(&three, a4, Row(0));
    let b4 = three.insert(row(5, 5.0)).expect("insert tras clear");
    assert_eq!(b4, handle(1, 2));
    assert_alive_at(&three, b4, Row(1));
    let c4 = three.insert(row(6, 6.0)).expect("insert tras clear");
    assert_eq!(c4, handle(2, 2));
    let d4 = three.insert(row(7, 7.0)).expect("insert tras clear");
    assert_eq!(d4, handle(3, 1));
    // Compartir slot con una entidad nueva no resucita a la vieja.
    assert_dead(&three, a3);
    assert_dead(&three, b3);
    assert_dead(&three, c3);
    assert_eq!(three.handles(), &[a4, b4, c4, d4]);
    assert_eq!(three.columns().ids.as_slice(), &[4, 5, 6, 7]);

    // `clear()` sobre una tabla vacía no hace nada.
    let mut empty: Table<TestColumns> = Table::with_capacity(8);
    let empty_before = snapshot(&empty);
    empty.clear();
    assert_eq!(snapshot(&empty), empty_before);
}

// CA-1 (continuación): `with_capacity(u32::MAX as usize)` hace pánico `capacidad de tabla
// excesiva`.
#[test]
#[should_panic(expected = "capacidad de tabla excesiva")]
fn ca_01_tabla_capacidad_excesiva_panica() {
    let excessive = usize::try_from(u32::MAX).expect("u32::MAX cabe en usize");
    let table: Table<TestColumns> = Table::with_capacity(excessive);
    drop(table);
}

// CA-1 (continuación): `handle_at(Row(3))` con tres filas hace pánico `fila fuera de la tabla`.
#[test]
#[should_panic(expected = "fila fuera de la tabla")]
fn ca_01_handle_at_fuera_panica() {
    let mut table: Table<TestColumns> = Table::with_capacity(8);
    for i in 0..3_u32 {
        table.insert(row(i, 0.5)).expect("caben tres");
    }
    assert_eq!(table.len(), 3);
    let out_of_table = table.handle_at(Row(3));
    unreachable!("handle_at(Row(3)) con tres filas devolvió {out_of_table:?}");
}

// CA-1 (apoyo): `Column<T>` conserva la capacidad al clonar, se serializa igual que su
// `FixedVec` (`02 07 08`) y entra en el hash como `[T]`. Las columnas de prueba de CA-1
// dependen de estas garantías de `Column`, fijadas en la sección `table` de la spec.
#[test]
fn ca_01_column_clone_serde_y_hash_como_slice() {
    let mut column = Column::<u8>::with_capacity(4);
    assert_eq!(column.len(), 0);
    assert!(column.is_empty());
    assert_eq!(column.capacity(), 4);
    column.push(7);
    column.push(8);
    assert_eq!(column.len(), 2);
    assert!(!column.is_empty());
    assert_eq!(column.as_slice(), &[7, 8]);
    assert_eq!(column[Row(0)], 7);
    assert_eq!(column[Row(1)], 8);
    column[Row(1)] = 9;
    assert_eq!(column.as_slice(), &[7, 9]);
    column.as_mut_slice()[1] = 8;
    assert_eq!(Row(1).index(), 1);

    let cloned = column.clone();
    assert_eq!(cloned.capacity(), 4);
    assert_eq!(cloned.as_slice(), &[7, 8]);
    assert_eq!(cloned, column);

    let bytes = postcard::to_allocvec(&column).expect("serializar la columna");
    assert_eq!(bytes, [0x02, 0x07, 0x08]);
    let mut back: Column<u8> = postcard::from_bytes(&bytes).expect("deserializar la columna");
    assert_eq!(back.as_slice(), &[7, 8]);
    assert_eq!(back.capacity(), 2);
    assert_eq!(back, column);
    back.reserve(4);
    assert_eq!(back.capacity(), 4);
    back.push(9);
    assert_eq!(back.as_slice(), &[7, 8, 9]);

    assert_eq!(hash_of(&column), hash_of(&[7_u8, 8][..]));
    assert_eq!(hash_of(&cloned), hash_of(&column));

    column.swap(Row(0), Row(1));
    assert_eq!(column.as_slice(), &[8, 7]);
    column.swap(Row(0), Row(0));
    assert_eq!(column.as_slice(), &[8, 7]);
    assert_eq!(column.pop(), Some(7));
    assert_eq!(column.pop(), Some(8));
    assert_eq!(column.pop(), None);
    column.push(1);
    column.clear();
    assert_eq!(column.len(), 0);
    assert_eq!(column.capacity(), 4);
}

// CA-2: Con cuatro entidades `a, b, c, d` en las filas 0 a 3: `mark_removed(b)` y
// `mark_removed(d)` devuelven `true`, un segundo `mark_removed(b)` devuelve `false`,
// `is_alive(b)` sigue cierto y `pending_removals() == [b, d]`; `apply_removals` deja
// `removed == [d, b]` (fila descendente), `len() == 2`, `row_of(a) == Some(Row(0))`,
// `row_of(c) == Some(Row(1))`, `row_of(b) == None`, `is_alive(d)` falso y
// `pending_removals()` vacío; las columnas tienen los valores de `a` y `c` en las filas 0 y 1;
// el siguiente `insert` reutiliza el slot de `b` (el último liberado) con generación
// `next_generation(b.generation())`, y el siguiente el de `d`; `is_alive(b)` e `is_alive(d)`
// siguen falsos aunque compartan slot con los nuevos; `apply_removals` sin pendientes no
// cambia nada; `mark_removed` de un handle muerto devuelve `false`; con la tabla llena,
// `mark_removed` seguido de `insert` da `TableFull` hasta `apply_removals`. Lo demuestra: test
// `ca_02_borrado_diferido_descendente_y_generaciones`.
#[test]
fn ca_02_borrado_diferido_descendente_y_generaciones() {
    // Capacidad 4: con `a, b, c, d` la tabla está llena, que es lo que pide el último punto.
    let mut table: Table<TestColumns> = Table::with_capacity(4);
    let a = table.insert(row(10, 1.5)).expect("insert a");
    let b = table.insert(row(20, 2.5)).expect("insert b");
    let c = table.insert(row(30, 3.5)).expect("insert c");
    let d = table.insert(row(40, 4.5)).expect("insert d");
    assert_eq!(table.handles(), &[a, b, c, d]);
    assert_eq!(
        [a, b, c, d],
        [handle(0, 1), handle(1, 1), handle(2, 1), handle(3, 1)]
    );
    assert_eq!(table.pending_removals(), []);

    // Marcar no borra: `b` y `d` siguen vivas, en su fila, hasta `apply_removals`.
    assert!(table.mark_removed(b));
    assert!(table.mark_removed(d));
    assert!(
        !table.mark_removed(b),
        "segundo mark_removed del mismo handle"
    );
    assert!(table.is_alive(b));
    assert!(table.is_alive(d));
    assert_alive_at(&table, b, Row(1));
    assert_alive_at(&table, d, Row(3));
    assert_eq!(table.pending_removals(), &[b, d]);
    assert_eq!(table.len(), 4);
    assert_eq!(table.active_len(), 4);
    assert_eq!(table.handles(), &[a, b, c, d]);

    // Aplicar: por fila descendente, `d` (fila 3) antes que `b` (fila 1).
    let mut removed = FixedVec::<Handle>::with_capacity(4);
    removed.push(handle(9, 9)); // basura previa: `apply_removals` vacía `removed` antes de escribir
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[d, b]);
    assert_eq!(removed.capacity(), 4);
    assert_eq!(table.len(), 2);
    assert_eq!(table.active_len(), 2);
    assert_eq!(table.capacity(), 4);
    assert_eq!(table.row_of(a), Some(Row(0)));
    assert_eq!(table.row_of(c), Some(Row(1)));
    assert_eq!(table.row_of(b), None);
    assert_eq!(table.row_of(d), None);
    assert!(!table.is_alive(b));
    assert!(!table.is_alive(d));
    assert_dead(&table, b);
    assert_dead(&table, d);
    assert_alive_at(&table, a, Row(0));
    assert_alive_at(&table, c, Row(1));
    assert_eq!(table.handles(), &[a, c]);
    assert_eq!(table.pending_removals(), []);
    assert_eq!(table.columns().len(), 2);
    assert_eq!(table.columns().ids.as_slice(), &[10, 30]);
    assert_eq!(bits(table.columns().weights.as_slice()), bits(&[1.5, 3.5]));

    // La pila de libres devuelve primero el último slot liberado: el de `b`, después el de `d`,
    // cada uno con la generación siguiente a la que tenía.
    let e_in_b = table.insert(row(50, 5.5)).expect("insert e_in_b");
    assert_eq!(e_in_b.slot(), b.slot());
    assert_eq!(e_in_b.generation(), Handle::next_generation(b.generation()));
    assert_eq!(e_in_b, handle(1, 2));
    assert_alive_at(&table, e_in_b, Row(2));
    let f_in_d = table.insert(row(60, 6.5)).expect("insert f_in_d");
    assert_eq!(f_in_d.slot(), d.slot());
    assert_eq!(f_in_d.generation(), Handle::next_generation(d.generation()));
    assert_eq!(f_in_d, handle(3, 2));
    assert_alive_at(&table, f_in_d, Row(3));
    // Compartir slot no resucita a `b` ni a `d`.
    assert_dead(&table, b);
    assert_dead(&table, d);
    assert_eq!(table.handles(), &[a, c, e_in_b, f_in_d]);
    assert_eq!(table.len(), 4);
    assert_eq!(table.active_len(), 4);
    assert_eq!(table.columns().ids.as_slice(), &[10, 30, 50, 60]);

    // `apply_removals` sin pendientes no cambia nada de la tabla y deja `removed` vacío.
    let before = snapshot(&table);
    table.apply_removals(&mut removed);
    assert!(removed.is_empty());
    assert_eq!(removed.capacity(), 4);
    assert_eq!(snapshot(&table), before);

    // `mark_removed` de un handle muerto devuelve `false` y no apunta nada.
    assert!(!table.mark_removed(b));
    assert!(!table.mark_removed(d));
    assert!(!table.mark_removed(handle(0, 7)));
    assert_eq!(table.pending_removals(), []);
    assert_eq!(snapshot(&table), before);

    // Con la tabla llena, marcar no libera sitio: `insert` da `TableFull` hasta `apply_removals`.
    assert_eq!(table.len(), table.capacity());
    assert!(table.mark_removed(a));
    assert_eq!(table.pending_removals(), &[a]);
    assert_eq!(table.insert(row(70, 7.5)), Err(TableFull { capacity: 4 }));
    assert!(table.is_alive(a), "TableFull no borra la pendiente");
    assert_eq!(table.pending_removals(), &[a]);
    assert_eq!(table.len(), 4);
    assert_eq!(table.handles(), &[a, c, e_in_b, f_in_d]);
    table.apply_removals(&mut removed);
    assert_eq!(removed.as_slice(), &[a]);
    assert_dead(&table, a);
    assert_eq!(table.len(), 3);
    assert_eq!(table.active_len(), 3);
    // Fila 0 activa: se intercambia con la fila `active_len() - 1 == 3`, baja `active_len` y se
    // hace `pop` de la última, así que `f_in_d` pasa a la fila 0.
    assert_eq!(table.handles(), &[f_in_d, c, e_in_b]);
    assert_eq!(table.columns().ids.as_slice(), &[60, 30, 50]);
    assert_alive_at(&table, f_in_d, Row(0));
    assert_alive_at(&table, c, Row(1));
    assert_alive_at(&table, e_in_b, Row(2));
    let g_in_a = table
        .insert(row(70, 7.5))
        .expect("insert g_in_a tras apply_removals");
    assert_eq!(g_in_a, handle(0, 2));
    assert_alive_at(&table, g_in_a, Row(3));
    assert_dead(&table, a);
    assert_eq!(table.len(), 4);
    assert_eq!(table.insert(row(80, 8.5)), Err(TableFull { capacity: 4 }));
}

// CA-2 (continuación, doc de `apply_removals`): `removed` tiene que tener capacidad para
// `pending_removals().len()`; si no, pánico `columna llena`.
#[test]
#[should_panic(expected = "columna llena")]
fn ca_02_apply_removals_sin_sitio_en_removed_panica() {
    let mut table: Table<TestColumns> = Table::with_capacity(4);
    let a = table.insert(row(10, 1.5)).expect("insert a");
    let b = table.insert(row(20, 2.5)).expect("insert b");
    assert!(table.mark_removed(a));
    assert!(table.mark_removed(b));
    let mut removed = FixedVec::<Handle>::with_capacity(1);
    table.apply_removals(&mut removed);
}

// CA-22: `FixedVec::<u8>::with_capacity(4)`: `push` de 7 y 8, `len() == 2`, `capacity() == 4`,
// `as_slice() == [7, 8]`; `try_push(9)` da `Ok(())`, `try_push(10)` da `Ok(())` y
// `try_push(11)` da `Err(11)` con el buffer intacto; `push` con el buffer lleno hace pánico
// `columna llena`; `pop()` devuelve `Some(10)`; `truncate(1)` deja `[7]`; `swap(0, 0)` no
// cambia nada y `swap(0, 1)` con `len() == 1` hace pánico `índice fuera del buffer`; `clear()`
// deja `len() == 0` y `capacity() == 4`; `clone()` de un buffer con `[7, 8]` conserva
// `capacity() == 4`; `postcard::to_allocvec` de `[7, 8]` da `02 07 08`, `from_bytes` devuelve
// `[7, 8]` con `capacity() == 2` y `reserve(4)` la sube a 4; `hash_of` del buffer es igual a
// `hash_of(&[7u8, 8][..])`. Lo demuestran: tests `ca_22_fixedvec_capacidad_fija` y
// `ca_22_fixedvec_lleno_y_fuera_de_rango_panican`.
#[test]
fn ca_22_fixedvec_capacidad_fija() {
    let mut buffer = FixedVec::<u8>::with_capacity(4);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert_eq!(buffer.capacity(), 4);
    assert_eq!(buffer.as_slice(), []);
    assert_eq!(buffer.pop(), None);

    buffer.push(7);
    buffer.push(8);
    assert_eq!(buffer.len(), 2);
    assert!(!buffer.is_empty());
    assert_eq!(buffer.capacity(), 4);
    assert_eq!(buffer.as_slice(), &[7, 8]);
    assert_eq!(buffer[0], 7);
    assert_eq!(buffer[1], 8);

    // Un buffer con `[7, 8]` y capacidad 4, para el clon, los bytes y el hash de más abajo.
    let two = buffer.clone();
    assert_eq!(two.capacity(), 4);
    assert_eq!(two.len(), 2);
    assert_eq!(two.as_slice(), &[7, 8]);
    assert_eq!(two, buffer);

    // `try_push` llena hasta la capacidad y después devuelve el valor sin tocar el buffer.
    assert_eq!(buffer.try_push(9), Ok(()));
    assert_eq!(buffer.try_push(10), Ok(()));
    assert_eq!(buffer.len(), 4);
    assert_eq!(buffer.try_push(11), Err(11));
    assert_eq!(buffer.as_slice(), &[7, 8, 9, 10]);
    assert_eq!(buffer.len(), 4);
    assert_eq!(buffer.capacity(), 4);

    assert_eq!(buffer.pop(), Some(10));
    assert_eq!(buffer.len(), 3);
    assert_eq!(buffer.as_slice(), &[7, 8, 9]);

    // `swap` dentro de rango y `IndexMut` / `as_mut_slice` escriben en su sitio.
    buffer.swap(0, 2);
    assert_eq!(buffer.as_slice(), &[9, 8, 7]);
    buffer.swap(2, 0);
    assert_eq!(buffer.as_slice(), &[7, 8, 9]);
    buffer[2] = 90;
    assert_eq!(buffer.as_slice(), &[7, 8, 90]);
    buffer.as_mut_slice()[2] = 9;
    assert_eq!(buffer.as_slice(), &[7, 8, 9]);

    // `truncate` recorta a 1 y no hace nada si pide más de lo que hay; nunca toca la capacidad.
    buffer.truncate(1);
    assert_eq!(buffer.as_slice(), &[7]);
    assert_eq!(buffer.capacity(), 4);
    buffer.truncate(5);
    assert_eq!(buffer.as_slice(), &[7]);
    assert_eq!(buffer.capacity(), 4);

    buffer.swap(0, 0);
    assert_eq!(buffer.as_slice(), &[7]);

    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert_eq!(buffer.capacity(), 4);
    assert_eq!(buffer.pop(), None);
    // Tras `clear` vuelven a caber cuatro.
    for value in 1..=4_u8 {
        assert_eq!(buffer.try_push(value), Ok(()));
    }
    assert_eq!(buffer.try_push(5), Err(5));
    assert_eq!(buffer.as_slice(), &[1, 2, 3, 4]);

    // `postcard`: la misma secuencia que un `Vec<u8>` con `[7, 8]`.
    let bytes = postcard::to_allocvec(&two).expect("serializar el buffer");
    assert_eq!(bytes, [0x02, 0x07, 0x08]);
    let mut back: FixedVec<u8> = postcard::from_bytes(&bytes).expect("deserializar el buffer");
    assert_eq!(back.as_slice(), &[7, 8]);
    assert_eq!(back.len(), 2);
    assert_eq!(
        back.capacity(),
        2,
        "tras deserializar, capacidad igual a longitud"
    );
    // `==` ignora la capacidad: 2 frente a 4.
    assert_eq!(back, two);
    // Con capacidad 2 no cabe un tercero hasta `reserve`.
    assert_eq!(back.try_push(9), Err(9));
    back.reserve(4);
    assert_eq!(back.capacity(), 4);
    assert_eq!(back.as_slice(), &[7, 8]);
    assert_eq!(back.try_push(9), Ok(()));
    assert_eq!(back.as_slice(), &[7, 8, 9]);
    // `reserve` con menos no encoge.
    back.reserve(1);
    assert_eq!(back.capacity(), 4);
    assert_eq!(back.as_slice(), &[7, 8, 9]);
    // Un buffer vacío deserializado tiene capacidad 0 y `try_push` falla hasta `reserve`.
    let mut empty: FixedVec<u8> = postcard::from_bytes(&[0x00]).expect("deserializar vacío");
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0);
    assert_eq!(empty.try_push(1), Err(1));
    empty.reserve(1);
    assert_eq!(empty.try_push(1), Ok(()));

    // `hash_of` como `[T]`: longitud y elementos; la capacidad no entra.
    assert_eq!(hash_of(&two), hash_of(&[7_u8, 8][..]));
    back.truncate(2);
    assert_eq!(hash_of(&back), hash_of(&two));
    assert_ne!(hash_of(&two), hash_of(&[7_u8][..]));
}

// CA-22 (continuación): `push` con el buffer lleno hace pánico `columna llena`.
#[test]
#[should_panic(expected = "columna llena")]
fn ca_22_fixedvec_lleno_panica() {
    let mut buffer = FixedVec::<u8>::with_capacity(4);
    for value in 7..=10_u8 {
        buffer.push(value);
    }
    assert_eq!(buffer.len(), 4);
    buffer.push(11);
}

// CA-22 (continuación): `swap(0, 1)` con `len() == 1` hace pánico `índice fuera del buffer`.
#[test]
#[should_panic(expected = "índice fuera del buffer")]
fn ca_22_fixedvec_fuera_de_rango_panica() {
    let mut buffer = FixedVec::<u8>::with_capacity(4);
    buffer.push(7);
    assert_eq!(buffer.len(), 1);
    buffer.swap(0, 1);
}

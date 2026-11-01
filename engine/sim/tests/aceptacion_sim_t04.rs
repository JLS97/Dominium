//! Tests de aceptación de P02-T04: `parallel` y `events`.
//!
//! Cubren CA-7, CA-8 y CA-9 de `specs/architecture/engine_sim.md`. Solo usan la API
//! pública de `engine_sim` y no los modifica el implementador. El scratch de cada test
//! es una struct propia (`ChunkRecord`, `EventScratch`, `CounterScratch`), como hará
//! `game_rules`, y las referencias se calculan en serie con bucles simples sobre la
//! entrada.
//!
//! Todo lo que produce resultado se comprueba con pools de 1, 3, 6 y 12 hilos
//! (`Pool::new(n)` en el propio test; `rayon` admite más hilos que núcleos). Nada sale
//! de la aleatoriedad del sistema: la entrada se deriva del índice de fila con una
//! mezcla fija. Los pánicos van uno por test con `#[should_panic(expected = ...)]`;
//! como CA-7 y CA-8 agrupan dos pánicos en un nombre, cada uno se parte con un sufijo.

use engine_sim::buffer::FixedVec;
use engine_sim::events::{EventKey, SortedEvents};
use engine_sim::parallel::{
    Accumulate, CHUNK, ChunkId, ChunkScratch, Pool, PoolError, chunk_count, par_map, par_scan,
    reduce_in_order,
};
use engine_sim::table::Row;

/// Filas de las fases grandes de CA-7, CA-8 y CA-9.
const ROWS: usize = 10_000;
/// Chunks que ocupan `ROWS` filas: `10_000.div_ceil(512)`.
const EXPECTED_CHUNKS: usize = 20;
/// Filas del último chunk: `10_000 - 19 * 512`.
const LAST_CHUNK_LEN: usize = 272;
/// Chunks sobrantes del scratch "más grande de lo necesario" de CA-7 y CA-9.
const SPARE_CHUNKS: usize = 5;
/// Los pools con los que se comprueba todo lo que produce resultado.
const THREADS: [usize; 4] = [1, 3, 6, 12];
/// El chunk que no emite ningún evento en CA-8: en medio, con vecinos que sí emiten.
const SILENT_CHUNK: ChunkId = ChunkId(7);
/// Capacidad del buffer de eventos de cada chunk en CA-8: como máximo tres por fila.
const MAX_EVENTS_PER_CHUNK: usize = 3 * CHUNK;
/// Capacidad insuficiente para el chunk 0 en el test de `columna llena`: el chunk 0 emite
/// `128 * (0 + 1 + 2 + 3) = 768` eventos.
const SMALL_EVENT_CAPACITY: usize = 100;
/// Clave única del caso "todos los eventos de la misma clave" de CA-8.
const SAME_KEY: (u64, u64, u32) = (5, 5, 5);
/// Constante de mezcla de la entrada de CA-7 (la de Fibonacci en 64 bits; cualquier impar vale).
const MIX: u64 = 0x9E37_79B9_7F4A_7C15;
/// Multiplicador de la fase de CA-7.
const PHASE_FACTOR: u64 = 31;

// ---------------------------------------------------------------------------------------
// Ayudantes comunes
// ---------------------------------------------------------------------------------------

/// `usize` como `u32`; en los tests nunca falla.
fn u32_of(value: usize) -> u32 {
    u32::try_from(value).expect("el valor cabe en u32")
}

/// `usize` como `u64`; en los tests nunca falla.
fn u64_of(value: usize) -> u64 {
    u64::try_from(value).expect("el valor cabe en u64")
}

/// Un pool con `threads` hilos, o pánico con el error.
fn pool(threads: usize) -> Pool {
    Pool::new(threads).unwrap_or_else(|error| panic!("Pool::new({threads}) falló: {error}"))
}

/// La entrada de CA-7: una mezcla fija del índice de fila, distinta en cada fila.
fn mixed(row: usize) -> u64 {
    u64_of(row).wrapping_mul(MIX)
}

/// Las filas `0..len` como `u32`, para que una fase sepa su fila global sin mirar el chunk.
fn row_indices(len: usize) -> Vec<u32> {
    (0..len).map(u32_of).collect()
}

/// Lo que la fase de CA-7 escribe en la fila `row` a partir de `value`: una función de los dos,
/// para que la salida detecte tanto un valor mal copiado como un trozo mal situado.
fn phase_output(value: u64, row: usize) -> u64 {
    value.wrapping_mul(PHASE_FACTOR).wrapping_add(u64_of(row))
}

/// Longitud del trozo del chunk `chunk` para `len` filas: `CHUNK` salvo el último.
fn expected_chunk_len(chunk: usize, len: usize) -> usize {
    let start = chunk * CHUNK;
    assert!(start < len, "el chunk {chunk} no existe para {len} filas");
    (len - start).min(CHUNK)
}

// ---------------------------------------------------------------------------------------
// CA-7: chunks fijos, Pool, ChunkScratch, par_map y par_scan
// ---------------------------------------------------------------------------------------

/// Lo que cada chunk registra al ejecutar la fase de CA-7.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChunkRecord {
    /// El `ChunkId` que recibió la fase; `None` si nunca se llamó.
    seen: Option<ChunkId>,
    /// La longitud del trozo que recibió.
    len: usize,
    /// Veces que se llamó a la fase para este chunk.
    calls: u32,
    /// Filas del trozo de entrada cuyo valor no es el de su fila global.
    bad_rows: u32,
}

/// El centinela con el que se inicializa cada chunk: detecta tanto una llamada que falta como
/// una escritura fuera de los chunks necesarios.
const UNTOUCHED: ChunkRecord = ChunkRecord {
    seen: None,
    len: usize::MAX,
    calls: 0,
    bad_rows: 0,
};

/// Un scratch de `chunks_for` filas con todos los chunks en `UNTOUCHED`.
fn fresh_records(chunks_for: usize) -> ChunkScratch<ChunkRecord> {
    ChunkScratch::new(chunks_for, |_| UNTOUCHED)
}

/// Registra en `record` la llamada de la fase para `chunk` sobre el trozo `input`, que empieza
/// en la fila global `chunk.first_row()`.
fn record_call(chunk: ChunkId, input: &[u64], record: &mut ChunkRecord) {
    let first = chunk.first_row().index();
    record.seen = Some(chunk);
    record.len = input.len();
    record.calls += 1;
    record.bad_rows += u32_of(
        input
            .iter()
            .enumerate()
            .filter(|&(k, &value)| value != mixed(first + k))
            .count(),
    );
}

/// El registro que debe tener el chunk `chunk` del scratch tras una fase sobre `len` filas.
fn expected_record(chunk: usize, len: usize) -> ChunkRecord {
    if chunk * CHUNK >= len {
        return UNTOUCHED;
    }
    ChunkRecord {
        seen: Some(ChunkId(u32_of(chunk))),
        len: expected_chunk_len(chunk, len),
        calls: 1,
        bad_rows: 0,
    }
}

/// Comprueba que el scratch tiene exactamente los registros de una fase sobre `len` filas, en
/// orden de chunk, con los chunks sobrantes intactos.
fn assert_records(label: &str, scratch: &ChunkScratch<ChunkRecord>, len: usize) {
    let mut seen_chunks = 0;
    for (index, (chunk, record)) in scratch.iter().enumerate() {
        assert_eq!(
            chunk,
            ChunkId(u32_of(index)),
            "{label}: iter fuera de orden"
        );
        assert_eq!(
            *record,
            expected_record(index, len),
            "{label}: registro del chunk {index} para {len} filas"
        );
        assert_eq!(scratch.get(chunk), record, "{label}: get del chunk {index}");
        seen_chunks += 1;
    }
    assert_eq!(
        seen_chunks,
        scratch.len(),
        "{label}: iter recorre todos los chunks"
    );
}

/// Ejecuta la fase de CA-7 con `par_map` sobre `len` filas y un scratch de `scratch_len` chunks,
/// y comprueba salida y registros. Devuelve el scratch para seguir comprobando.
fn run_par_map(pool: &Pool, len: usize, scratch_chunks: usize) -> ChunkScratch<ChunkRecord> {
    let label = format!("par_map, {} hilos, {len} filas", pool.threads());
    let input: Vec<u64> = (0..len).map(mixed).collect();
    let mut output = vec![0_u64; len];
    let mut scratch = fresh_records(scratch_chunks * CHUNK);
    assert_eq!(scratch.len(), scratch_chunks, "{label}: chunks del scratch");

    par_map(
        pool,
        &input,
        &mut output,
        &mut scratch,
        |chunk, input, output, record| {
            record_call(chunk, input, record);
            let first = chunk.first_row().index();
            for (k, (out, &value)) in output.iter_mut().zip(input).enumerate() {
                *out = phase_output(value, first + k);
            }
        },
    );

    // La misma salida calculada en serie, con un bucle simple sobre la entrada.
    let serial: Vec<u64> = input
        .iter()
        .enumerate()
        .map(|(row, &value)| phase_output(value, row))
        .collect();
    assert_eq!(
        output, serial,
        "{label}: la salida no es la de la versión en serie"
    );
    assert_records(&label, &scratch, len);
    scratch
}

/// Ejecuta la fase de CA-7 con `par_scan` sobre `len` filas y comprueba que recibe los mismos
/// chunks que `par_map`.
fn run_par_scan(pool: &Pool, len: usize, scratch_chunks: usize) -> ChunkScratch<ChunkRecord> {
    let label = format!("par_scan, {} hilos, {len} filas", pool.threads());
    let input: Vec<u64> = (0..len).map(mixed).collect();
    let mut scratch = fresh_records(scratch_chunks * CHUNK);
    par_scan(pool, &input, &mut scratch, record_call);
    assert_records(&label, &scratch, len);
    scratch
}

/// Lo que CA-7 exige del pool: `Pool::new(0)` da `ZeroThreads` con su mensaje, `threads` devuelve
/// lo pedido (también en los clones), `install` devuelve el valor de la clausura y el pool tiene
/// de verdad los hilos pedidos.
fn assert_pool_api() {
    let zero = Pool::new(0).err().expect("Pool::new(0) tiene que fallar");
    assert!(
        matches!(zero, PoolError::ZeroThreads),
        "Pool::new(0) dio {zero:?}"
    );
    assert_eq!(zero.to_string(), "el pool necesita al menos un hilo");
    let three = pool(3);
    assert_eq!(three.threads(), 3);
    assert_eq!(three.clone().threads(), 3);
    assert_eq!(three.install(|| 6 * 7), 42);
    for &threads in &THREADS {
        let pool = pool(threads);
        assert_eq!(pool.threads(), threads);
        // Dentro de `install` se está en el pool pedido, con exactamente esos hilos: `new` pasa
        // el número explícito a `rayon` y la variable `RAYON_NUM_THREADS` no manda. Es el único
        // sitio donde se pregunta a `rayon`, y es un test, no lógica del crate.
        assert_eq!(
            pool.install(rayon::current_num_threads),
            threads,
            "el pool de {threads} hilos no tiene {threads} hilos"
        );
    }
}

/// Lo que CA-7 exige de `ChunkScratch`: `new` construye en orden con `make`, y `len`, `is_empty`,
/// `get`, `get_mut`, `iter`, `iter_mut` y `Clone` se comportan como dice la spec.
fn assert_chunk_scratch_api() {
    let mut scratch = ChunkScratch::new(ROWS, |chunk| ChunkRecord {
        seen: Some(chunk),
        ..UNTOUCHED
    });
    assert_eq!(scratch.len(), EXPECTED_CHUNKS);
    assert!(!scratch.is_empty());
    for (index, (chunk, record)) in scratch.iter().enumerate() {
        assert_eq!(chunk, ChunkId(u32_of(index)));
        assert_eq!(record.seen, Some(chunk), "make recibió otro chunk");
    }
    for (chunk, record) in scratch.iter_mut() {
        record.calls = chunk.0 + 100;
    }
    assert_eq!(scratch.get(ChunkId(4)).calls, 104);
    scratch.get_mut(ChunkId(4)).calls = 7;
    assert_eq!(scratch.get(ChunkId(4)).calls, 7);
    let mut cloned = scratch.clone();
    assert_eq!(cloned.len(), scratch.len());
    assert!(cloned.iter().zip(scratch.iter()).all(|(a, b)| a == b));
    cloned.get_mut(ChunkId(4)).calls = 99;
    assert_eq!(
        scratch.get(ChunkId(4)).calls,
        7,
        "el clon comparte memoria con el original"
    );
    let empty = fresh_records(0);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.iter().count(), 0);
}

// CA-7: `chunk_count(0) == 0`, `chunk_count(1) == 1`, `chunk_count(512) == 1`,
// `chunk_count(513) == 2`; `ChunkId(3).first_row() == Row(1536)`; `Pool::new(0)` da
// `ZeroThreads` y `Pool::new(3).threads() == 3`; `par_map` sobre 10.000 filas registra en el
// scratch de cada chunk su `ChunkId` y la longitud de su trozo, que son `0..20` con 512 filas
// salvo el último con 272, con pools de 1, 3, 6 y 12 hilos; la salida es igual a la calculada
// en serie; con `len == 0` no llama a `f`; con un scratch de más chunks de los necesarios, los
// sobrantes quedan intactos; `par_map` con `input` y `output` de longitudes distintas hace
// pánico `longitudes distintas`, y con un scratch de menos chunks, `scratch insuficiente`;
// `par_scan` recibe los mismos chunks. Lo demuestran: tests
// `ca_07_chunks_fijos_con_cualquier_pool` y `ca_07_par_map_longitudes_y_scratch_panican`.
#[test]
fn ca_07_chunks_fijos_con_cualquier_pool() {
    // Los chunks son de 512 filas fijas y su número no depende de nada más que de `len`.
    assert_eq!(CHUNK, 512);
    assert_eq!(chunk_count(0), 0);
    assert_eq!(chunk_count(1), 1);
    assert_eq!(chunk_count(512), 1);
    assert_eq!(chunk_count(513), 2);
    assert_eq!(chunk_count(2 * CHUNK), 2);
    assert_eq!(chunk_count(ROWS), EXPECTED_CHUNKS);
    assert_eq!(ChunkId(3).first_row(), Row(1536));
    assert_eq!(ChunkId(0).first_row(), Row(0));
    assert_eq!(ChunkId(19).first_row(), Row(9728));
    assert_eq!(ROWS - ChunkId(19).first_row().index(), LAST_CHUNK_LEN);

    // El pool y la API de `ChunkScratch`, en sus ayudantes.
    assert_pool_api();
    assert_chunk_scratch_api();

    // `par_map` sobre 10.000 filas con pools de 1, 3, 6 y 12 hilos: chunks `0..20`, 512 filas
    // salvo el último con 272, salida igual a la de la versión en serie. Y los casos límite de
    // `CHUNK` y `CHUNK + 1` filas.
    for &threads in &THREADS {
        let pool = pool(threads);
        let scratch = run_par_map(&pool, ROWS, EXPECTED_CHUNKS);
        assert_eq!(scratch.get(ChunkId(0)).len, CHUNK);
        assert_eq!(scratch.get(ChunkId(19)).len, LAST_CHUNK_LEN);
        run_par_map(&pool, CHUNK, 1);
        run_par_map(&pool, CHUNK + 1, 2);
        run_par_map(&pool, 1, 1);

        // Con un scratch de más chunks de los necesarios, los sobrantes quedan intactos.
        let bigger = run_par_map(&pool, ROWS, EXPECTED_CHUNKS + SPARE_CHUNKS);
        for spare in EXPECTED_CHUNKS..EXPECTED_CHUNKS + SPARE_CHUNKS {
            assert_eq!(
                *bigger.get(ChunkId(u32_of(spare))),
                UNTOUCHED,
                "chunk sobrante {spare}"
            );
        }

        // `par_scan` recibe los mismos chunks.
        let scanned = run_par_scan(&pool, ROWS, EXPECTED_CHUNKS);
        assert!(
            scanned.iter().zip(scratch.iter()).all(|(a, b)| a == b),
            "par_scan y par_map reparten distinto con {threads} hilos"
        );
        run_par_scan(&pool, CHUNK + 1, 2);
        let scanned_bigger = run_par_scan(&pool, ROWS, EXPECTED_CHUNKS + SPARE_CHUNKS);
        assert!(
            scanned_bigger
                .iter()
                .zip(bigger.iter())
                .all(|(a, b)| a == b)
        );

        // Con `len == 0` no llama a `f`, ni con scratch vacío ni con scratch de sobra.
        let none: [u64; 0] = [];
        let mut no_output: [u64; 0] = [];
        let mut no_scratch = fresh_records(0);
        par_map(
            &pool,
            &none,
            &mut no_output,
            &mut no_scratch,
            |chunk, _, _, _| {
                panic!("par_map llamó a f con len == 0 para el chunk {chunk:?}");
            },
        );
        par_scan(&pool, &none, &mut no_scratch, |chunk, _, _| {
            panic!("par_scan llamó a f con len == 0 para el chunk {chunk:?}");
        });
        let mut spare_scratch = fresh_records(ROWS);
        par_map(
            &pool,
            &none,
            &mut no_output,
            &mut spare_scratch,
            |_, _, _, record| {
                record.calls += 1;
            },
        );
        par_scan(&pool, &none, &mut spare_scratch, |_, _, record| {
            record.calls += 1;
        });
        assert!(
            spare_scratch.iter().all(|(_, record)| *record == UNTOUCHED),
            "con len == 0 se tocó algún chunk"
        );
    }
}

// CA-7 (pánico `longitudes distintas`): `par_map` con `input` y `output` de longitudes
// distintas. El test de la spec `ca_07_par_map_longitudes_y_scratch_panican` agrupa dos
// pánicos; aquí va partido con sufijos, uno por `should_panic`.
#[test]
#[should_panic(expected = "longitudes distintas")]
fn ca_07_par_map_longitudes_y_scratch_panican_longitudes_distintas() {
    let pool = pool(1);
    let input = [1_u64; 10];
    let mut output = [0_u64; 9];
    let mut scratch = fresh_records(CHUNK);
    par_map(&pool, &input, &mut output, &mut scratch, |_, _, _, _| {});
}

// CA-7 (pánico `scratch insuficiente`): `par_map` con un scratch de menos chunks de los que
// piden las filas (19 para 10.000, que necesitan 20).
#[test]
#[should_panic(expected = "scratch insuficiente")]
fn ca_07_par_map_longitudes_y_scratch_panican_scratch_insuficiente() {
    let pool = pool(1);
    let input = vec![1_u64; ROWS];
    let mut output = vec![0_u64; ROWS];
    let mut scratch = fresh_records((EXPECTED_CHUNKS - 1) * CHUNK);
    assert_eq!(scratch.len(), EXPECTED_CHUNKS - 1);
    par_map(&pool, &input, &mut output, &mut scratch, |_, _, _, _| {});
}

// CA-7 (pánico `scratch insuficiente` en `par_scan`): su doc exige el mismo pánico que
// `par_map` con un scratch corto.
#[test]
#[should_panic(expected = "scratch insuficiente")]
fn ca_07_par_map_longitudes_y_scratch_panican_par_scan_scratch_insuficiente() {
    let pool = pool(1);
    let input = vec![1_u64; ROWS];
    let mut scratch = fresh_records((EXPECTED_CHUNKS - 1) * CHUNK);
    assert_eq!(scratch.len(), EXPECTED_CHUNKS - 1);
    par_scan(&pool, &input, &mut scratch, |_, _, _| {});
}

// ---------------------------------------------------------------------------------------
// CA-8: eventos en orden de chunk y clave
// ---------------------------------------------------------------------------------------

/// Un evento de prueba: la clave `(objetivo, origen, tipo)` más la fila que lo emitió y el
/// número de emisión dentro de la fila, para reconocer el orden de chunk y de emisión.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestEvent {
    target: u64,
    source: u64,
    kind: u32,
    row: u32,
    emission: u32,
}

impl EventKey for TestEvent {
    fn key(&self) -> (u64, u64, u32) {
        (self.target, self.source, self.kind)
    }
}

/// El scratch de la fase de eventos: un `FixedVec` por chunk, como manda la spec.
#[derive(Clone, Debug)]
struct EventScratch {
    events: FixedVec<TestEvent>,
}

/// Un scratch de eventos para `rows` filas con `capacity` eventos por chunk.
fn event_scratch(rows: usize, capacity: usize) -> ChunkScratch<EventScratch> {
    ChunkScratch::new(rows, |_| EventScratch {
        events: FixedVec::with_capacity(capacity),
    })
}

/// Los eventos del chunk, para `SortedEvents::collect`.
fn events_of(scratch: &EventScratch) -> &[TestEvent] {
    scratch.events.as_slice()
}

/// Cuántos eventos emite la fila `row` en la fase principal de CA-8: entre 0 y 3.
fn emissions_of(row: u32) -> u32 {
    row % 4
}

/// El evento `emission` de la fila `row` en la fase principal de CA-8: claves repetidas
/// (objetivo `fila % 97`, origen `fila % 13`, tipo `fila % 3`).
fn repeated_key_event(row: u32, emission: u32) -> TestEvent {
    TestEvent {
        target: u64::from(row % 97),
        source: u64::from(row % 13),
        kind: row % 3,
        row,
        emission,
    }
}

/// La fase principal de CA-8: cada fila emite `0..emissions_of(fila)` eventos, salvo las del
/// chunk silencioso, que no emiten ninguno.
fn repeated_key_phase(chunk: ChunkId, rows: &[u32], scratch: &mut EventScratch) {
    if chunk == SILENT_CHUNK {
        return;
    }
    for &row in rows {
        for emission in 0..emissions_of(row) {
            scratch.events.push(repeated_key_event(row, emission));
        }
    }
}

/// La referencia en serie de la fase principal: los eventos en orden de chunk y de emisión
/// (la concatenación de los chunks es el orden de fila), ordenados de forma estable por clave.
fn expected_repeated_key_events(len: usize) -> Vec<TestEvent> {
    let mut events = Vec::new();
    for row in row_indices(len) {
        if row / u32_of(CHUNK) == SILENT_CHUNK.0 {
            continue;
        }
        for emission in 0..emissions_of(row) {
            events.push(repeated_key_event(row, emission));
        }
    }
    // `sort_by_key` de la biblioteca estándar es estable: los empates conservan el orden de
    // fila y de emisión, que es el orden de chunk y de emisión.
    events.sort_by_key(EventKey::key);
    events
}

/// Comprueba en los propios eventos el orden que exige CA-8: claves no decrecientes y, dentro
/// de cada clave, fila y emisión crecientes.
fn assert_sorted_by_key_then_emission(label: &str, events: &[TestEvent]) {
    for (index, pair) in events.windows(2).enumerate() {
        let (a, b) = (pair[0], pair[1]);
        assert!(
            a.key() <= b.key(),
            "{label}: clave fuera de orden en {index}: {a:?} > {b:?}"
        );
        if a.key() == b.key() {
            assert!(
                (a.row, a.emission) < (b.row, b.emission),
                "{label}: misma clave fuera del orden de emisión en {index}: {a:?}, {b:?}"
            );
        }
    }
}

/// La fase de "todos con la misma clave": dos eventos por fila, con la clave `SAME_KEY`.
fn same_key_phase(_chunk: ChunkId, rows: &[u32], scratch: &mut EventScratch) {
    for &row in rows {
        for emission in 0..2 {
            scratch.events.push(TestEvent {
                target: SAME_KEY.0,
                source: SAME_KEY.1,
                kind: SAME_KEY.2,
                row,
                emission,
            });
        }
    }
}

/// Los eventos recogidos, en el orden final, como `Vec`.
fn collected(sorted: &SortedEvents<TestEvent>) -> Vec<TestEvent> {
    sorted.iter().copied().collect()
}

// CA-8: Con 10.000 filas y una fase que emite por cada fila entre 0 y 3 eventos de claves
// repetidas (objetivo `fila % 97`, origen `fila % 13`, tipo `fila % 3`), con un chunk sin
// eventos en medio: `SortedEvents::collect` deja los eventos ordenados por
// `(objetivo, origen, tipo)` y, dentro de cada clave, en orden de chunk y de emisión; la
// secuencia es idéntica con pools de 1, 3, 6 y 12 hilos; con todos los eventos de la misma
// clave, el orden final es exactamente el de chunk y emisión; con capacidad exactamente igual
// al total, `collect` funciona; `collect` tras un calentamiento no asigna; sin eventos deja
// `len() == 0`; con capacidad menor que el total hace pánico `eventos llenos`; un `FixedVec`
// de eventos de un chunk que se llena hace pánico `columna llena`. Lo demuestran: tests
// `ca_08_eventos_en_orden_de_chunk_y_clave` y `ca_08_eventos_capacidad_panica`.
#[test]
fn ca_08_eventos_en_orden_de_chunk_y_clave() {
    let rows = row_indices(ROWS);
    let expected = expected_repeated_key_events(ROWS);
    let total = expected.len();
    assert!(total > 0, "la fase de referencia no emite nada");
    assert_sorted_by_key_then_emission("referencia", &expected);
    // Hay claves repetidas de verdad: menos claves distintas que eventos.
    let mut keys: Vec<(u64, u64, u32)> = expected.iter().map(EventKey::key).collect();
    keys.dedup();
    assert!(keys.len() < total, "la fase de referencia no repite claves");

    let mut first: Option<Vec<TestEvent>> = None;
    for &threads in &THREADS {
        let label = format!("{threads} hilos");
        let pool = pool(threads);
        let mut scratch = event_scratch(ROWS, MAX_EVENTS_PER_CHUNK);
        par_scan(&pool, &rows, &mut scratch, repeated_key_phase);
        assert!(
            scratch.get(SILENT_CHUNK).events.is_empty(),
            "{label}: el chunk silencioso emitió"
        );
        assert!(!scratch.get(ChunkId(SILENT_CHUNK.0 - 1)).events.is_empty());
        assert!(!scratch.get(ChunkId(SILENT_CHUNK.0 + 1)).events.is_empty());

        // Capacidad exactamente igual al total: `collect` funciona y deja el orden esperado.
        let mut sorted = SortedEvents::with_capacity(total);
        assert_eq!(sorted.len(), 0);
        assert!(sorted.is_empty());
        sorted.collect(&scratch, events_of);
        assert_eq!(sorted.len(), total, "{label}: número de eventos");
        assert!(!sorted.is_empty());
        let got = collected(&sorted);
        assert_sorted_by_key_then_emission(&label, &got);
        assert_eq!(
            got, expected,
            "{label}: la secuencia no es la de la referencia en serie"
        );
        match &first {
            None => first = Some(got),
            Some(first) => assert_eq!(&got, first, "{label}: secuencia distinta que con 1 hilo"),
        }

        // Tras un calentamiento, `collect` no asigna (regla 19); vacía antes de concatenar.
        let info = allocation_counter::measure(|| sorted.collect(&scratch, events_of));
        assert_eq!(
            info.count_total, 0,
            "{label}: collect asignó {} veces ({} bytes) tras calentar",
            info.count_total, info.bytes_total
        );
        assert_eq!(
            sorted.len(),
            total,
            "{label}: collect repetido acumuló o perdió eventos"
        );
        assert_eq!(collected(&sorted), expected);

        // `clear` vacía sin perder la capacidad: el siguiente `collect` sigue sin asignar.
        sorted.clear();
        assert_eq!(sorted.len(), 0);
        assert!(sorted.is_empty());
        assert_eq!(sorted.iter().count(), 0);
        let info = allocation_counter::measure(|| sorted.collect(&scratch, events_of));
        assert_eq!(info.count_total, 0, "{label}: collect tras clear asignó");
        assert_eq!(collected(&sorted), expected);

        // El clon conserva la capacidad: un clon de uno vacío con sitio para `total` recoge
        // `total` eventos sin pánico, y el clon de uno lleno lleva los mismos eventos.
        let mut from_empty = SortedEvents::<TestEvent>::with_capacity(total).clone();
        from_empty.collect(&scratch, events_of);
        assert_eq!(collected(&from_empty), expected, "{label}: clon de vacío");
        let from_full = sorted.clone();
        assert_eq!(from_full.len(), total);
        assert_eq!(collected(&from_full), expected, "{label}: clon de lleno");

        // Todos los eventos de la misma clave: el orden final es exactamente el de chunk y
        // emisión, es decir, fila por fila y dentro de la fila por emisión.
        let mut same_scratch = event_scratch(ROWS, 2 * CHUNK);
        par_scan(&pool, &rows, &mut same_scratch, same_key_phase);
        let mut same_sorted = SortedEvents::with_capacity(2 * ROWS);
        same_sorted.collect(&same_scratch, events_of);
        assert_eq!(same_sorted.len(), 2 * ROWS);
        let same_expected: Vec<TestEvent> = rows
            .iter()
            .flat_map(|&row| {
                (0..2).map(move |emission| TestEvent {
                    target: SAME_KEY.0,
                    source: SAME_KEY.1,
                    kind: SAME_KEY.2,
                    row,
                    emission,
                })
            })
            .collect();
        assert_eq!(
            collected(&same_sorted),
            same_expected,
            "{label}: misma clave"
        );

        // Sin eventos: `len() == 0`, con chunks que no emiten y con un scratch sin chunks.
        let mut silent_scratch = event_scratch(ROWS, 1);
        par_scan(&pool, &rows, &mut silent_scratch, |_, _, _| {});
        let mut none = SortedEvents::<TestEvent>::with_capacity(total);
        none.collect(&silent_scratch, events_of);
        assert_eq!(none.len(), 0, "{label}: eventos de la nada");
        assert!(none.is_empty());
        assert_eq!(none.iter().count(), 0);
        let no_chunks = event_scratch(0, 1);
        assert!(no_chunks.is_empty());
        sorted.collect(&no_chunks, events_of);
        assert_eq!(sorted.len(), 0, "{label}: collect sin chunks no vació");
        assert!(sorted.is_empty());
    }
}

// CA-8 (pánico `eventos llenos`): `collect` con capacidad menor que el total. El test de la
// spec `ca_08_eventos_capacidad_panica` agrupa dos pánicos; aquí va partido con sufijos, uno
// por `should_panic`.
#[test]
#[should_panic(expected = "eventos llenos")]
fn ca_08_eventos_capacidad_panica_eventos_llenos() {
    let pool = pool(1);
    let rows = row_indices(ROWS);
    let total = expected_repeated_key_events(ROWS).len();
    let mut scratch = event_scratch(ROWS, MAX_EVENTS_PER_CHUNK);
    par_scan(&pool, &rows, &mut scratch, repeated_key_phase);
    let emitted: usize = scratch.iter().map(|(_, s)| s.events.len()).sum();
    assert_eq!(
        emitted, total,
        "la fase emitió otro total que la referencia"
    );
    let mut sorted = SortedEvents::with_capacity(total - 1);
    sorted.collect(&scratch, events_of);
}

// CA-8 (pánico `columna llena`): el `FixedVec` de eventos de un chunk con menos capacidad que
// los eventos que emite (100 frente a los 768 del chunk 0) hace pánico al llenarse, dentro de
// la fase, y el pánico sale de `par_scan`.
#[test]
#[should_panic(expected = "columna llena")]
fn ca_08_eventos_capacidad_panica_columna_llena() {
    let pool = pool(1);
    let rows = row_indices(ROWS);
    let mut scratch = event_scratch(ROWS, SMALL_EVENT_CAPACITY);
    par_scan(&pool, &rows, &mut scratch, repeated_key_phase);
}

// ---------------------------------------------------------------------------------------
// CA-9: reducción en orden de chunk
// ---------------------------------------------------------------------------------------

/// Un contador entero (regla 9): `Default` es 0 y `accumulate` suma.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Count(u64);

impl Accumulate for Count {
    fn accumulate(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

/// Registra el orden de las sumas: cada `accumulate` añade los valores que llegan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Trace(Vec<u64>);

impl Accumulate for Trace {
    fn accumulate(&mut self, other: &Self) {
        self.0.extend_from_slice(&other.0);
    }
}

/// Un acumulador cuyo `Default` no es cero, para distinguir "`A::default()`" de "cero".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Based(u64);

/// El valor inicial de `Based`.
const BASE: u64 = 1_000;

impl Default for Based {
    fn default() -> Based {
        Based(BASE)
    }
}

impl Accumulate for Based {
    fn accumulate(&mut self, other: &Self) {
        self.0 += other.0;
    }
}

/// El scratch de CA-9: los tres acumuladores del chunk, que valen `chunk + 1`.
#[derive(Clone, Debug)]
struct CounterScratch {
    count: Count,
    trace: Trace,
    based: Based,
}

/// Un scratch de `chunks` chunks cuyos contadores valen `chunk + 1`.
fn counter_scratch(chunks: usize) -> ChunkScratch<CounterScratch> {
    let scratch = ChunkScratch::new(chunks * CHUNK, |chunk| {
        let value = u64::from(chunk.0) + 1;
        CounterScratch {
            count: Count(value),
            trace: Trace(vec![value]),
            based: Based(value),
        }
    });
    assert_eq!(scratch.len(), chunks);
    scratch
}

// CA-9: `reduce_in_order` sobre un scratch cuyos contadores valen `chunk + 1` devuelve la
// suma `n(n+1)/2` y, con un `Accumulate` que registra el orden de las sumas, los chunks
// llegan en orden ascendente; con cero chunks devuelve `A::default()`. Lo demuestra: test
// `ca_09_reduccion_en_orden_de_chunk`.
#[test]
fn ca_09_reduccion_en_orden_de_chunk() {
    // Con n chunks que valen `chunk + 1`: la suma es `n(n+1)/2`, el orden es ascendente y
    // cada chunk entra una sola vez; el acumulador parte de `A::default()`.
    for chunks in [1, 7, EXPECTED_CHUNKS] {
        let n = u64_of(chunks);
        let scratch = counter_scratch(chunks);
        let sum = reduce_in_order(&scratch, |s| &s.count);
        assert_eq!(sum, Count(n * (n + 1) / 2), "suma con {chunks} chunks");
        let trace = reduce_in_order(&scratch, |s| &s.trace);
        assert_eq!(
            trace.0,
            (1..=n).collect::<Vec<u64>>(),
            "orden con {chunks} chunks"
        );
        let based = reduce_in_order(&scratch, |s| &s.based);
        assert_eq!(
            based,
            Based(BASE + n * (n + 1) / 2),
            "base con {chunks} chunks"
        );
    }

    // Con cero chunks devuelve `A::default()`, sea cero, vacío o la base.
    let empty = counter_scratch(0);
    assert!(empty.is_empty());
    assert_eq!(reduce_in_order(&empty, |s| &s.count), Count::default());
    assert_eq!(reduce_in_order(&empty, |s| &s.trace), Trace::default());
    assert_eq!(reduce_in_order(&empty, |s| &s.based), Based(BASE));

    // Contadores producidos por una fase real: `par_scan` cuenta las filas de cada chunk y la
    // reducción da el total con cualquier pool, también con chunks de sobra, que valen 0.
    let rows = row_indices(ROWS);
    for &threads in &THREADS {
        let pool = pool(threads);
        for spare in [0, SPARE_CHUNKS] {
            let mut scratch = ChunkScratch::new((EXPECTED_CHUNKS + spare) * CHUNK, |_| Count(0));
            par_scan(&pool, &rows, &mut scratch, |_, rows, count| {
                count.0 += u64_of(rows.len());
            });
            let total = reduce_in_order(&scratch, |count| count);
            assert_eq!(
                total,
                Count(u64_of(ROWS)),
                "{threads} hilos, {spare} chunks de sobra"
            );
            let trace_scratch = ChunkScratch::new(ROWS, |chunk| Trace(vec![scratch.get(chunk).0]));
            let trace = reduce_in_order(&trace_scratch, |trace| trace);
            let mut expected = vec![u64_of(CHUNK); EXPECTED_CHUNKS - 1];
            expected.push(u64_of(LAST_CHUNK_LEN));
            assert_eq!(
                trace.0, expected,
                "{threads} hilos: longitudes por chunk en orden"
            );
        }
    }
}

# Crate `engine_sim`

- **Carpeta:** `engine/sim`
- **Responsabilidad:** la simulación determinista a tick fijo: tablas SoA con el handle común, fases paralelas por chunks de tamaño fijo, pipeline de eventos ordenados, cola de órdenes selladas por tick, bucle de simulación con hash de estado por tick y por fase, replay, guardado del estado canónico y grid espacial de orden estable. No conoce ni toca gráficos, y no sabe qué es un soldado: las tablas concretas, las fases y las órdenes las define `game_rules`.
- **Reglas de la constitución que le afectan:**
  - **1 y 2.** Depende solo de `engine_core`, `rayon`, `serde` y `thiserror`. Nada gráfico ni de ventana; lo vigila `check-arch`.
  - **3.** `#![forbid(unsafe_code)]` en `src/lib.rs`.
  - **4.** `Simulation::step` avanza exactamente un tick y no existe ninguna función que reciba segundos.
  - **5.** Las entidades viven en `Table<C>`: un `Column<T>` por campo, nunca un vector de structs. El único struct por entidad es el valor de inserción (`Columns::Row`), que se descompone en columnas al entrar.
  - **6.** Todo lo que influye en el resultado está en `World`, en la cola de órdenes y en el tick, y se guarda en `SaveBody`. El grid, los buffers de trabajo y cualquier lista que `game_rules` construya a partir de las tablas son derivados y se reconstruyen al cargar (`World::rebuild_derived`). La frontera activa de cada tabla (`active_len`) es canónica. Ningún valor de `SimConfig` influye en el resultado: una cola llena es un pánico, no una rama (decisión 17).
  - **7.** Sin estado global: cada fase es un map sobre entrada congelada con salida por fila; los únicos buffers compartidos son los `ChunkScratch`, uno por chunk. Ningún `static` en el crate (CA-19).
  - **9.** El grid espacial es la única forma de buscar vecinos y cuenta los candidatos examinados para los contadores de la regla 9; `mark_removed` y las consultas al directorio son de tiempo constante.
  - **10.** La navegación con cambios en caliente y pasabilidad por facción es del módulo `nav`, cuya API fija su spec en el hito 2.
  - **13.** Chunks de `CHUNK` filas fijas, eventos concatenados en orden de chunk y ordenados por (objetivo, origen, tipo, secuencia), reducciones solo secuenciales y en orden de chunk, aleatoriedad por `engine_core::rng`, y el mismo hash de estado con cualquier número de hilos (CA-12).
  - **14.** Todo corre sin gráficos: `Simulation` no necesita ventana ni GPU, y los tests del crate son el primer binario sin gráficos.
  - **15.** La IA y el jugador usan la misma cola: `TickContext::submit` desde dentro del tick y `Simulation::submit` desde fuera sellan igual y se aplican igual. El replay reproduce lo que hizo la IA porque la regenera (decisión 6).
  - **17.** Solo `Tick` y `Ticks`. Ni `Instant`, ni `SystemTime`, ni flotantes de tiempo (CA-19).
  - **18.** Replays y guardados van en el sobre de `engine_core` con el hash de datos, y `decode_*` rechaza otro hash (CA-13, CA-14).
  - **19.** Cero asignaciones en `step` tras el calentamiento, medido con un hilo (CA-18): tablas, colas, buffers de eventos, scratch y el registro de replay tienen capacidad fija y la agotan con un error, un pánico o, en el registro, dejando de grabar; nunca crecen en silencio. Con más hilos, lo que asigne `rayon` por dentro se mide en el hito 2.
  - **12.** Le corresponden las filas de tick, ticks por segundo y guardar partida de la tabla de presupuestos (ver "Presupuestos").
- **ADR relacionados:** 0002 (nada genérico sin uso en el juego), 0004 (tablas SoA con handle común, borrado diferido, conjunto activo), 0006 (tick a 20 Hz; latencia de orden de un tick), 0007 (sobre `postcard` para guardado y replay), 0008 (tests de aceptación aparte), 0009 (hashers y hash por bytes).
- **Estado:** aprobada

## Qué hace

Un módulo por responsabilidad, cada uno en su archivo bajo `src/`:

| Módulo | Archivo | Contenido |
| --- | --- | --- |
| `buffer` | `src/buffer.rs` | `FixedVec<T>`: vector de capacidad fija que nunca crece |
| `table` | `src/table.rs` | `Row`, `Column<T>`, el trait `Columns`, `Table<C>`: inserción, borrado diferido, indirección, conjunto activo, reordenación |
| `parallel` | `src/parallel.rs` | `CHUNK`, `ChunkId`, `Pool`, `ChunkScratch<S>`, `par_map`, `par_scan`, `reduce_in_order` |
| `events` | `src/events.rs` | `EventKey` y `SortedEvents<E>`: concatenación en orden de chunk y orden estable sin asignar |
| `orders` | `src/orders.rs` | `Sealed<O>` y `OrderQueue<O>`: la cola canónica de órdenes selladas por tick |
| `world` | `src/world.rs` | el trait `World` y `TickContext`: fases, hash por fase, órdenes del tick |
| `sim` | `src/sim.rs` | `Simulation<W>`, `SimConfig`, `TickReport`: el bucle de ticks, puntos de control, reproducción y carga |
| `replay` | `src/replay.rs` | `Replay<S, O>` y su codificación en el sobre de `engine_core` |
| `save` | `src/save.rs` | `SaveBody<W>`, la versión de formato compuesta y su codificación en el sobre |
| `grid` | `src/grid.rs` | `GridConfig`, `CellId`, `Grid`: grid espacial de orden estable, vecinos por radio, orden por celda |
| `desync` | `src/desync.rs` | `first_divergence`: comparación de hashes por fase |

Reservados, sin código hasta su spec: `snapshot` (anillo de cuatro buffers para el render; `specs/engine/snapshots.md`, hito 1), `thread` (hilo de simulación y canales con la app; misma spec, hito 1), `nav` (flow fields por sectores, portales, pasabilidad por facción; `specs/engine/navegacion.md`, hito 2) y `visibility` (grid de visibilidad por equipo, hito 3, si el autor confirma la recomendación sobre la niebla de guerra). Cada uno llega con una tarea de contrato.

`src/lib.rs` declara los módulos, lleva `#![forbid(unsafe_code)]` y `#![deny(missing_docs)]`, y no define nada más. Toda la documentación está en español; los identificadores, en inglés. Ningún comentario del crate escribe los nombres que prohíbe CA-19.

## Qué no hace nunca

- No define ninguna tabla concreta, fase, orden ni evento de juego. `Simulation<W>` es genérica sobre el `World` que `game_rules` implementa.
- No carga datos: ni RON ni archivos. Recibe el mundo ya construido y el hash de datos ya calculado por `engine_core::data`.
- No abre ventanas ni habla con la GPU. No depende de `engine_render`, `wgpu`, `winit` ni `egui` (regla 2).
- No mide tiempo real ni acelera nada: la velocidad de partida y la pausa, si existen (pregunta 10 del GDD), son del acumulador de la app, que llama a `step` tantas veces como toque.
- No asigna memoria dentro de `Simulation::step`.
- No usa `std::collections::HashMap` ni `HashSet` (lo impide `clippy.toml`), ni reducciones paralelas, ni `par_bridge`, ni pregunta a `rayon` cuántos hilos hay en la lógica.
- No crea hilos salvo los del `Pool` de `rayon` que se le pide, y no tiene `static` alguno.
- No toma decisiones de juego: una orden que no cabe, una tabla llena o una permutación inválida son errores o pánicos que el llamante decide cómo evitar con sus topes de datos.
- No graba en el replay las órdenes que nacen dentro del tick: la IA es determinista y se regenera al reproducir (decisión 6).

## API pública

Es el contrato. `autor-tests` escribe contra estas firmas y los implementadores no las cambian salvo en una tarea de tipo `contrato`. Todo lo que no esté aquí es privado, o lleva `#[doc(hidden)]` y no es contrato. Las derivaciones listadas forman parte del contrato.

Los pánicos citados son comprobaciones explícitas con mensaje fijo, iguales en dev y en release: `columna llena`, `índice fuera del buffer`, `fila fuera de la tabla`, `capacidad de tabla excesiva`, `columnas desalineadas`, `permutación inválida`, `longitudes distintas`, `scratch insuficiente`, `demasiadas fases`, `fase mal anidada`, `orden del pasado`, `cola de órdenes llena`, `eventos llenos`, `configuración de grid inválida` y, heredado de `engine_core`, `periodo cero`. Los tests los usan en `#[should_panic(expected = ...)]`.

`Clone` conserva la capacidad en todos los tipos con capacidad fija de este crate (`FixedVec`, `Column`, `Table`, `OrderQueue`, `SortedEvents`, `ChunkScratch`): son implementaciones propias, porque un `Vec` clonado reserva solo su longitud y el clon haría pánico `columna llena` al primer `push`.

### `engine_sim::buffer`

```rust
use core::ops::{Index, IndexMut};
use serde::{Deserialize, Serialize};

/// Vector de capacidad fija. Reserva una vez y nunca crece: `push` con `len() >= capacity()` hace pánico `columna llena`.
/// Es la única forma de buffer en el camino caliente (regla 19). Se serializa como una secuencia, igual que un `Vec`
/// (`#[serde(transparent)]` sobre los elementos); al deserializar, la capacidad queda igual a la longitud hasta que
/// alguien llame a `reserve`. La igualdad compara solo los elementos: la capacidad no es canónica y no entra en `==`,
/// en `serde` ni en `HashState` (fijado en P02-T01).
#[derive(Debug, Serialize)]
pub struct FixedVec<T> { /* privado */ }
impl<T: Clone> Clone for FixedVec<T> {}   // conserva la capacidad
/// Implementaciones propias: comparan `as_slice()`, sin la capacidad.
impl<T: PartialEq> PartialEq for FixedVec<T> {}
impl<T: Eq> Eq for FixedVec<T> {}
/// Implementación propia: deserializa la secuencia y deja `capacity() == len()`, nunca 0 (regla 19: un buffer
/// deserializado no puede crecer en silencio al primer `push`).
impl<'de, T: Deserialize<'de>> Deserialize<'de> for FixedVec<T> {}

impl<T> FixedVec<T> {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> FixedVec<T>;   // asigna
    /// Amplía la capacidad hasta `capacity` si es menor. Asigna. Solo tras deserializar o al construir.
    pub fn reserve(&mut self, capacity: usize);
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn capacity(&self) -> usize;
    #[must_use]
    pub fn as_slice(&self) -> &[T];
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T];
    pub fn push(&mut self, value: T);
    /// `Err(value)` si estaba lleno, sin pánico, con el valor intacto, para quien prefiera decidir.
    pub fn try_push(&mut self, value: T) -> Result<(), T>;
    pub fn pop(&mut self) -> Option<T>;
    pub fn swap(&mut self, a: usize, b: usize);   // pánico `índice fuera del buffer` si alguno ≥ len
    pub fn clear(&mut self);
    pub fn truncate(&mut self, len: usize);
}
impl<T> Index<usize> for FixedVec<T> { type Output = T; }
impl<T> IndexMut<usize> for FixedVec<T> {}
impl<T: engine_core::hash::HashState> engine_core::hash::HashState for FixedVec<T> {}   // como `[T]`
```

### `engine_sim::table`

```rust
use core::num::NonZeroU32;
use core::ops::{Index, IndexMut};
use engine_core::handle::Handle;
use engine_core::hash::HashState;
use serde::{Deserialize, Serialize};
use crate::buffer::FixedVec;

/// Fila densa de una tabla. Cambia con borrados, activaciones y reordenaciones; nunca se guarda de un tick a otro.
/// Lo estable es el `Handle`; `Handle::slot()` indexa el directorio de la tabla, no la fila.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Row(pub u32);

impl Row {
    #[must_use]
    pub const fn index(self) -> usize;
}

/// Una columna SoA: un `FixedVec<T>` indexado por `Row`. Se serializa igual que el `FixedVec` (`#[serde(transparent)]`).
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Column<T> { /* privado: FixedVec<T> */ }
impl<T: Clone> Clone for Column<T> {}   // conserva la capacidad

impl<T> Column<T> {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Column<T>;
    pub fn reserve(&mut self, capacity: usize);
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn capacity(&self) -> usize;
    #[must_use]
    pub fn as_slice(&self) -> &[T];
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T];
    pub fn push(&mut self, value: T);          // pánico `columna llena`
    pub fn swap(&mut self, a: Row, b: Row);    // pánico `fila fuera de la tabla`
    pub fn pop(&mut self) -> Option<T>;
    pub fn clear(&mut self);
}
impl<T> Index<Row> for Column<T> { type Output = T; }
impl<T> IndexMut<Row> for Column<T> {}
impl<T: HashState> HashState for Column<T> {}   // como `[T]`: longitud y elementos, con la vía en bloque de `T`

/// Las columnas de una tabla concreta. Lo implementa `game_rules` a mano, una línea por columna y método
/// (ADR-0004: sin macro antes del hito 3). Toda operación estructural pasa por aquí, así ninguna columna se desalinea.
/// `len` devuelve la longitud de las columnas, que deben coincidir entre sí: la tabla solo compara ese valor con el suyo,
/// así que una desalineación entre dos columnas la detecta `len` si `game_rules` las compara (recomendado,
/// con `debug_assert_eq!`), y la tabla no.
pub trait Columns: Sized {
    /// Los valores de una entidad al insertarla. Es el único struct por entidad que existe.
    type Row;
    fn with_capacity(capacity: usize) -> Self;
    fn reserve(&mut self, capacity: usize);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn push(&mut self, row: Self::Row);
    fn swap(&mut self, a: Row, b: Row);
    fn pop(&mut self);
    fn clear(&mut self);
}

/// Tabla densa con directorio: filas `0..len()`, las activas delante (`0..active_len()`), las inactivas detrás.
/// Garantiza: `handle_at(row_of(h)) == h` para todo handle vivo; un handle muerto no vuelve a estar vivo (la generación
/// del directorio sube al liberar el slot, `Handle::next_generation`), salvo tras 2^32 - 1 reutilizaciones del mismo slot;
/// los borrados se aplican solo en `apply_removals`, por fila descendente; ninguna operación asigna tras
/// `with_capacity`, e `insert` falla con `TableFull` en vez de crecer. Tras cada operación estructural comprueba
/// `C::len() == len()` y, si no cuadra porque alguien tocó las columnas por `columns_mut`, hace pánico `columnas desalineadas`.
#[derive(Serialize)]
pub struct Table<C: Columns> { /* privado */ }
impl<C: Columns + Clone> Clone for Table<C> {}   // conserva la capacidad
/// Implementación propia: valida los invariantes y vuelve a reservar la capacidad (ver "Datos y formatos").
impl<'de, C: Columns + Deserialize<'de>> Deserialize<'de> for Table<C> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("tabla llena: capacidad {capacity}")]
pub struct TableFull { pub capacity: usize }

impl<C: Columns> Table<C> {
    /// Exige `capacity < u32::MAX`; si no, pánico `capacidad de tabla excesiva`. Asigna una vez: directorio de `capacity`
    /// slots, pila de libres con todos los slots (el 0 sale primero), handles, pendientes, bandera de pendiente por slot
    /// y scratch de permutación, y `C::with_capacity`.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Table<C>;
    #[must_use]
    pub fn capacity(&self) -> usize;
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn active_len(&self) -> usize;
    /// Entra activa, en la fila `active_len()` (la inactiva que estaba ahí pasa al final). El slot del directorio sale de la
    /// pila de libres: el último liberado primero y, al principio, 0, 1, 2… La generación es la que tenga ese slot.
    pub fn insert(&mut self, row: C::Row) -> Result<Handle, TableFull>;
    #[must_use]
    pub fn is_alive(&self, handle: Handle) -> bool;
    #[must_use]
    pub fn row_of(&self, handle: Handle) -> Option<Row>;
    /// Pánico `fila fuera de la tabla` si `row.index() >= len()`.
    #[must_use]
    pub fn handle_at(&self, row: Row) -> Handle;
    /// La columna densa fila → handle, en orden de fila.
    #[must_use]
    pub fn handles(&self) -> &[Handle];
    #[must_use]
    pub fn is_active(&self, handle: Handle) -> bool;
    /// Mueve la fila a la frontera del conjunto activo por intercambio con la fila `active_len() - 1` (al desactivar) o
    /// `active_len()` (al activar). Devuelve `false` sin cambiar nada si el handle está muerto o ya estaba así.
    pub fn set_active(&mut self, handle: Handle, active: bool) -> bool;
    /// Apunta el borrado para `apply_removals`, en tiempo constante (bandera por slot). `false` si el handle está muerto
    /// o ya estaba apuntado. No asigna.
    pub fn mark_removed(&mut self, handle: Handle) -> bool;
    #[must_use]
    pub fn pending_removals(&self) -> &[Handle];
    /// Aplica los borrados pendientes así: toma la fila de cada pendiente al empezar, y las recorre de mayor a menor (el
    /// "slot descendente" de ADR-0004, que aquí es la fila densa); para cada fila `r`, si `r < active_len()` intercambia `r`
    /// con `active_len() - 1` y baja `active_len`, y después intercambia la fila en la que ha quedado con `len() - 1` y hace
    /// `pop`. Cada slot liberado sube de generación y entra en la pila de libres, así que el último borrado es el primero
    /// que se reutiliza. Vacía `removed` y deja en él los handles borrados en ese orden; `removed` tiene que tener
    /// capacidad para `pending_removals().len()`, si no, pánico `columna llena`. Después, `pending_removals()` está vacío.
    pub fn apply_removals(&mut self, removed: &mut FixedVec<Handle>);
    /// Reordena las filas activas: la fila `i` pasa a tener lo que tenía `order[i]`. Exige que `order` sea una permutación
    /// de `0..active_len()`; si no, pánico `permutación inválida`. Mantiene el directorio. No asigna.
    pub fn permute_active(&mut self, order: &[Row]);
    #[must_use]
    pub fn columns(&self) -> &C;
    /// Solo para leer y escribir valores. Las operaciones estructurales de `Columns` las hace la tabla; hacerlas aquí
    /// acaba en pánico `columnas desalineadas` en la siguiente operación.
    #[must_use]
    pub fn columns_mut(&mut self) -> &mut C;
    /// Borra todas las entidades, por fila descendente: las generaciones de sus slots suben y los slots vuelven a la pila
    /// en ese orden, así que el slot de la fila 0 es el primero que se reutiliza. Nada vuelve a estar vivo.
    pub fn clear(&mut self);
}

impl<C: Columns + HashState> HashState for Table<C> {}
```

Secuencia de bytes de `Table<C>` en el hash de estado, en este orden: `len` y `active_len` como `u64`; `handles()` como `[Handle]`; el directorio como dos slices de `capacity` entradas, fila por slot (`u32`, `u32::MAX` si libre) y generación por slot (`u32`); la pila de slots libres (`[u32]`, del fondo a la cima); los borrados pendientes (`[Handle]`); y después `C::hash_state`. Cada slice entra como `[T]` de `engine_core::hash`: su longitud como `u64` y después los elementos por la vía en bloque de `T` (fijado en P02-T01). La capacidad entra de forma indirecta, por la longitud del directorio y de la pila: dos tablas con el mismo contenido y distinta capacidad dan hashes distintos. El scratch de permutación y la bandera de pendiente por slot no entran.

### `engine_sim::parallel`

```rust
use crate::table::Row;

/// Filas por chunk. Fijo, para que el número de chunks no dependa de los hilos (regla 13).
pub const CHUNK: usize = 512;

#[must_use]
pub const fn chunk_count(len: usize) -> usize;   // `len.div_ceil(CHUNK)`; 0 para 0

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ChunkId(pub u32);

impl ChunkId {
    /// `Row(self.0 * CHUNK)`: la primera fila del chunk, para leer otras columnas por fila.
    #[must_use]
    pub const fn first_row(self) -> Row;
}

/// El pool de `rayon` de la simulación, compartido por clones (`Arc`). Lo crea quien arranca la simulación con los hilos
/// que decida (núcleos físicos menos uno en el juego; 1, 3, 6 y 12 en los tests). `new` guarda el número de hilos y
/// `threads` lo devuelve: nada en el crate pregunta a `rayon` cuántos hilos hay.
#[derive(Clone)]
pub struct Pool { /* privado: Arc<rayon::ThreadPool>, threads */ }

#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("el pool necesita al menos un hilo")]
    ZeroThreads,
    #[error("no se pudo crear el pool: {0}")]
    Build(String),
}

impl Pool {
    /// Asigna. `threads == 0` da `ZeroThreads`.
    pub fn new(threads: usize) -> Result<Pool, PoolError>;
    #[must_use]
    pub fn threads(&self) -> usize;
    /// Ejecuta `f` dentro del pool; si ya se está dentro de él, en línea en el hilo actual. Desde un hilo ajeno cuesta
    /// despertar a un trabajador, por eso `Simulation::step` entra una sola vez por tick. Es la única puerta a `rayon`;
    /// `par_map` y `par_scan` la usan.
    pub fn install<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R;
}

/// Un valor por chunk, en orden de chunk. Es la "arena por hilo" de la regla 19, pero por chunk, para que el reparto no
/// dependa del número de hilos. Aquí viven los buffers de eventos, los vecinos y los contadores de cada fase.
pub struct ChunkScratch<S> { /* privado: Vec<S> */ }
impl<S: Clone> Clone for ChunkScratch<S> {}

impl<S> ChunkScratch<S> {
    /// `chunk_count(max_len)` valores, construidos en orden con `make`. Asigna.
    pub fn new(max_len: usize, make: impl FnMut(ChunkId) -> S) -> ChunkScratch<S>;
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn get(&self, chunk: ChunkId) -> &S;
    #[must_use]
    pub fn get_mut(&mut self, chunk: ChunkId) -> &mut S;
    pub fn iter(&self) -> impl Iterator<Item = (ChunkId, &S)>;              // orden de chunk
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ChunkId, &mut S)>;  // orden de chunk
}

/// Fase paralela: map sobre entrada congelada, salida por fila. El chunk `i` recibe las filas
/// `i * CHUNK .. min((i + 1) * CHUNK, len)` de `input` y de `output`, y su `scratch`. El reparto no depende del número
/// de hilos; el orden de ejecución sí, y por eso `f` solo escribe en su trozo de `output` y en su `scratch`, y lee lo demás
/// (otras columnas, el grid) por referencia congelada. Exige `input.len() == output.len()` (pánico `longitudes distintas`)
/// y `scratch.len() >= chunk_count(input.len())` (pánico `scratch insuficiente`); con `len == 0` no llama a `f`. No asigna.
pub fn par_map<I, O, S, F>(pool: &Pool, input: &[I], output: &mut [O], scratch: &mut ChunkScratch<S>, f: F)
where
    I: Sync,
    O: Send,
    S: Send,
    F: Fn(ChunkId, &[I], &mut [O], &mut S) + Sync + Send;

/// Igual que `par_map` sin salida por fila: la fase solo produce eventos y contadores en su `scratch`.
pub fn par_scan<I, S, F>(pool: &Pool, input: &[I], scratch: &mut ChunkScratch<S>, f: F)
where
    I: Sync,
    S: Send,
    F: Fn(ChunkId, &[I], &mut S) + Sync + Send;

/// Lo que se puede sumar en orden de chunk: contadores enteros (regla 9) y cualquier reducción. Las reducciones
/// paralelas de flotantes están prohibidas (regla 13); esta es secuencial y en orden fijo.
pub trait Accumulate: Default {
    fn accumulate(&mut self, other: &Self);
}

/// `A::default()` más `get(scratch de cada chunk)` en orden de chunk.
#[must_use]
pub fn reduce_in_order<S, A: Accumulate>(scratch: &ChunkScratch<S>, get: impl Fn(&S) -> &A) -> A;
```

### `engine_sim::events`

```rust
use crate::parallel::ChunkScratch;

/// Clave de orden de un evento: (objetivo, origen, tipo). Objetivo y origen son `Handle::to_raw()`, o 0 si no hay;
/// el tipo lo numera quien define los eventos. Los desempates de juego (dos matan al mismo en el mismo tick) salen de
/// este orden y se fijan en la spec de combate.
pub trait EventKey {
    fn key(&self) -> (u64, u64, u32);
}

/// Los eventos de todos los chunks, concatenados en orden de chunk y ordenados por clave; los empates conservan el orden
/// de chunk y de emisión. Sin asignar: `sort_unstable` con la secuencia de emisión como último criterio, porque el `sort`
/// estable de la biblioteca estándar reserva un buffer temporal.
pub struct SortedEvents<E> { /* privado: FixedVec<(E, u32)> */ }
impl<E: Clone> Clone for SortedEvents<E> {}   // conserva la capacidad

impl<E: EventKey + Clone> SortedEvents<E> {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> SortedEvents<E>;   // asigna
    /// Vacía, concatena los eventos de cada chunk en orden de chunk y ordena. Exige capacidad para todos; si no,
    /// pánico `eventos llenos`. No asigna.
    pub fn collect<S>(&mut self, scratch: &ChunkScratch<S>, events_of: impl Fn(&S) -> &[E]);
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = &E>;   // en el orden final
    pub fn clear(&mut self);
}
```

Los buffers de eventos por chunk son `FixedVec<E>` dentro del scratch de cada chunk; su capacidad la fija quien define la fase (un evento por fila y tipo, como máximo), y agotarla es pánico `columna llena`, no un `Vec` que crece.

### `engine_sim::orders`

```rust
use engine_core::hash::HashState;
use engine_core::time::Tick;
use serde::{Deserialize, Serialize};
use crate::buffer::FixedVec;

/// Una orden y el tick en que se aplica.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed<O> {
    pub tick: Tick,
    pub order: O,
}
impl<O: HashState> HashState for Sealed<O> {}   // tick y después la orden

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum QueueError {
    #[error("cola de órdenes llena: capacidad {capacity}")]
    Full { capacity: usize },
    #[error("orden sellada para el tick {tick:?}, anterior al último encolado {last:?}")]
    OutOfOrder { tick: Tick, last: Tick },
    #[error("la simulación está reproduciendo un replay y no admite órdenes externas")]
    Replaying,
}

/// La cola canónica de órdenes selladas (regla 6): las pendientes se guardan (`SaveBody::pending`) y se reproducen.
/// Orden de aplicación: por tick sellado y, dentro del tick, por orden de entrada. Capacidad fija. No se serializa ella
/// misma: `SaveBody` lleva `pending()` y `load` la reconstruye con la capacidad de `SimConfig`.
pub struct OrderQueue<O> { /* privado */ }
impl<O: Clone> Clone for OrderQueue<O> {}   // conserva la capacidad

impl<O> OrderQueue<O> {
    #[must_use]
    pub fn with_capacity(capacity: usize) -> OrderQueue<O>;   // asigna
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn capacity(&self) -> usize;
    /// Comprueba primero el orden y después el sitio: `OutOfOrder` si `sealed.tick` es anterior al tick de la última orden
    /// encolada, y si no, `Full` si no cabe. No asigna.
    pub fn push(&mut self, sealed: Sealed<O>) -> Result<(), QueueError>;
    /// Vacía `out` y mueve a él, en orden de entrada, las órdenes selladas exactamente para `tick`. Si queda alguna sellada
    /// para un tick anterior, pánico `orden del pasado`: es un invariante roto. `out` necesita capacidad; si no, pánico
    /// `columna llena`.
    pub fn drain_for(&mut self, tick: Tick, out: &mut FixedVec<O>);
    /// Las pendientes, en orden de aplicación.
    #[must_use]
    pub fn pending(&self) -> &[Sealed<O>];
    pub fn clear(&mut self);
}
impl<O: HashState> HashState for OrderQueue<O> {}   // `pending()` como slice
```

### `engine_sim::world`

```rust
use engine_core::hash::{HashState, StateHash};
use engine_core::rng::Seed;
use engine_core::time::Tick;
use serde::{de::DeserializeOwned, Serialize};
use crate::parallel::Pool;

/// Lo que `game_rules` implementa. Un `World` es el estado canónico entero de una partida: tablas, grids derivados,
/// contadores de juego. `Clone` para la copia que serializa el hilo de entrada y salida.
pub trait World: Clone + Send + Serialize + DeserializeOwned + HashState + 'static {
    /// La orden del juego (API de órdenes, regla 15: la IA solo produce esto).
    type Order: Clone + Send + Serialize + DeserializeOwned + HashState + 'static;
    /// Lo que hace falta, junto con la semilla y los datos, para construir el mundo inicial: mapa, facciones, perfiles.
    /// Va en el replay y en el guardado. `engine_sim` no lo interpreta.
    type Scenario: Clone + Send + Serialize + DeserializeOwned + HashState + 'static;

    /// La versión de las structs canónicas del juego (`Order`, `Scenario` y todo lo que contiene `Self`). `game_rules`
    /// la sube con cada cambio; `engine_sim` la compone con la suya en el `format_version` del sobre (`save::format_version`).
    const FORMAT_VERSION: u16;

    /// Un tick entero (regla 4). Lee `ctx.orders()`, ejecuta sus fases en un orden fijo con `begin_phase`/`end_phase`,
    /// y aplica nacimientos y borrados al final, nunca a mitad de fase. Toda aleatoriedad sale de
    /// `engine_core::rng` con `ctx.seed()`, `ctx.tick()` y el handle (regla 13). Corre dentro de un único `Pool::install`
    /// por tick. Las zonas de perfilado las abre cada fase con `engine_core::span!("nombre")` dentro de un bloque;
    /// `begin_phase` no las abre.
    fn step(&mut self, ctx: &mut TickContext<'_, Self::Order>);

    /// Reconstruye todo lo derivado (grids, listas, cachés) a partir del canónico, tras cargar. Debe dejar el mundo
    /// indistinguible del que nunca se guardó (regla 6).
    fn rebuild_derived(&mut self, pool: &Pool);
}

/// Lo que una fase sabe del tick. Lo construye `Simulation::step`; `World::step` lo recibe.
pub struct TickContext<'a, O> { /* privado */ }

impl<'a, O> TickContext<'a, O> {
    /// El tick que se está calculando: `Simulation::tick().next()` durante `step`.
    #[must_use]
    pub fn tick(&self) -> Tick;
    #[must_use]
    pub fn seed(&self) -> Seed;
    #[must_use]
    pub fn pool(&self) -> &Pool;
    /// Las órdenes selladas para este tick, en orden de entrada.
    #[must_use]
    pub fn orders(&self) -> &[O];
    /// Encola `order` sellada para `tick().next()`. Es la misma API que usa el jugador desde fuera (regla 15). Si no cabe
    /// (`SimConfig::order_capacity`), pánico `cola de órdenes llena`: el resultado no puede depender de la capacidad
    /// (regla 6, decisión 17). Estas órdenes no se graban en el replay: al reproducir, el mundo las vuelve a emitir porque
    /// es determinista.
    pub fn submit(&mut self, order: O);
    /// Abre la fase: cuenta una fase más (pánico `demasiadas fases` si pasa de `SimConfig::max_phases`, con el buscador
    /// activo o no) y guarda el nombre. Dos `begin_phase` sin `end_phase` en medio es pánico `fase mal anidada`.
    pub fn begin_phase(&mut self, name: &'static str);
    /// Cierra la fase. Con el buscador de desincronizaciones activo (`SimConfig::phase_hashes`), hashea `state` con
    /// `hash_of` y guarda `PhaseHash { name, hash }`; si no, no hace nada más. Sin `begin_phase` abierto es pánico
    /// `fase mal anidada`. Se llama con `&*self` desde `World::step`.
    pub fn end_phase<H: HashState + ?Sized>(&mut self, state: &H);
    #[must_use]
    pub fn phase_hashes_enabled(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhaseHash {
    pub name: &'static str,
    pub hash: StateHash,
}
```

### `engine_sim::sim`

```rust
use engine_core::data::DataHash;
use engine_core::hash::StateHash;
use engine_core::rng::Seed;
use engine_core::time::{Tick, Ticks};
use crate::orders::{QueueError, Sealed};
use crate::parallel::Pool;
use crate::replay::{Replay, ReplayError};
use crate::save::SaveBody;
use crate::world::{PhaseHash, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SimConfig {
    /// Cada cuántos ticks se calcula y registra un hash de estado (punto de control del replay). `None`: solo cuando hay
    /// un hash esperado, y entonces no se registra. `Some(Ticks(0))` hace pánico `periodo cero` en `new` y en `load`.
    pub hash_every: Option<Ticks>,
    /// Buscador de desincronizaciones: hash tras cada fase. Cuesta un hash del mundo por fase; apagado en el juego.
    pub phase_hashes: bool,
    /// Capacidad de la cola de órdenes pendientes y del buffer de órdenes de un tick.
    pub order_capacity: usize,
    /// Fases como máximo por tick.
    pub max_phases: usize,
    /// Órdenes externas que puede grabar el registro de replay. Al llenarse deja de grabar (ver `TickReport::replay_truncated`).
    pub replay_orders: usize,
    /// Puntos de control que puede grabar el registro de replay. Ídem.
    pub replay_checkpoints: usize,
}

/// Valores iniciales, a ajustar con las mediciones del hito 2: `hash_every: None`, `phase_hashes: false`,
/// `order_capacity: 1024`, `max_phases: 64`, `replay_orders: 65_536`, `replay_checkpoints: 16_384`.
impl Default for SimConfig {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub tick: Tick,
    pub expected: StateHash,
    pub found: StateHash,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickReport {
    /// El tick recién completado.
    pub tick: Tick,
    /// El hash de estado, si este tick era punto de control por `hash_every` o por hash esperado.
    pub checkpoint: Option<StateHash>,
    /// Si había un hash esperado para este tick y no coincide.
    pub divergence: Option<Divergence>,
    /// Cierto una sola vez: en el primer `step` después de que uno de los dos registros del replay (órdenes o puntos de
    /// control) se llenara; desde entonces ninguno de los dos graba.
    pub replay_truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("el hash del estado cargado ({found}) no es el guardado ({expected})")]
    HashMismatch { expected: StateHash, found: StateHash },
    #[error(transparent)]
    Queue(#[from] QueueError),
}

/// El bucle de simulación: posee el mundo, el tick, la semilla, el hash de datos, la cola de órdenes, un clon del pool y
/// el registro de replay. Avanza solo con `step`, de tick en tick (regla 4).
pub struct Simulation<W: World> { /* privado */ }

impl<W: World> Simulation<W> {
    /// Tick 0. Asigna: colas, buffers y registro, con las capacidades de `config`.
    pub fn new(scenario: W::Scenario, seed: Seed, data_hash: DataHash, world: W, pool: Pool, config: SimConfig) -> Simulation<W>;
    #[must_use]
    pub fn tick(&self) -> Tick;
    /// El tick en que empezó esta simulación: 0 si viene de `new`, el del guardado si viene de `load`.
    #[must_use]
    pub fn origin_tick(&self) -> Tick;
    #[must_use]
    pub fn seed(&self) -> Seed;
    #[must_use]
    pub fn data_hash(&self) -> DataHash;
    #[must_use]
    pub fn scenario(&self) -> &W::Scenario;
    #[must_use]
    pub fn world(&self) -> &W;
    /// Solo para montar el mundo antes del primer `step` o en tests. Tocarlo entre ticks de una partida grabada invalida el replay.
    #[must_use]
    pub fn world_mut(&mut self) -> &mut W;
    #[must_use]
    pub fn pool(&self) -> &Pool;
    #[must_use]
    pub fn config(&self) -> &SimConfig;
    /// Encola `order` sellada para `tick().next()` y, solo si cupo, la graba en el registro de replay (si el registro está
    /// lleno, la orden entra igual y el registro queda truncado). `Replaying` si hay un replay preparado: durante la
    /// reproducción las órdenes externas vienen del replay.
    pub fn submit(&mut self, order: W::Order) -> Result<(), QueueError>;
    #[must_use]
    pub fn pending_orders(&self) -> &[Sealed<W::Order>];
    /// Un tick, en este orden: si hay un replay preparado, encola sus órdenes selladas para `tick().next()` (si alguna no
    /// cabe, pánico `cola de órdenes llena`: la capacidad es menor que la de la partida grabada); saca de la cola las
    /// órdenes selladas para ese tick; entra una sola vez en el pool y llama a `World::step`; encola lo que la fase haya
    /// enviado con `submit`, sellado para el tick siguiente (si no cabe, pánico `cola de órdenes llena`); avanza el tick; y
    /// si toca punto de control, porque `hash_every` divide al tick o porque hay un hash esperado para él, calcula
    /// `state_hash`, lo registra si es de `hash_every` y lo compara con el esperado si lo hay.
    pub fn step(&mut self) -> TickReport;
    /// Hash de todo el canónico: tick, semilla, cola pendiente y mundo, en ese orden, con `StateHasher`.
    #[must_use]
    pub fn state_hash(&self) -> StateHash;
    /// Los hashes por fase del último `step`, vacío si el buscador está apagado.
    #[must_use]
    pub fn last_phase_hashes(&self) -> &[PhaseHash];
    /// El replay de lo jugado: escenario, semilla, órdenes externas selladas, puntos de control y hash final. Asigna (copia).
    /// Errores: `NotFromStart` si la simulación viene de `load`; `Reproducing` si tiene un replay preparado; `Truncated`
    /// si el registro se llenó; `PendingExternal` si hay órdenes externas selladas para `tick().next()` todavía sin
    /// aplicar, porque el `final_hash` no sería reproducible: hay que dar un `step` más o no haber enviado nada.
    pub fn replay(&self) -> Result<Replay<W::Scenario, W::Order>, ReplayError>;
    /// Prepara la reproducción: guarda una copia de las órdenes y fija los puntos de control del replay como esperados.
    /// No toca la cola: `step` encola cada orden en su tick. Exige tick 0, cola vacía, ningún replay preparado antes, la
    /// misma semilla y el mismo escenario (comparado por `hash_of`); si no, `WrongStart`. Asigna (copia).
    pub fn prepare_replay(&mut self, replay: &Replay<W::Scenario, W::Order>) -> Result<(), ReplayError>;
    /// Fija hashes esperados por tick, para comparar en `step`, sustituyendo los anteriores. Exige ticks estrictamente
    /// crecientes y posteriores a `tick()`; si no, `BadCheckpoints`. Asigna (copia).
    pub fn expect_checkpoints(&mut self, checkpoints: &[(Tick, StateHash)]) -> Result<(), ReplayError>;
    /// Copia del canónico para el hilo de entrada y salida: clona el mundo, la cola y el resto. Asigna. Entre ticks.
    #[must_use]
    pub fn save_body(&self) -> SaveBody<W>;
    /// Reconstruye una simulación desde un guardado: reconstruye la cola con `config.order_capacity` (`Queue(Full)` si lo
    /// pendiente no cabe), llama a `World::rebuild_derived`, recalcula el hash y lo compara con `body.hash` (regla 6).
    /// `origin_tick` es `body.tick`. Asigna.
    pub fn load(body: SaveBody<W>, data_hash: DataHash, pool: Pool, config: SimConfig) -> Result<Simulation<W>, LoadError>;
}
```

### `engine_sim::replay`

```rust
use engine_core::data::DataHash;
use engine_core::hash::StateHash;
use engine_core::rng::Seed;
use engine_core::save::SaveError;
use engine_core::time::Tick;
use serde::{Deserialize, Serialize};
use crate::orders::Sealed;
use crate::world::World;

/// La parte del motor de la versión del formato del replay. Sube con cada cambio de la struct `Replay`.
pub const REPLAY_FORMAT_VERSION: u16 = 1;

/// Semilla, escenario, órdenes externas selladas por tick, puntos de control y hash final: el formato de replay del hito 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay<S, O> {
    pub seed: Seed,
    pub scenario: S,
    /// Las órdenes que entraron por `Simulation::submit`, con el tick en que se aplicaron, en orden de aplicación.
    pub orders: Vec<Sealed<O>>,
    /// Hashes de estado en los ticks que `SimConfig::hash_every` marcó, en orden.
    pub checkpoints: Vec<(Tick, StateHash)>,
    pub last_tick: Tick,
    pub final_hash: StateHash,
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("la simulación no está en el tick 0 con la cola vacía, sin replay preparado, y con la semilla y el escenario del replay")]
    WrongStart,
    #[error("la simulación empezó en el tick {origin:?}, no en el 0: no hay replay desde un guardado")]
    NotFromStart { origin: Tick },
    #[error("la simulación está reproduciendo un replay")]
    Reproducing,
    #[error("hay órdenes externas sin aplicar: da un tick más antes de pedir el replay")]
    PendingExternal,
    /// `at` es el tick para el que se selló la primera orden que no cupo, o el tick del primer punto de control que no cupo.
    #[error("el registro de replay se llenó en el tick {at:?}")]
    Truncated { at: Tick },
    #[error("los puntos de control esperados no son estrictamente crecientes y posteriores al tick actual")]
    BadCheckpoints,
}

/// Sobre `Kind::Replay`, `format_version` = `save::format_version(REPLAY_FORMAT_VERSION, W::FORMAT_VERSION)`,
/// `game_version` = `GAME_VERSION`, y `data_hash`.
pub fn encode_replay<W: World>(replay: &Replay<W::Scenario, W::Order>, data_hash: DataHash, out: &mut Vec<u8>) -> Result<(), SaveError>;
/// Rechaza un hash de datos distinto (regla 18) y una versión de formato distinta, del motor o del juego.
pub fn decode_replay<W: World>(bytes: &[u8], data_hash: DataHash) -> Result<Replay<W::Scenario, W::Order>, SaveError>;
```

### `engine_sim::save`

```rust
use engine_core::data::DataHash;
use engine_core::hash::StateHash;
use engine_core::rng::Seed;
use engine_core::save::SaveError;
use engine_core::time::Tick;
use serde::{Deserialize, Serialize};
use crate::orders::Sealed;
use crate::world::World;

/// La parte del motor de la versión del formato del guardado. Sube con cada cambio de la struct `SaveBody`.
pub const SAVE_FORMAT_VERSION: u16 = 1;

/// `(engine << 16) | game`: lo que va en `Header::format_version` del sobre. El motor pone la parte alta y el juego la baja,
/// así cualquiera de los dos invalida los archivos antiguos sin tocar al otro.
#[must_use]
pub const fn format_version(engine: u16, game: u16) -> u32;

/// El estado canónico entero entre dos ticks.
#[derive(Clone, Serialize, Deserialize)]
pub struct SaveBody<W: World> {
    pub tick: Tick,
    pub seed: Seed,
    pub scenario: W::Scenario,
    pub world: W,
    /// La cola de órdenes pendientes, en orden de aplicación.
    pub pending: Vec<Sealed<W::Order>>,
    /// `Simulation::state_hash()` en el momento de guardar; `load` lo comprueba.
    pub hash: StateHash,
}

/// Sobre `Kind::Save`, `format_version` = `format_version(SAVE_FORMAT_VERSION, W::FORMAT_VERSION)`,
/// `game_version` = `GAME_VERSION`, y `data_hash`.
pub fn encode_save<W: World>(body: &SaveBody<W>, data_hash: DataHash, out: &mut Vec<u8>) -> Result<(), SaveError>;
/// Rechaza un hash de datos distinto (regla 18) y una versión de formato distinta, del motor o del juego.
pub fn decode_save<W: World>(bytes: &[u8], data_hash: DataHash) -> Result<SaveBody<W>, SaveError>;
```

### `engine_sim::grid`

```rust
use engine_core::math::Vec2;
use serde::{Deserialize, Serialize};
use crate::buffer::FixedVec;
use crate::table::Row;

/// Celdas como máximo: 2^20. `rebuild` recorre todas las celdas cada tick, así que el tamaño de celda de cada mapa se
/// elige con `benches/grid.rs` para que el coste quepa en el tick, también con 300 unidades; el grid de navegación del
/// hito 2 es otra estructura y su tamaño lo fija su spec tras la pregunta 16 del GDD.
pub const MAX_CELLS: u64 = 1 << 20;

/// Un grid uniforme sobre el rectángulo `origin .. origin + cell_size * (cells_x, cells_y)`. Lo fija el mapa (datos).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridConfig {
    pub origin: Vec2,     // finito
    pub cell_size: f32,   // finito y > 0
    pub cells_x: u32,     // ≥ 1
    pub cells_y: u32,     // ≥ 1; `cells_x as u64 * cells_y as u64 <= MAX_CELLS`, comprobado en u64
}

/// `y * cells_x + x`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CellId(pub u32);

/// Grid espacial de orden estable (celda, fila). Es estado derivado: se reconstruye cada tick desde las posiciones y no se
/// guarda. Con counting sort estable: `rebuild` es lineal en filas más celdas y no asigna.
pub struct Grid { /* privado */ }

impl Grid {
    /// Exige una `config` válida (pánico `configuración de grid inválida` si no). Asigna una vez para `max_rows` filas y
    /// todas las celdas.
    #[must_use]
    pub fn new(config: GridConfig, max_rows: usize) -> Grid;
    #[must_use]
    pub fn config(&self) -> &GridConfig;
    /// Recorta al borde: una coordenada fuera del rectángulo cae en la celda del borde más cercano (`-0.0` y los infinitos
    /// incluidos), y una coordenada NaN cae en la celda 0 de su eje. Determinista.
    #[must_use]
    pub fn cell_of(&self, position: Vec2) -> CellId;
    /// Indexa las filas `0..positions.len()`. Exige `positions.len() <= max_rows` (pánico `scratch insuficiente` si no). No asigna.
    pub fn rebuild(&mut self, positions: &[Vec2]);
    /// Filas indexadas en el último `rebuild`.
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    /// Las filas de la celda, en orden ascendente.
    #[must_use]
    pub fn rows_in_cell(&self, cell: CellId) -> &[Row];
    /// Los candidatos son las filas de las celdas del rectángulo de `cell_of(center - radius)` a `cell_of(center + radius)`,
    /// recortado al grid; se recorren en orden fijo (celdas por `y` y luego `x`, filas ascendentes dentro de cada celda) y
    /// se visitan las que están a distancia al cuadrado ≤ `radius * radius` de `center`. Con `radius` negativo o NaN no
    /// examina ninguna y devuelve 0. `positions` es el mismo slice que recibió `rebuild`. Devuelve el número de candidatos
    /// examinados, que es el contador de la regla 9. La propia fila del centro se visita si está a distancia 0: la salta quien
    /// llama. No asigna.
    pub fn for_each_in_radius(&self, center: Vec2, radius: f32, positions: &[Vec2], visit: impl FnMut(Row)) -> u32;
    /// Vacía `out` y escribe la permutación (celda, fila) de las filas indexadas, para `Table::permute_active`. Exige
    /// capacidad para `len()`; si no, pánico `columna llena`.
    pub fn cell_order(&self, out: &mut FixedVec<Row>);
}
```

### `engine_sim::desync`

```rust
use crate::world::PhaseHash;

/// Índice de la primera fase en la que `a` y `b` difieren en nombre o en hash, o `None` si son iguales; si una es prefijo
/// de la otra, el índice donde se acaba la corta. Es el núcleo del buscador de desincronizaciones: dos ejecuciones con
/// `phase_hashes` activo, comparadas tick a tick.
#[must_use]
pub fn first_divergence(a: &[PhaseHash], b: &[PhaseHash]) -> Option<usize>;
```

### Features del crate

| Feature | Por defecto | Efecto |
| --- | --- | --- |
| `tracy` | no | `engine_core/tracy`: `Simulation::step` abre la zona `tick` y llama a `tick_mark`; las fases abren las suyas con `span!` |

## Contratos compartidos que toca

| Contrato | Esta spec | Quién lo consume |
| --- | --- | --- |
| Handle, `Tick`, hash de estado, aleatoriedad, sobre | los consume de `engine_core`; no los cambia | |
| `World`, `Simulation`, `SimConfig`, `TickContext`, `Pool` | los crea: es el contrato principal con el juego y la app | `game_rules` (implementa `World`), `game_app` (posee la `Simulation` en el hilo de simulación), herramientas |
| Tablas SoA (`Table`, `Columns`, `Column`, `Row`, `FixedVec`) | los crea | `game_rules` (tablas concretas), `engine_render` (filas del snapshot, hito 1) |
| Fases y eventos (`par_map`, `par_scan`, `ChunkScratch`, `SortedEvents`, `EventKey`) | los crea | `game_rules` |
| API de órdenes (`Sealed`, `OrderQueue`, `TickContext::submit`, `Simulation::submit`) | crea el mecanismo; **no** el tipo de orden, que es `World::Order` | `game_rules` (define `Order`), `game_app` (jugador), IA |
| Formato de replay (`Replay`, `REPLAY_FORMAT_VERSION`) | lo crea | visor de replays (hito 1), partidas IA contra IA, CI |
| Formato de guardado (`SaveBody`, `SAVE_FORMAT_VERSION`, `format_version`) | crea el contenedor y la composición de versiones; **no** las structs canónicas del juego, que versiona `World::FORMAT_VERSION` | `game_rules`, `game_app` |
| Grid espacial | lo crea | `game_rules` (vecinos, combate, separación) |
| Layout del snapshot | **no lo toca**; módulo reservado | `specs/engine/snapshots.md`, hito 1 |
| Navegación | **no la toca**; módulo reservado | `specs/engine/navegacion.md`, hito 2 |
| `[workspace.dependencies]` | añade `rayon` y la entrada `engine_core` | el `Cargo.toml` raíz, en la tarea de contrato |
| Puerta adicional de este crate | la declara "Criterios de aceptación" | `/planificar` la copia en el plan y en cada tarea; `/verificar` la ejecuta |

## Dependencias permitidas

Crates del workspace: **`engine_core`**, declarado en `[workspace.dependencies]` como `engine_core = { path = "engine/core", version = "0.1.0" }` (con versión, para que `wildcards = "deny"` de `deny.toml` no lo tome por comodín) y en el crate como `engine_core.workspace = true`.

Externos, con la línea exacta que va en `[workspace.dependencies]` del `Cargo.toml` raíz. Versiones y licencias comprobadas en crates.io el 26 de octubre de 2026 con `cargo info` (`rayon` 1.12.0, licencia MIT o Apache-2.0, `rust-version` 1.80); la tarea de contrato las confirma con `cargo check --workspace --all-targets` y `cargo deny check`.

| Crate | Línea en `[workspace.dependencies]` | Para qué | Licencia |
| --- | --- | --- | --- |
| `rayon` | `rayon = "1.12"` | `Pool`, `par_map`, `par_scan`; es el único sitio del crate que nombra `rayon` | MIT o Apache-2.0 |
| `serde` | ya está (spec de `engine_core`) | derivaciones de `FixedVec`, `Table`, `Sealed`, `Replay`, `SaveBody`, `GridConfig` | MIT o Apache-2.0 |
| `thiserror` | ya está | errores de este crate | MIT o Apache-2.0 |

`glam` y `bytemuck` se usan por los reexports de `engine_core` (Q-008) y no se declaran; `rustc-hash`, `xxhash-rust` y `ron` no se usan; `check-arch` lo vigila (CA-23 de `engine_core`). `postcard` solo como dependencia de desarrollo.

De desarrollo (solo `[dev-dependencies]` de `engine/sim/Cargo.toml`, todas ya en `[workspace.dependencies]`):

| Crate | Para qué | Licencia |
| --- | --- | --- |
| `proptest` | tests de propiedades de `Table` y de la cola | MIT o Apache-2.0 |
| `criterion` | benchmarks del tick sintético, la tabla, el grid y el guardado | Apache-2.0 o MIT |
| `allocation-counter` | CA-18, regla 19; contador por hilo, por eso se mide dentro de `Pool::install` con un hilo | MIT o Apache-2.0 |
| `postcard` | comprobar la codificación de tablas, replays y guardados en los tests de formato | MIT o Apache-2.0 |

Los tests no usan `tempfile` ni `rand`: carpetas bajo `CARGO_TARGET_TMPDIR` y `xorshift64` de semilla fija, como en `engine_core`. Cualquier otro crate es una violación de las reglas 1 y 2 y una pregunta para el autor.

## Datos y formatos

Los vectores de bytes de esta sección se calcularon con `postcard` 1.1.3.

**`FixedVec<T>` y `Column<T>` en `serde`**: una secuencia con longitud, igual que un `Vec<T>`. Un `FixedVec<u8>` de capacidad 4 con `[7, 8]` codifica a `02 07 08`, y vuelve con capacidad 2.

**`Table<C>` en `serde`**, como una struct con estos campos en este orden, todos sin saltar salvo el scratch de permutación y la bandera de pendiente por slot, que no se serializan: `capacity` (`u32`), `dir_rows` (`Vec<u32>`, una entrada por slot, `u32::MAX` si libre), `dir_generations` (`Vec<u32>`), `free` (`Vec<u32>`, del fondo a la cima de la pila), `handles` (`Vec<Handle>`), `active_len` (`u32`), `pending` (`Vec<Handle>`) y `columns` (`C`). Al deserializar, `Table` valida, antes de indexar nada: `capacity < u32::MAX`; `dir_rows` y `dir_generations` de longitud `capacity`; ninguna generación 0; todo slot de `handles`, `free` y `pending` menor que `capacity`; `handles.len() <= capacity` e igual a `C::len()`; para cada fila `r`, `dir_rows[handles[r].slot()] == r` y `dir_generations[…] == handles[r].generation()`; `free.len() == capacity - handles.len()`, cada slot libre con `dir_rows == u32::MAX`, sin repetir y sin estar en `handles`; `active_len <= handles.len()`; pendientes vivos y sin repetir. Si algo falla, el deserializador devuelve error, que `engine_core::save::decode` convierte en `Corrupt`. Después reserva la capacidad en todas sus partes (`FixedVec::reserve`, `C::reserve`) y reconstruye la bandera de pendiente. Vector fijado: una tabla de capacidad 2 con una entidad en el slot 0, generación 1, activa, y una sola columna `Column<u8>` con el valor 7 (dentro de un struct `Columns` de un campo) codifica a los 20 bytes `02 02 00 FF FF FF FF 0F 02 01 01 01 01 01 00 01 01 00 01 07`.

**Replay**, en el sobre de `engine_core::save` con `Kind::Replay` y `format_version = format_version(REPLAY_FORMAT_VERSION, W::FORMAT_VERSION)`: `Replay<S, O>` en `postcard`, campos en el orden de la struct. Vector fijado: `Replay { seed: Seed(1), scenario: (), orders: [], checkpoints: [], last_tick: Tick(0), final_hash: StateHash(2) }` son los 5 bytes `01 00 00 00 02`; con una orden `Sealed { tick: Tick(3), order: 9u8 }`, un punto de control `(Tick(25), StateHash(16))` y `last_tick: Tick(30)`, los 9 bytes `01 01 03 09 01 19 10 1E 02`. Con `REPLAY_FORMAT_VERSION = 1` y `W::FORMAT_VERSION = 1`, el `format_version` del sobre es 65.537, que en la cabecera son los bytes `81 80 04`.

**Guardado**, en el sobre con `Kind::Save` y `format_version = format_version(SAVE_FORMAT_VERSION, W::FORMAT_VERSION)`: `SaveBody<W>` en `postcard`, campos en el orden de la struct; `W` incluye sus `Table<C>` con la codificación de arriba. Quien cambie una struct canónica del juego sube `W::FORMAT_VERSION`; quien cambie `Replay` o `SaveBody` sube la constante del motor. Las migraciones van por `decode_header` más la struct antigua, como en `engine_core`.

**Hash de estado de una simulación** (`Simulation::state_hash`), en este orden: `tick` (`u64`), `seed` (`u64`), la cola pendiente (longitud y `Sealed` en orden) y `W::hash_state`. Un `PhaseHash` es `hash_of(mundo)` justo después de la fase. Lo derivado no entra: un `Grid` no implementa `HashState`.

No lee ni escribe archivos.

## Presupuestos

De la tabla de presupuestos de la constitución, percentil 99:

| Métrica | Mínima | Referencia | Quién la mide |
| --- | --- | --- | --- |
| Tick en tiempo real con 15.000 unidades | ≤ 10 ms con 3 workers | ≤ 6 ms con 5 workers | hito 2, con la simulación real; aquí `benches/tick_sintetico.rs` lo reporta para el mundo de prueba |
| Sin gráficos con 15.000 unidades | ≥ 150 ticks/s | ≥ 400 ticks/s | ídem |
| Sin gráficos con 300 unidades | ≥ 5.000 ticks/s | ≥ 10.000 ticks/s | ídem; es el presupuesto que manda (GDD) y el que decide el tamaño de celda del grid |
| Pico de navegación | ≤ 2 ms por tick | igual | hito 2, spec de navegación |
| Snapshot de simulación a render | ~300 KB por tick | igual | hito 1, spec de snapshots |
| Guardar partida | parón ≤ 50 ms; archivo ≤ 20 MB | igual | el parón es `save_body`, clonar unos 3 MB; `benches/guardado.rs` reporta su tiempo y los bytes de `encode_save` |
| Asignaciones por tick tras el calentamiento, con un hilo | 0 | 0 | CA-18 |
| `Grid::rebuild` con 300 y con 15.000 filas, en un grid de `MAX_CELLS` celdas y en uno de 2^12 | sin umbral; se reporta | igual | `benches/grid.rs` |

Umbrales exigidos desde el hito 2 (regla 12); hasta entonces los benchmarks son informativos.

## Criterios de aceptación

Los tests van en `engine/sim/tests/aceptacion_sim_tNN.rs`, uno por tarea, y usan solo la API pública de `engine_sim` y `engine_core`. El **mundo de prueba** vive en `engine/sim/tests/common/mod.rs`, lo escribe `autor-tests` en la tarea 7 (la primera que lo necesita) y los benchmarks lo incluyen con `#[path = "../tests/common/mod.rs"]`; las tareas anteriores usan columnas y mundos sueltos definidos en su propio archivo de tests. Es una `Table` con columnas posición (`Vec2`), velocidad (`Vec2`), vida (`u32`) y objetivo (`Option<Handle>`), un `Grid`, y un `step` con cinco fases con nombre: `mover` (aplica cada `Empujar` de `ctx.orders()` sumando `(dx, dy)` a la velocidad del handle si sigue vivo; después `par_map`: posición más velocidad, rebotando en el borde), `vecinos` (`rebuild` del grid con las posiciones activas y `par_scan` con `for_each_in_radius`, que emite un evento `Golpe { objetivo, origen }` por cada vecino a menos de un radio y cuenta candidatos), `aplicar` (secuencial: `SortedEvents`, resta uno de vida al objetivo; si el escenario tiene `ia: true`, cada 5 ticks envía con `ctx.submit` una orden `Empujar` para la fila 0), `borrar` (marca las filas con vida 0 y `apply_removals`; cada 10 ticks inserta filas nuevas con aleatoriedad de `engine_core::rng`; cada 15 ticks desactiva la entidad de la fila 1 y la reactiva al tick siguiente) y `reordenar` (cada 20 ticks, `cell_order` y `permute_active`). Su `Order` es `Empujar { handle, dx, dy }`, su `Scenario` es `{ filas, mapa, ia }` y su `FORMAT_VERSION` es 1; un segundo tipo `MundoV2`, idéntico salvo `FORMAT_VERSION = 2`, sirve para los tests de versión; un tercero, `MundoVacio`, con `step` vacío, para los casos límite. Las órdenes de los tests se generan con `xorshift64`.

**Puerta adicional de este crate.** Además de las cinco comprobaciones de `CLAUDE.md`, toda tarea de `engine_sim` pasa `cargo test -p engine_sim --release`, `cargo clippy -p engine_sim --features tracy --all-targets -- -D warnings`, `cargo test -p engine_sim --features tracy`, `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_sim --no-deps` y `cargo tree -p engine_sim -e normal` (CA-21). `/planificar` la copia en el plan y `/verificar` la ejecuta.

**Comprobaciones de la constitución.** La tabla de estado de la constitución asigna las comprobaciones "hito 0" a `specs/engine/simulacion-sintetica.md`. Esta spec las demuestra a nivel de crate con el mundo de prueba: regla 4 (CA-11), 6 (CA-13), 7 y 17 (CA-19), 9 (CA-17), 13 (CA-12), 18 (CA-13, CA-14) y 19 (CA-18). La tabla pasa a "hecha" con los tests de la simulación sintética, que son los que la constitución nombra y los que corren a la escala del hito 0 (decisión 18).

- **CA-1.** `Table::with_capacity(8)` con columnas de prueba: tres `insert` devuelven handles con slots 0, 1 y 2 y generación 1, en las filas 0, 1 y 2, `len() == 3`, `active_len() == 3`, `row_of`, `handle_at` e `is_alive` coherentes, `handles()` igual a los tres en orden; cinco `insert` más llenan la tabla y el noveno devuelve `Err(TableFull { capacity: 8 })` sin cambiar nada; `allocation_counter::measure` cuenta 0 asignaciones en los ocho `insert`; `clone()` conserva `capacity() == 8`, los mismos `handles()` y valores, y admite `insert` hasta la capacidad sin pánico tras `clear()`; tras `clear()` de tres entidades, los tres handles están muertos y el siguiente `insert` reutiliza el slot de la antigua fila 0 con generación 2, el siguiente el de la fila 1; `with_capacity(u32::MAX as usize)` hace pánico `capacidad de tabla excesiva`; `handle_at(Row(3))` con tres filas hace pánico `fila fuera de la tabla`. Lo demuestran: tests `ca_01_tabla_insercion_llena_clon_y_clear`, `ca_01_tabla_capacidad_excesiva_panica` y `ca_01_handle_at_fuera_panica`.
- **CA-2.** Con cuatro entidades `a, b, c, d` en las filas 0 a 3: `mark_removed(b)` y `mark_removed(d)` devuelven `true`, un segundo `mark_removed(b)` devuelve `false`, `is_alive(b)` sigue cierto y `pending_removals() == [b, d]`; `apply_removals` deja `removed == [d, b]` (fila descendente), `len() == 2`, `row_of(a) == Some(Row(0))`, `row_of(c) == Some(Row(1))`, `row_of(b) == None`, `is_alive(d)` falso y `pending_removals()` vacío; las columnas tienen los valores de `a` y `c` en las filas 0 y 1; el siguiente `insert` reutiliza el slot de `b` (el último liberado) con generación `next_generation(b.generation())`, y el siguiente el de `d`; `is_alive(b)` e `is_alive(d)` siguen falsos aunque compartan slot con los nuevos; `apply_removals` sin pendientes no cambia nada; `mark_removed` de un handle muerto devuelve `false`; con la tabla llena, `mark_removed` seguido de `insert` da `TableFull` hasta `apply_removals`. Lo demuestra: test `ca_02_borrado_diferido_descendente_y_generaciones`.
- **CA-3.** Propiedad con `proptest`: 2.000 operaciones aleatorias de `insert`, `mark_removed`, `apply_removals`, `set_active`, `permute_active` (con permutaciones válidas) y `clear` sobre una tabla de capacidad 256, y una variante con capacidad 0 en la que todo `insert` da `TableFull` y el resto no hace nada, comparadas con un modelo de referencia (un `Vec` de entidades vivas con su valor): tras cada operación, `len()` y `active_len()` coinciden con el modelo, cada handle vivo del modelo resuelve a una fila cuyo valor en las columnas es el suyo, cada handle muerto da `row_of == None` e `is_alive == false`, `handles()` no repite handles, las filas `0..active_len()` son exactamente las activas del modelo, y la longitud de cada columna es `len()`. Lo demuestra: test `ca_03_tabla_invariantes_propiedad`.
- **CA-4.** Con cinco entidades activas: `set_active(c, false)` devuelve `true`, `active_len() == 4`, `c` queda en la fila 4 y la que estaba en la fila 4 pasa a la de `c`; `set_active(c, false)` otra vez devuelve `false`; `set_active(c, true)` devuelve `true` y `active_len() == 5`; `is_active` refleja cada paso; una entidad insertada con dos inactivas detrás entra en la fila `active_len()` y las inactivas siguen detrás; `set_active` de un handle muerto devuelve `false` sin cambiar `active_len()`; `set_active(x, false)` de una fila con borrado pendiente y después `apply_removals` la borra igual; y con `[a b c d | e f]` (`active_len` 4), `mark_removed(b)` y `apply_removals` dejan `handles() == [a, d, c, f, e]` y `active_len() == 3`. Lo demuestra: test `ca_04_conjunto_activo_como_prefijo`.
- **CA-5.** Con seis entidades, dos inactivas: `permute_active(&[Row(3), Row(0), Row(2), Row(1)])` deja en la fila `i` los valores que tenía `order[i]`, `handle_at` coherente con `row_of` para las seis, las dos inactivas intactas en las filas 4 y 5; `permute_active` con una fila repetida, con una fila ≥ `active_len()` o con longitud distinta de `active_len()` hace pánico `permutación inválida`; `permute_active(&[])` sobre una tabla sin activas no hace nada; `allocation_counter::measure` cuenta 0 asignaciones en una permutación válida de 4.096 filas; `Column::swap` con una fila ≥ `len` hace pánico `fila fuera de la tabla`; un `pop` directo en `columns_mut()` seguido de `insert` hace pánico `columnas desalineadas`. Lo demuestran: tests `ca_05_permutacion_de_activas`, `ca_05_permutacion_invalida_panica`, `ca_05_swap_fuera_de_rango_panica` y `ca_05_columnas_desalineadas_panica`.
- **CA-6.** La tabla mínima de "Datos y formatos" codifica con `postcard::to_allocvec` exactamente a los 20 bytes fijados y vuelve igual con `from_bytes`. Una tabla con 1.000 entidades, 100 inactivas y 50 borrados pendientes: ida y vuelta por `postcard` devuelve una tabla con el mismo `hash_of`, los mismos `handles()`, `active_len()` y `pending_removals()`, y los mismos valores de columnas; tras cargar, `insert` hasta la capacidad original no asigna y el siguiente da `TableFull`; `apply_removals` en las dos tablas da los mismos `removed`; fallan al deserializar, cada uno con su test, unos bytes con una columna de longitud distinta de `handles()`, con `capacity` menor que `len`, con una generación 0, con un slot libre repetido, con `free` de longitud distinta de `capacity - len`, con un slot `>= capacity` en `handles` y con `active_len` mayor que `len`, ninguno con pánico; y el `hash_of` de la tabla es igual a la secuencia de la sección `table` escrita a mano con un `StateHasher`, y distinto del de una tabla igual con capacidad 2.000. Lo demuestran: tests `ca_06_tabla_bytes_fijados`, `ca_06_tabla_serde_ida_y_vuelta_con_capacidad`, `ca_06_tabla_corrupta_no_deserializa` y `ca_06_tabla_hash_segun_la_secuencia`.
- **CA-7.** `chunk_count(0) == 0`, `chunk_count(1) == 1`, `chunk_count(512) == 1`, `chunk_count(513) == 2`; `ChunkId(3).first_row() == Row(1536)`; `Pool::new(0)` da `ZeroThreads` y `Pool::new(3).threads() == 3`; `par_map` sobre 10.000 filas registra en el scratch de cada chunk su `ChunkId` y la longitud de su trozo, que son `0..20` con 512 filas salvo el último con 272, con pools de 1, 3, 6 y 12 hilos; la salida es igual a la calculada en serie; con `len == 0` no llama a `f`; con un scratch de más chunks de los necesarios, los sobrantes quedan intactos; `par_map` con `input` y `output` de longitudes distintas hace pánico `longitudes distintas`, y con un scratch de menos chunks, `scratch insuficiente`; `par_scan` recibe los mismos chunks. Lo demuestran: tests `ca_07_chunks_fijos_con_cualquier_pool` y `ca_07_par_map_longitudes_y_scratch_panican`.
- **CA-8.** Con 10.000 filas y una fase que emite por cada fila entre 0 y 3 eventos de claves repetidas (objetivo `fila % 97`, origen `fila % 13`, tipo `fila % 3`), con un chunk sin eventos en medio: `SortedEvents::collect` deja los eventos ordenados por `(objetivo, origen, tipo)` y, dentro de cada clave, en orden de chunk y de emisión; la secuencia es idéntica con pools de 1, 3, 6 y 12 hilos; con todos los eventos de la misma clave, el orden final es exactamente el de chunk y emisión; con capacidad exactamente igual al total, `collect` funciona; `collect` tras un calentamiento no asigna; sin eventos deja `len() == 0`; con capacidad menor que el total hace pánico `eventos llenos`; un `FixedVec` de eventos de un chunk que se llena hace pánico `columna llena`. Lo demuestran: tests `ca_08_eventos_en_orden_de_chunk_y_clave` y `ca_08_eventos_capacidad_panica`.
- **CA-9.** `reduce_in_order` sobre un scratch cuyos contadores valen `chunk + 1` devuelve la suma `n(n+1)/2` y, con un `Accumulate` que registra el orden de las sumas, los chunks llegan en orden ascendente; con cero chunks devuelve `A::default()`. Lo demuestra: test `ca_09_reduccion_en_orden_de_chunk`.
- **CA-10.** `OrderQueue::with_capacity(4)`: `push` de órdenes selladas para los ticks 1, 1, 2 y 3 funciona; una quinta para el tick 3 da `Full { capacity: 4 }`; con la cola llena, una sellada para el tick 2 da `OutOfOrder { tick: 2, last: 3 }`, porque el orden se comprueba antes que el sitio; `drain_for(Tick(1))` deja en `out` las dos del tick 1 en orden de entrada y `pending()` las del 2 y el 3; cuatro órdenes del mismo tick salen juntas y en orden de entrada; `drain_for(Tick(0))` con la cola vacía deja `out` vacío; `drain_for(Tick(3))` con la del tick 2 todavía pendiente hace pánico `orden del pasado`; tras vaciar la cola, cuatro `push` más caben sin asignar; `clone()` conserva la capacidad; `hash_of` de la cola es igual a su secuencia escrita a mano. Lo demuestran: tests `ca_10_cola_de_ordenes_sellada_por_tick` y `ca_10_cola_orden_del_pasado_panica`.
- **CA-11.** Con el mundo de prueba (`ia: true`) y `SimConfig { hash_every: Some(Ticks(10)), phase_hashes: true, .. }`: `tick()` empieza en `Tick::ZERO` y cada `step` lo sube en uno; una orden enviada con `submit` antes de `step` aparece en `ctx.orders()` de ese `step` y en ningún otro, y `mover` la aplica a la velocidad; las órdenes que `aplicar` envía con `ctx.submit` en el tick 5 aparecen en `ctx.orders()` del tick 6, delante de las externas enviadas con `submit` entre los ticks 5 y 6, porque entraron antes en la cola; `pending_orders()` refleja ambas; `MundoVacio` avanza el tick igual y su `state_hash()` cambia solo por el tick; `state_hash()` es igual a `tick`, `seed`, cola pendiente y mundo escritos a mano con un `StateHasher`; cada `PhaseHash` es igual a `hash_of(world())` calculado por el propio test en una variante del mundo que no cambia nada después de la última fase; `TickReport::checkpoint` es `Some` exactamente en los ticks 10, 20 y 30 e igual a `state_hash()` en ese momento; `last_phase_hashes()` tiene cinco entradas con los nombres `mover`, `vecinos`, `aplicar`, `borrar` y `reordenar`, y está vacío con `phase_hashes: false`; el `state_hash()` por tick es el mismo con `phase_hashes` cierto o falso; `SimConfig::default()` tiene los valores de la API; con `max_phases: 2` el tercer `begin_phase` hace pánico `demasiadas fases`, también con `phase_hashes: false`; `Simulation::new` con `hash_every: Some(Ticks(0))` hace pánico `periodo cero`; con `hash_every: Some(Ticks(1))` todos los ticks son punto de control y con `None` ninguno; `world_mut` después del tick 0 funciona; un mundo que llama a `ctx.submit` con la cola llena (`order_capacity: 1` y dos envíos) hace pánico `cola de órdenes llena`; un mundo que llama a `end_phase` sin `begin_phase`, o a `begin_phase` dos veces seguidas, hace pánico `fase mal anidada`. Lo demuestran: tests `ca_11_bucle_de_ticks_ordenes_y_puntos_de_control`, `ca_11_demasiadas_fases_panica`, `ca_11_periodo_cero_panica`, `ca_11_ctx_submit_cola_llena_panica` y `ca_11_fase_mal_anidada_panica`.
- **CA-12.** Regla 13. El mundo de prueba (`ia: true`) con 4.000 filas, mapa de 200 por 200 y radio 2, 300 ticks, con una orden `Empujar` externa cada 7 ticks generada con `xorshift64`: la secuencia de `state_hash()` por tick es idéntica con pools de 1, 3, 6 y 12 hilos, y también los `last_phase_hashes()` de cada tick; el test se repite cinco veces seguidas con el pool de 6 hilos y da lo mismo; y una pasada con 15.000 filas (30 chunks) y 20 ticks da el mismo hash con 1 y 12 hilos. Lo demuestra: test `ca_12_mismo_hash_con_1_3_6_12_hilos`.
- **CA-13.** Regla 6. El mundo de prueba (`ia: true`), 150 ticks con pool de 3 hilos y una orden externa enviada con `submit` justo antes de guardar, de modo que `pending_orders()` tiene esa y las de `ctx.submit` del tick 150; `save_body` y `encode_save`; `decode_save` con el mismo hash de datos y `Simulation::load` con pool de 6 hilos; `pending_orders()` coincide tras cargar; las dos simulaciones avanzan 150 ticks más y sus `state_hash()` coinciden tick a tick con los de una tercera que recibió la misma orden en el tick 150 y avanzó 300 ticks seguidos con pool de 1 hilo; cargar con un `SimConfig` de `hash_every` distinto no cambia ningún `state_hash()`; `load` con `order_capacity` menor que lo pendiente da `LoadError::Queue(Full)`; `load` con `hash_every: Some(Ticks(0))` hace pánico `periodo cero`; `decode_save` con otro hash de datos da `DataHashMismatch`; `decode_save::<MundoV2>` de un guardado de `MundoPrueba` da `UnsupportedFormat { expected: 65_538, found: 65_537 }`; `load` de un cuerpo cuyo `world` se ha alterado da `LoadError::HashMismatch`; `save_body` en el tick 0 y carga inmediata reproducen el mismo hash y `origin_tick() == Tick::ZERO`; tras cargar en el tick 150, `origin_tick() == Tick(150)`. Lo demuestran: tests `ca_13_guardar_cargar_y_continuar_con_otro_pool` y `ca_13_load_periodo_cero_panica`.
- **CA-14.** Replay. El mundo de prueba (`ia: true`), 300 ticks con `hash_every: Some(Ticks(25))` y órdenes externas cada 7 ticks, la última aplicada en el tick 294: `replay()` devuelve `Ok` con `last_tick == Tick(300)`, 12 puntos de control, solo las órdenes externas (una cada 7 ticks, ninguna de `ctx.submit`) selladas con su tick de aplicación, y `final_hash == state_hash()`; `encode_replay` y `decode_replay` devuelven una struct igual, y los vectores de bytes fijados en "Datos y formatos" se cumplen con un `Replay<(), u8>`; una simulación nueva con el mismo escenario y semilla, `prepare_replay`, y 300 `step` con pool de 6 hilos y `SimConfig { hash_every: None, .. }` termina con el mismo `state_hash()`, sin ningún `divergence`, con `pending_orders()` igual al de la partida original en cada punto de control, y el mundo vuelve a emitir sus órdenes de IA sin duplicarlas; `submit` durante la reproducción da `Replaying` y `replay()` da `Reproducing`; un `submit` tras el último `step` de la partida original hace que `replay()` dé `PendingExternal`, y un `step` más lo arregla; `prepare_replay` sobre una simulación que ya avanzó, con otra semilla, o por segunda vez, da `WrongStart`; `expect_checkpoints` con ticks desordenados, repetidos o ya pasados da `BadCheckpoints`, y con un hash para el tick 1.000 no produce divergencia en 300 ticks; cambiar una orden del replay produce `divergence` en el primer punto de control posterior y no antes; `decode_replay` con otro hash de datos da `DataHashMismatch` y con `MundoV2`, `UnsupportedFormat`; `replay()` de una simulación cargada con `load` da `NotFromStart`; con `replay_orders: 8`, el noveno `submit` entra en la cola, el `step` siguiente devuelve `replay_truncated: true` una sola vez, los `step` posteriores lo devuelven falso aunque haya más órdenes y puntos de control, y `replay()` da `Truncated { at }` con el tick para el que se selló esa orden; y reproducir con `order_capacity: 1` un replay con dos órdenes externas en el mismo tick hace pánico `cola de órdenes llena` en el `step` de ese tick. Lo demuestran: tests `ca_14_replay_reproduce_y_detecta_divergencias`, `ca_14_replay_bytes_fijados_y_limites` y `ca_14_replay_cola_llena_panica`.
- **CA-15.** Buscador de desincronizaciones. Un mundo de prueba cuya tercera fase escribe `ctx.pool().threads()` en una columna (no determinista a propósito): con `phase_hashes` activo, `first_divergence` entre los `last_phase_hashes()` del mismo tick con pools de 1 y 3 hilos devuelve `Some(2)`, y `Some(0)` si es la primera fase la que lo hace; con el mundo de prueba normal devuelve `None` en todos los ticks; con listas de distinta longitud devuelve la longitud de la corta. Lo demuestra: test `ca_15_buscador_senala_la_fase_que_diverge`.
- **CA-16.** Grid de 10 por 10 celdas de tamaño 2 con origen `(0, 0)`: `cell_of((0.5, 0.5)) == CellId(0)`, `cell_of((19.9, 19.9)) == CellId(99)`, `cell_of((-5, 50)) == CellId(90)`, `cell_of((-0.0, INFINITY)) == CellId(90)`, `cell_of((NaN, 3)) == CellId(10)`; con 1.000 posiciones pseudoaleatorias, `rebuild` deja cada `rows_in_cell` ascendente, la unión de todas las celdas es exactamente `0..1000`, y cada fila está en la celda de `cell_of`; `for_each_in_radius` con radio 3 desde 50 centros visita el mismo conjunto de filas que una comprobación de todas contra todas, en orden de celda `(y, x)` y fila ascendente, y devuelve un número de candidatos ≥ el de visitadas; con una fila en el centro de cada celda, desde `(5, 5)` con radio 2 examina exactamente 9 candidatos y visita 5; con radio 0 solo visita filas en la posición exacta; con radio negativo o NaN no visita ninguna y devuelve 0; con radio 100 visita las 1.000 una vez; `cell_order` devuelve una permutación de `0..1000` ordenada por `(celda, fila)`; un grid de una celda y otro con `max_rows == 0` funcionan; `rebuild` con más de `max_rows` posiciones hace pánico `scratch insuficiente`; `Grid::new` con `cell_size` 0, con `origin` infinito, con `cells_x` 0 y con `cells_x == cells_y == 65_536` (cuyo producto en `u32` daría 0) hace pánico `configuración de grid inválida`. Lo demuestran: tests `ca_16_grid_orden_estable_y_vecinos`, `ca_16_grid_casos_limite` y `ca_16_grid_configuracion_invalida_panica`.
- **CA-17.** Regla 9. Posiciones uniformes a densidad constante: 5.000 filas en un mapa de 100 por 100 y 10.000 en uno de 141 por 141, celdas y radio de tamaño 2; la suma de candidatos examinados por `for_each_in_radius` desde todas las filas, acumulada por chunk con `par_scan` y `reduce_in_order`, da para 10.000 entre 1,7 y 2,3 veces lo que da para 5.000, y el mismo valor con pools de 1 y 6 hilos. Lo demuestra: test `ca_17_contador_de_vecinos_lineal_con_densidad_constante`.
- **CA-18.** Regla 19. El mundo de prueba (`ia: true`) con 4.000 filas, `SimConfig { hash_every: Some(Ticks(1)), replay_orders: 1024, replay_checkpoints: 1024, .. }` y un `Pool` de 1 hilo del que la simulación recibe un clon: tras cinco ticks de calentamiento, `pool.install(|| allocation_counter::measure(|| sim.step()))` cuenta 0 asignaciones en 20 `step` seguidos sin órdenes externas, y 0 en 20 `step` con una orden externa por tick, incluidos los ticks en que el mundo reordena (`cell_order` y `permute_active`) y cambia el conjunto activo. Lo demuestra: test `ca_18_tick_sin_asignaciones`.
- **CA-19.** Reglas 7 y 17. El texto de todos los archivos de `engine/sim/src/` (los once de la tabla más `lib.rs`, leídos con `include_str!`) no contiene `Instant`, `SystemTime`, `thread_rng`, `current_num_threads`, `par_bridge` ni `static ` seguido de un identificador (la expresión `(^|\s)static\s+(mut\s+)?[A-Za-z_]`), ni, como palabra completa, los identificadores `seconds`, `secs`, `dt`, `elapsed` o `delta_time`; `lib.rs` contiene `#![forbid(unsafe_code)]` y `#![deny(missing_docs)]`. Lo demuestra: test `ca_19_sin_tiempo_real_ni_globales_en_el_codigo`.
- **CA-20.** Existen y corren con `cargo bench -p engine_sim`: `benches/tick_sintetico.rs` (el mundo de prueba con 300 y con 15.000 filas, en ticks por segundo, con pools de 1, 3 y 5 hilos), `benches/tabla.rs` (10.000 `insert`, 10.000 `mark_removed` más `apply_removals`, `permute_active` de 15.000 filas), `benches/grid.rs` (`rebuild` y consultas de radio con 300 y con 15.000 filas, en un grid de 2^12 celdas y en uno de `MAX_CELLS`) y `benches/guardado.rs` (`save_body` y `encode_save` de un mundo de 15.000 filas, en milisegundos y bytes). Lo demuestra: métrica informativa; umbrales en el hito 2.
- **CA-21.** `include_str!` de `src/lib.rs` contiene los dos atributos; `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_sim --no-deps` termina sin avisos; `cargo tree -p engine_sim -e normal --depth 1` lista exactamente `engine_core`, `rayon`, `serde` y `thiserror` como dependencias directas; `cargo tree -p engine_sim -e normal` no contiene `wgpu`, `winit` ni `egui`; `cargo xtask check-arch` pasa; `engine/sim/clippy.toml` existe (lo crea el contrato de `engine_core`). Lo demuestran: test `ca_21_lib_rs_lleva_los_atributos` y la puerta adicional, cuyos `cargo tree` compara el orquestador con la lista en `/verificar`.
- **CA-22.** `FixedVec::<u8>::with_capacity(4)`: `push` de 7 y 8, `len() == 2`, `capacity() == 4`, `as_slice() == [7, 8]`; `try_push(9)` da `Ok(())`, `try_push(10)` da `Ok(())` y `try_push(11)` da `Err(11)` con el buffer intacto; `push` con el buffer lleno hace pánico `columna llena`; `pop()` devuelve `Some(10)`; `truncate(1)` deja `[7]`; `swap(0, 0)` no cambia nada y `swap(0, 1)` con `len() == 1` hace pánico `índice fuera del buffer`; `clear()` deja `len() == 0` y `capacity() == 4`; `clone()` de un buffer con `[7, 8]` conserva `capacity() == 4`; `postcard::to_allocvec` de `[7, 8]` da `02 07 08`, `from_bytes` devuelve `[7, 8]` con `capacity() == 2` y `reserve(4)` la sube a 4; `hash_of` del buffer es igual a `hash_of(&[7u8, 8][..])`. Lo demuestran: tests `ca_22_fixedvec_capacidad_fija` y `ca_22_fixedvec_lleno_y_fuera_de_rango_panican`.

**Reparto previsto en tareas**, para `/planificar`. Todas en serie: una tarea que toca `engine_sim` no va en paralelo con otra de `engine_sim`. Ninguna empieza antes de que estén integradas las tareas de `engine_core` que usa: la 2 necesita `hash` (tarea 5 de `engine_core`), la 7 necesita `rng` (6), y la 8 y la 9 necesitan `save` (8); lo simple es empezar con las tareas 1 a 8 de `engine_core` integradas.

1. Contrato A: `[workspace.dependencies]` (`rayon`, `engine_core` por ruta y versión), `engine/sim/Cargo.toml` (feature `tracy`, dev-dependencies, cuatro `[[bench]]` con `harness = false` y sus archivos con `fn main() {}`), y las firmas con `todo!()` de `buffer`, `table`, `parallel`, `events`, `orders` y `grid`.
2. `buffer`, `Column`, `Columns` y `Table` hasta `apply_removals`, con `clear` y `Clone` (CA-1, CA-2, CA-22).
3. `set_active`, `permute_active`, `HashState` y `serde` de `table` (CA-3 a CA-6).
4. `parallel` y `events` (CA-7 a CA-9).
5. `orders` y `grid` (CA-10, CA-16, CA-17).
6. Contrato B: firmas de `world`, `sim`, `replay`, `save` y `desync`.
7. `world`, `sim` y `desync`, con `save_body`, `load`, `replay`, `prepare_replay` y `expect_checkpoints` todavía en `todo!()`; el mundo de prueba nace aquí (CA-11, CA-12, CA-15).
8. `save`, `save_body` y `load` (CA-13).
9. `replay`, `replay`, `prepare_replay` y `expect_checkpoints` (CA-14).
10. Cierre: `tracy`, benchmarks, CA-18 a CA-21.

Cada contrato ronda las 300 líneas con documentación; ninguna implementación pasa de unas 400 líneas de diff, y si la 2 pasa, se parte en `buffer` más `Column` y `Columns` por un lado y `Table` por otro.

## Casos límite

- **Tabla.** Capacidad 0 (CA-3); tabla llena y `mark_removed` seguido de `insert` en el mismo tick (CA-2); `set_active` de una fila con borrado pendiente (CA-4); `apply_removals` con filas inactivas (CA-4); `permute_active` con borrados pendientes (CA-3); una sola entidad y todas inactivas (CA-3); `handle_at(Row(len))` (CA-1); `clear` no resucita handles y fija el orden de la pila (CA-1, CA-3); distinta capacidad con el mismo contenido da distinto hash (CA-6); generación que da la vuelta en `u32::MAX` (cubierto por `Handle::next_generation`, CA-3 de `engine_core`).
- **Buffer.** Lleno, vacío, `try_push` que no cabe, `swap` fuera de rango, capacidad tras deserializar (CA-22).
- **Chunks.** `len == 0` (CA-7); `len == CHUNK` y `CHUNK + 1` (CA-7); scratch con más chunks de los necesarios (CA-7); pool de un hilo (CA-7, CA-18).
- **Eventos.** Ningún evento, todos con la misma clave, un chunk sin eventos entre dos con eventos, capacidad justa (CA-8).
- **Órdenes.** Cola vacía y `drain_for(Tick(0))` (CA-10); todas del mismo tick (CA-10); `order_capacity` menor que las órdenes de un tick del replay (pánico `cola de órdenes llena` en `step`, CA-14); `ctx.submit` con la cola llena (pánico, CA-11); una orden sellada para `Tick(u64::MAX)` haría pánico `desbordamiento de ticks` en `engine_core` al sellar la siguiente: imposible en la práctica (regla 4: 2^64 ticks), documentado y sin test.
- **Simulación.** `step` sin fases (`MundoVacio`, CA-11); `hash_every: Some(Ticks(1))`, `None` y `Some(Ticks(0))` en `new` y en `load` (CA-11, CA-13); `expect_checkpoints` con ticks que nunca llegan, desordenados, repetidos o pasados (CA-14); carga con otro `SimConfig` (CA-13); `world_mut` tras el tick 0 (CA-11); `save_body` en el tick 0 (CA-13); registro de replay lleno (CA-14); `replay()` con una orden externa sin aplicar, durante una reproducción o tras `load` (CA-14); `prepare_replay` dos veces (CA-14); fases mal anidadas (CA-11).
- **Grid.** `max_rows == 0`, una celda, todas en la misma celda, radio mayor que el mapa, radio negativo o NaN, infinitos y `-0.0`, `origin` no finito (CA-16); `cells_x * cells_y` mayor que `MAX_CELLS`, comprobado en `u64` (CA-16).
- **Hilos.** Todo lo que produce resultado se comprueba con 1, 3, 6 y 12 hilos (CA-7, CA-8, CA-12, CA-14, CA-17); el único comportamiento que depende del pool es el del mundo adrede no determinista de CA-15.

## Fuera de alcance

- Las tablas, fases, órdenes, eventos y escenarios concretos del juego: `game_rules`. El orden de fases de la simulación real y los desempates de combate: la spec de combate.
- La simulación sintética del hito 0 como escenario (10.000 unidades, paseo aleatorio, 10.000 ticks, binario sin gráficos con ticks por segundo, dónde vive ese binario, qué RON carga, el valor de `hash_every` en release): `specs/engine/simulacion-sintetica.md`. El mundo de prueba de esta spec es solo para sus tests. Esa spec incluye al menos un borrado de entidad a mitad de partida, antes del punto de guardado, para que el orden denso de las tablas tras cargar difiera del vivo y el test de la regla 6 atrape cualquier extracción de `rng` o recorrido que dependa de ese orden (Q-022 de `engine_core`).
- Snapshots para el render, anillo de buffers, interpolación, hilo de simulación y canales con la app: `specs/engine/snapshots.md`, hito 1. Esa spec decide también si el hilo de entrada y salida vacía el registro de replay por trozos para que no se llene (decisión 9).
- Navegación, flow fields, pasabilidad por facción, recálculo incremental con presupuesto (regla 10) y el tamaño de su grid: `specs/engine/navegacion.md`, hito 2, tras la pregunta 16 del GDD.
- Grid de visibilidad por equipo: hito 3, si el autor confirma la recomendación sobre la niebla.
- Replay desde un guardado (reproducir una partida cargada) y migraciones de guardados: sin hito asignado; se piden cuando hagan falta (decisión 6).
- El visor de replays y la herramienta de comparación de dos ejecuciones: hito 1, sobre `first_divergence`.
- Reparto de la IA entre ticks y presupuestos por capa: `game_rules`, como estado canónico suyo.
- Reordenación periódica por celda como política (cada cuántos ticks): lo decide `game_rules`; aquí solo `cell_order` y `permute_active`.
- Lo que `rayon` asigne por dentro con más de un hilo: se mide en el hito 2.
- Hashes dorados de partidas: las specs de features cuando exista el código.

## Decisiones de redacción

Las tomó el orquestador porque ningún documento las fijaba. El autor las confirma o las cambia al aprobar; cada una dice qué criterios cambian si no se confirma.

1. **El conjunto activo es un prefijo denso de la tabla**, no una columna de banderas: las fases recorren `0..active_len()` con `par_map` y escriben su salida por fila sin dispersión, y una unidad de guarnición no cuesta ni la comprobación de la bandera. El precio: activar o desactivar mueve una fila por intercambio. Cambia CA-3, CA-4 y CA-5.
2. **Una orden se aplica en el tick siguiente al que se envía**, venga del jugador entre ticks o de la IA dentro de un tick. Es la latencia de "50 ms más un frame" del ADR-0006 y hace que la cola sea la misma para los dos (regla 15). Cambia CA-10 y CA-11.
3. **Eventos ordenados con `sort_unstable` y secuencia de emisión como desempate**, porque el `sort` estable de la biblioteca estándar asigna un buffer temporal (regla 19; medido: 1 asignación por llamada frente a 0). Mismo resultado que un orden estable. Cambia CA-8.
4. **`Columns` como trait escrito a mano más `Column<T>`**, sin macro, siguiendo ADR-0004 ("si el hito 3 lo pide, una macro"). Cambia CA-1 a CA-6.
5. **`Simulation<W>` genérica con el trait `World`**: el motor posee el bucle, la cola, el replay y el guardado, y el juego posee el orden de las fases dentro de `step`. El "scheduler determinista" son `par_map`, `par_scan`, `ChunkScratch` y `TickContext`, no una lista declarativa de sistemas: una lista obligaría a un segundo modelo de datos y las fases del juego se conocen de antemano (ADR-0002, nada genérico sin uso). Cambia CA-11 a CA-15.
6. **El replay graba solo las órdenes externas (`Simulation::submit`), más los puntos de control y el hash final; la IA no se graba porque se regenera.** Las órdenes de `TickContext::submit` salen del estado canónico, la semilla y el tick, así que al reproducir el mundo las vuelve a emitir idénticas (regla 13); grabarlas obligaría a descartarlas al reproducir y duplicaría el tamaño. Cumple el motivo de la regla 15, que el replay reproduzca lo que hizo la IA, y es "esa cola es el replay" del GDD: la cola es el canal de entrada. El replay solo arranca en el tick 0 y no se pide con órdenes externas sin aplicar; reproducir desde un guardado queda sin hito hasta que haga falta. Cambia CA-14.
7. **El grid es derivado y se reconstruye entero cada tick** con counting sort, en vez de mantenerse incremental: la revisión técnica estima 0,2-0,3 ms para 15.000 unidades, y no deja progreso a medias que hubiera que guardar (regla 6). Cambia CA-16 y CA-17.
8. **Capacidades fijas con error o pánico en vez de crecimiento**: `TableFull`, `QueueError::Full`, `columna llena`, `eventos llenos`. Los topes salen de los datos de juego y de los límites del mapa. Cambia CA-1, CA-8 y CA-10.
9. **El registro de replay también tiene capacidad fija** (`replay_orders`, `replay_checkpoints`) y, al llenarse, deja de grabar y lo avisa con `replay_truncated` y `ReplayError::Truncated`, en vez de crecer o de detener la partida. Así la regla 19 no necesita excepción. La alternativa, que el hilo de entrada y salida vacíe el registro por trozos, la decide la spec de hilos del hito 1 y no cambia este contrato. Cambia CA-14 y CA-18.
10. **El `Pool` es un `Arc` clonable que la simulación comparte**, y el número de hilos se decide al crearlo; cargar con otro pool es el caso normal de los tests (CA-13), y el clon es lo que permite medir asignaciones dentro de `install` (CA-18). Cambia CA-12, CA-13 y CA-18.
11. **El mundo adrede no determinista de CA-15** escribe el número de hilos en una columna: es la única forma barata de probar el buscador sin depender de una carrera. Cambia CA-15.
12. **`MAX_CELLS = 2^20`** para el grid de vecinos, porque `rebuild` recorre todas las celdas cada tick y el presupuesto que manda es el de 300 unidades; el tamaño de celda de cada mapa se elige con `benches/grid.rs`. El grid de navegación del hito 2 es otra estructura. Cambia CA-16 y CA-20.
13. **Los valores de `SimConfig::default()` son iniciales**, no medidos: 1.024 órdenes, 64 fases, 65.536 órdenes y 16.384 puntos de control en el registro. El hito 2 los ajusta. Cambia CA-11.
14. **La versión de formato se compone**: 16 bits del motor (`SAVE_FORMAT_VERSION`, `REPLAY_FORMAT_VERSION`) y 16 bits del juego (`World::FORMAT_VERSION`), para que `game_rules` invalide guardados y replays antiguos sin una tarea de contrato en `engine_sim`, y al revés. Cambia CA-13 y CA-14.
15. **`begin_phase` no abre zonas de perfilado**: `engine_core::span!` exige un literal (contrato aprobado) y cada fase abre la suya dentro de un bloque. `begin_phase` solo cuenta y nombra. Cambia CA-11.
16. **`Clone` conserva la capacidad** en todos los tipos con capacidad fija, con implementaciones propias; un `derive` daría clones que hacen pánico al primer `push`. Cambia CA-1, CA-10 y CA-22.
17. **`TickContext::submit` hace pánico con la cola llena** en vez de devolver un error, para que `order_capacity` no pueda decidir el resultado sin ser canónica (regla 6): el tope se dimensiona por datos. `Simulation::submit` sí devuelve `Full`, porque fuera del tick el llamante puede esperar. Cambia CA-11.
18. **La tabla de estado de la constitución pasa a "hecha" con los tests de la simulación sintética**, no con los de este crate: son los que la constitución nombra y los que corren a la escala del hito 0; los de aquí son la primera línea. Cambia nada; es una lectura del orquestador.
19. **`Simulation::step` entra una sola vez en el pool por tick**, y las fases corren en línea dentro; desde el hilo de simulación de la app, fuera del pool, cada entrada despierta a un trabajador. Cambia CA-18.

## Preguntas abiertas

Ninguna pendiente. Las decisiones 6, 9, 12, 13, 14 y 17 afectan a CA-11, CA-13, CA-14, CA-16, CA-18 y CA-20 y las confirma el autor al aprobar; la spec no pasa a aprobada sin esa confirmación. Las cuatro cuestiones que planteó la revisión (qué graba el replay, qué pasa al llenarse el registro, qué hace la IA con la cola llena y si el replay desde guardado tiene hito) están en las decisiones 6, 9, 17 y 6.

//! Fases paralelas deterministas (regla 13): `CHUNK`, `ChunkId`, `Pool`,
//! `ChunkScratch<S>`, `par_map`, `par_scan` y `reduce_in_order`. El trabajo se
//! reparte en chunks de `CHUNK` filas fijas, nunca por número de hilos; cada
//! chunk escribe solo en su trozo de salida y en su scratch, y las reducciones
//! son secuenciales y en orden de chunk.
//!
//! Es el único archivo del crate que nombra `rayon`: `Pool` envuelve su pool
//! de hilos y `Pool::install` es la única puerta a él.

use std::sync::Arc;

use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use rayon::slice::{ParallelSlice, ParallelSliceMut};

use crate::table::Row;

/// Filas por chunk. Fijo, para que el número de chunks no dependa de los hilos (regla 13).
pub const CHUNK: usize = 512;

/// `CHUNK` como `u32`, para la aritmética de `ChunkId::first_row` en `const fn`. La aserción de abajo lo ata a
/// `CHUNK` en tiempo de compilación.
const CHUNK_U32: u32 = 512;
const _: () = assert!(
    CHUNK_U32 as usize == CHUNK,
    "CHUNK_U32 tiene que valer CHUNK"
);

/// Chunks que hacen falta para `len` filas: `len.div_ceil(CHUNK)`; 0 para 0.
#[must_use]
pub const fn chunk_count(len: usize) -> usize {
    len.div_ceil(CHUNK)
}

/// Índice de un chunk: el `i` que recibe las filas `i * CHUNK .. min((i + 1) * CHUNK, len)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ChunkId(pub u32);

impl ChunkId {
    /// `Row(self.0 * CHUNK)`: la primera fila del chunk, para leer otras columnas por fila.
    #[must_use]
    pub const fn first_row(self) -> Row {
        Row(self.0 * CHUNK_U32)
    }

    /// El chunk en la posición `index` de un scratch. Las filas caben en `u32` (`Row`), así que los chunks también.
    fn at(index: usize) -> ChunkId {
        ChunkId(u32::try_from(index).expect("el índice de chunk cabe en u32"))
    }

    /// La posición del chunk en un scratch.
    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// El pool de `rayon` de la simulación, compartido por clones (`Arc`). Lo crea quien arranca la simulación con los hilos
/// que decida (núcleos físicos menos uno en el juego; 1, 3, 6 y 12 en los tests). `new` guarda el número de hilos y
/// `threads` lo devuelve: nada en el crate pregunta a `rayon` cuántos hilos hay.
#[derive(Clone)]
pub struct Pool {
    /// El pool de hilos, compartido por los clones.
    inner: Arc<rayon::ThreadPool>,
    /// Los hilos con los que se creó.
    threads: usize,
}

/// Fallos al crear el pool.
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    /// `Pool::new(0)`.
    #[error("el pool necesita al menos un hilo")]
    ZeroThreads,
    /// `rayon` no pudo crear el pool; lleva su mensaje.
    #[error("no se pudo crear el pool: {0}")]
    Build(String),
}

impl Pool {
    /// Asigna. `threads == 0` da `ZeroThreads`.
    pub fn new(threads: usize) -> Result<Pool, PoolError> {
        if threads == 0 {
            return Err(PoolError::ZeroThreads);
        }
        // Siempre con el número explícito, nunca 0: con 0 `rayon` leería `RAYON_NUM_THREADS` y el pool dependería del
        // entorno.
        let inner = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|error| PoolError::Build(error.to_string()))?;
        Ok(Pool {
            inner: Arc::new(inner),
            threads,
        })
    }

    /// Los hilos con los que se creó el pool.
    #[must_use]
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Ejecuta `f` dentro del pool; si ya se está dentro de él, en línea en el hilo actual. Desde un hilo ajeno cuesta
    /// despertar a un trabajador, por eso `Simulation::step` entra una sola vez por tick. Es la única puerta a `rayon`;
    /// `par_map` y `par_scan` la usan.
    pub fn install<R: Send>(&self, f: impl FnOnce() -> R + Send) -> R {
        self.inner.install(f)
    }
}

/// Un valor por chunk, en orden de chunk. Es la "arena por hilo" de la regla 19, pero por chunk, para que el reparto no
/// dependa del número de hilos. Aquí viven los buffers de eventos, los vecinos y los contadores de cada fase.
pub struct ChunkScratch<S> {
    /// Un valor por chunk, en orden de chunk.
    chunks: Vec<S>,
}

/// Clona cada valor en orden de chunk.
impl<S: Clone> Clone for ChunkScratch<S> {
    fn clone(&self) -> ChunkScratch<S> {
        ChunkScratch {
            chunks: self.chunks.clone(),
        }
    }
}

impl<S> ChunkScratch<S> {
    /// `chunk_count(max_len)` valores, construidos en orden con `make`. Asigna.
    pub fn new(max_len: usize, mut make: impl FnMut(ChunkId) -> S) -> ChunkScratch<S> {
        let chunks = (0..chunk_count(max_len))
            .map(|index| make(ChunkId::at(index)))
            .collect();
        ChunkScratch { chunks }
    }

    /// Número de chunks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    /// Cierto si no hay ningún chunk.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// El valor del chunk.
    #[must_use]
    pub fn get(&self, chunk: ChunkId) -> &S {
        &self.chunks[chunk.index()]
    }

    /// El valor del chunk, mutable.
    #[must_use]
    pub fn get_mut(&mut self, chunk: ChunkId) -> &mut S {
        &mut self.chunks[chunk.index()]
    }

    /// Los valores con su chunk, en orden de chunk.
    pub fn iter(&self) -> impl Iterator<Item = (ChunkId, &S)> {
        self.chunks
            .iter()
            .enumerate()
            .map(|(index, chunk)| (ChunkId::at(index), chunk))
    }

    /// Los valores con su chunk, mutables, en orden de chunk.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ChunkId, &mut S)> {
        self.chunks
            .iter_mut()
            .enumerate()
            .map(|(index, chunk)| (ChunkId::at(index), chunk))
    }

    /// Los primeros `count` chunks, mutables, para repartirlos entre los trozos de una fase.
    ///
    /// # Panics
    ///
    /// Pánico `scratch insuficiente` si hay menos de `count` chunks.
    fn take_mut(&mut self, count: usize) -> &mut [S] {
        assert!(self.chunks.len() >= count, "scratch insuficiente");
        &mut self.chunks[..count]
    }
}

/// Fase paralela: map sobre entrada congelada, salida por fila. El chunk `i` recibe las filas
/// `i * CHUNK .. min((i + 1) * CHUNK, len)` de `input` y de `output`, y su `scratch`. El reparto no depende del número
/// de hilos; el orden de ejecución sí, y por eso `f` solo escribe en su trozo de `output` y en su `scratch`, y lee lo demás
/// (otras columnas, el grid) por referencia congelada. Exige `input.len() == output.len()` (pánico `longitudes distintas`)
/// y `scratch.len() >= chunk_count(input.len())` (pánico `scratch insuficiente`); con `len == 0` no llama a `f`. No asigna.
///
/// # Panics
///
/// Pánico `longitudes distintas` si `input.len() != output.len()` y `scratch insuficiente` si
/// `scratch.len() < chunk_count(input.len())`.
pub fn par_map<I, O, S, F>(
    pool: &Pool,
    input: &[I],
    output: &mut [O],
    scratch: &mut ChunkScratch<S>,
    f: F,
) where
    I: Sync,
    O: Send,
    S: Send,
    F: Fn(ChunkId, &[I], &mut [O], &mut S) + Sync + Send,
{
    assert_eq!(input.len(), output.len(), "longitudes distintas");
    let count = chunk_count(input.len());
    let chunks = scratch.take_mut(count);
    if count == 0 {
        return;
    }
    // `par_chunks` y `par_chunks_mut` trocean por `CHUNK` filas fijas y `zip` empareja por índice: el chunk `i` recibe
    // siempre las mismas filas y el mismo scratch, sea cual sea el número de hilos. Ninguna reducción: cada chunk solo
    // escribe en lo suyo.
    pool.install(|| {
        input
            .par_chunks(CHUNK)
            .zip(output.par_chunks_mut(CHUNK))
            .zip(chunks.par_iter_mut())
            .enumerate()
            .for_each(|(index, ((input, output), scratch))| {
                f(ChunkId::at(index), input, output, scratch);
            });
    });
}

/// Igual que `par_map` sin salida por fila: la fase solo produce eventos y contadores en su `scratch`.
///
/// # Panics
///
/// Pánico `scratch insuficiente` si `scratch.len() < chunk_count(input.len())`.
pub fn par_scan<I, S, F>(pool: &Pool, input: &[I], scratch: &mut ChunkScratch<S>, f: F)
where
    I: Sync,
    S: Send,
    F: Fn(ChunkId, &[I], &mut S) + Sync + Send,
{
    let count = chunk_count(input.len());
    let chunks = scratch.take_mut(count);
    if count == 0 {
        return;
    }
    pool.install(|| {
        input
            .par_chunks(CHUNK)
            .zip(chunks.par_iter_mut())
            .enumerate()
            .for_each(|(index, (input, scratch))| {
                f(ChunkId::at(index), input, scratch);
            });
    });
}

/// Lo que se puede sumar en orden de chunk: contadores enteros (regla 9) y cualquier reducción. Las reducciones
/// paralelas de flotantes están prohibidas (regla 13); esta es secuencial y en orden fijo.
pub trait Accumulate: Default {
    /// Suma `other` a `self`.
    fn accumulate(&mut self, other: &Self);
}

/// `A::default()` más `get(scratch de cada chunk)` en orden de chunk.
#[must_use]
pub fn reduce_in_order<S, A: Accumulate>(scratch: &ChunkScratch<S>, get: impl Fn(&S) -> &A) -> A {
    let mut total = A::default();
    for chunk in &scratch.chunks {
        total.accumulate(get(chunk));
    }
    total
}

//! Colecciones con hasher fijo (ADR-0009, regla 13).
//!
//! Son alias de `rustc-hash`; ningún otro crate del workspace declara esa
//! dependencia. Dos colecciones construidas con la misma secuencia de
//! operaciones dan el mismo resultado en cualquier proceso e hilo, pero el
//! orden de iteración sigue siendo arbitrario y no puede influir en la
//! simulación.
// Contrato A: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T02 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T02")]

/// Mapa con hasher fijo (ADR-0009). Se construye con `FxHashMap::default()` o con `fx_map_with_capacity`.
/// Garantiza el mismo resultado para la misma secuencia de operaciones. No garantiza ningún orden de iteración:
/// un resultado que dependa de iterar un mapa viola la regla 13.
pub type FxHashMap<K, V> = rustc_hash::FxHashMap<K, V>;
/// Conjunto con hasher fijo (ADR-0009). Mismas garantías y mismos límites que `FxHashMap`.
pub type FxHashSet<T> = rustc_hash::FxHashSet<T>;
/// Constructor de hasher y hasher de `rustc-hash`, para quien necesite nombrarlos en una firma.
pub use rustc_hash::{FxBuildHasher, FxHasher};

/// Reserva `capacity` entradas. Asigna: no es para el camino caliente.
#[must_use]
pub fn fx_map_with_capacity<K, V>(capacity: usize) -> FxHashMap<K, V> {
    todo!("P01-T02")
}

/// Reserva `capacity` elementos. Asigna: no es para el camino caliente.
#[must_use]
pub fn fx_set_with_capacity<T>(capacity: usize) -> FxHashSet<T> {
    todo!("P01-T02")
}

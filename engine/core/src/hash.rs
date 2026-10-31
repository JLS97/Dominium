//! Hash de estado determinista (regla 13, ADR-0009): `StateHasher`, `StateHash`
//! y el trait `HashState`, más el reexport de `bytemuck`.
//!
//! Cada valor entra como una secuencia de bytes fija, independiente de la
//! plataforma, el número de hilos y el perfil de compilación. La vía en
//! bloque (`cast_slice` sobre una columna entera) produce exactamente los
//! mismos bytes que la secuencia elemento a elemento, y por eso solo vale en
//! little-endian.

use core::fmt;

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::Xxh3;

use crate::data::DataHash;
use crate::handle::Handle;
use crate::math::{IVec2, IVec3, UVec2, UVec3, Vec2, Vec3, Vec4};
use crate::rng::Seed;
use crate::time::{Tick, Ticks};
use crate::version::Version;

/// Reexport de `bytemuck`, para que `engine_sim` y `game_rules` pasen columnas a bytes con `cast_slice` sin
/// declararlo (Q-008, ADR-0009). Solo la API segura: aquí no hay `unsafe` que la acompañe.
pub use bytemuck;

// La vía en bloque del hash de estado lee los bytes en memoria de las columnas tal cual, y solo
// coinciden con la secuencia little-endian elemento a elemento en una plataforma little-endian
// (la fase A es `x86_64-pc-windows-msvc`, ADR-0001).
const _: () = assert!(cfg!(target_endian = "little"));

/// Handles que `write_handles` convierte de golpe antes de cada `write`. El buffer de pila ocupa
/// `HANDLE_BLOCK_LEN * 8` = 512 bytes: múltiplo de la banda de 64 bytes que consume xxh3 y el doble
/// de su buffer interno de 256 bytes, y lo bastante pequeño para no pesar en ninguna pila. La spec no
/// fija el tamaño; los tests de CA-7 barren longitudes de 0 a 1.024 y 15.000, que cruzan este límite.
const HANDLE_BLOCK_LEN: usize = 64;

/// Hash de 64 bits del estado, o de una parte de él. `Display` lo escribe en hexadecimal de 16 dígitos en minúsculas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StateHash(pub u64);

impl fmt::Display for StateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

/// xxh3 de 64 bits en streaming con semilla 0 (ADR-0009). Vive en la pila (576 bytes) y no asigna.
/// El hash de una secuencia de bytes no depende de cómo se trocee entre llamadas a `write*`.
#[derive(Clone, Default)]
pub struct StateHasher {
    /// Estado en streaming de xxh3. Privado: el algoritmo se cambia tocando solo este crate.
    inner: Xxh3,
}

impl StateHasher {
    /// Hasher vacío: `finish()` da el hash de la secuencia vacía.
    #[must_use]
    pub fn new() -> StateHasher {
        StateHasher { inner: Xxh3::new() }
    }

    /// Entran `bytes` tal cual.
    pub fn write(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    /// Los enteros entran en little-endian con su anchura exacta.
    pub fn write_u8(&mut self, v: u8) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, dos bytes.
    pub fn write_u16(&mut self, v: u16) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, cuatro bytes.
    pub fn write_u32(&mut self, v: u32) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, ocho bytes.
    pub fn write_u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` como un byte.
    pub fn write_i8(&mut self, v: i8) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, dos bytes.
    pub fn write_i16(&mut self, v: i16) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, cuatro bytes.
    pub fn write_i32(&mut self, v: i32) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v` en little-endian, ocho bytes.
    pub fn write_i64(&mut self, v: i64) {
        self.write(&v.to_le_bytes());
    }

    /// Entra `v.to_bits()` en little-endian, sin normalizar: `0.0` y `-0.0` dan hashes distintos y cada NaN conserva su carga.
    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }

    /// Entra `v.to_bits()` en little-endian, sin normalizar, igual que `write_f32`.
    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    /// Un byte: 0 o 1.
    pub fn write_bool(&mut self, v: bool) {
        self.write_u8(u8::from(v));
    }

    /// Los `to_raw()` de `handles` en orden, como `u64` little-endian y sin longitud delante. Mismos bytes que
    /// `write_u64(h.to_raw())` por elemento, pero troceados en bloques desde un buffer de pila de tamaño fijo: es la
    /// vía en bloque de las columnas de handles, que no pueden ir por `cast_slice` porque su memoria es (slot, generación).
    /// Con un slice vacío no escribe nada.
    pub fn write_handles(&mut self, handles: &[Handle]) {
        // Buffer de pila de tamaño fijo (`HANDLE_BLOCK_LEN` handles de 8 bytes): ninguna asignación (regla 19).
        // Cada handle entra con `to_le_bytes` de su `to_raw()`, exactamente como `write_u64`, y el bloque se
        // aplana a `&[u8]` sin copiar. Con un slice vacío `chunks` no produce nada y no se escribe ningún byte.
        let mut block = [[0_u8; 8]; HANDLE_BLOCK_LEN];
        for chunk in handles.chunks(HANDLE_BLOCK_LEN) {
            for (slot, handle) in block.iter_mut().zip(chunk) {
                *slot = handle.to_raw().to_le_bytes();
            }
            self.write(block[..chunk.len()].as_flattened());
        }
    }

    /// Entra `value` según su implementación de `HashState`.
    pub fn write_value<T: HashState + ?Sized>(&mut self, value: &T) {
        value.hash_state(self);
    }

    /// Hash de todo lo escrito hasta ahora. No consume ni reinicia: sirve para el hash por sistema del buscador de desincronizaciones.
    #[must_use]
    pub fn finish(&self) -> StateHash {
        StateHash(self.inner.digest())
    }

    /// Vuelve al estado de `new()`.
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

/// Cómo entra un valor en el hash de estado. Cada implementación define una secuencia de bytes fija.
pub trait HashState {
    /// Escribe en `hasher` la secuencia de bytes de este valor.
    fn hash_state(&self, hasher: &mut StateHasher);

    /// Cómo entra un slice de este tipo, sin la longitud delante. Por defecto, `hash_state` de cada elemento en orden;
    /// las implementaciones con vía en bloque lo sobrescriben con una sola escritura que produce exactamente los mismos
    /// bytes. `[T]`, `Vec<T>` y `[T; N]` lo llaman. Es el gancho que sustituye a la especialización, que Rust estable no tiene.
    fn hash_state_slice(items: &[Self], hasher: &mut StateHasher)
    where
        Self: Sized,
    {
        for item in items {
            item.hash_state(hasher);
        }
    }
}

/// `StateHasher::new()`, `write_value(value)`, `finish()`.
#[must_use]
pub fn hash_of<T: HashState + ?Sized>(value: &T) -> StateHash {
    let mut hasher = StateHasher::new();
    hasher.write_value(value);
    hasher.finish()
}

/// La longitud de un slice como `u64`, que es como entra delante de `[T]` y `Vec<T>`. En los objetivos de 64 bits la
/// conversión nunca falla y el compilador la elimina; el pánico solo existiría en un objetivo con punteros más anchos.
fn len_u64(len: usize) -> u64 {
    u64::try_from(len).expect("la longitud de un slice cabe en u64")
}

/// Implementaciones con vía en bloque: el valor entra en little-endian con su anchura y un slice entra con
/// un solo `write(bytemuck::cast_slice(items))`, que da los mismos bytes.
///
/// El valor suelto entra con `bytemuck::bytes_of`, que en una plataforma little-endian (aserción de
/// compilación de arriba) es el entero en little-endian con su anchura, `to_bits()` en los flotantes, 0 o 1 en
/// `bool` y los componentes `x, y, z, w` seguidos en los vectores de `glam`. Así la vía en bloque y la de elemento
/// a elemento son, por construcción, los mismos bytes.
macro_rules! impl_hash_state_pod {
    ($($t:ty),* $(,)?) => {
        $(
            impl HashState for $t {
                fn hash_state(&self, hasher: &mut StateHasher) {
                    hasher.write(bytemuck::bytes_of(self));
                }

                fn hash_state_slice(items: &[Self], hasher: &mut StateHasher) {
                    hasher.write(bytemuck::cast_slice(items));
                }
            }
        )*
    };
}

// Enteros: little-endian con su anchura. `usize` e `isize` quedan fuera porque su anchura depende de la plataforma.
impl_hash_state_pod!(u8, u16, u32, u64, i8, i16, i32, i64);
// `bool`: un byte, 0 o 1.
impl_hash_state_pod!(bool);
// Flotantes: `to_bits()` en little-endian, sin normalizar.
impl_hash_state_pod!(f32, f64);
// Vectores de `glam`: los componentes en orden `x, y, z, w`. `Vec3A` queda fuera porque lleva relleno.
impl_hash_state_pod!(Vec2, Vec3, Vec4, IVec2, IVec3, UVec2, UVec3);

/// Implementaciones que entran como su `u64`, con `hash_state_slice` por defecto.
macro_rules! impl_hash_state_u64_newtype {
    ($($t:ty),* $(,)?) => {
        $(
            impl HashState for $t {
                fn hash_state(&self, hasher: &mut StateHasher) {
                    hasher.write_u64(self.0);
                }
            }
        )*
    };
}

impl_hash_state_u64_newtype!(Tick, Ticks, StateHash, DataHash, Seed);

/// `to_raw()` como `u64`; un slice entra por `write_handles`, la vía en bloque de las columnas de handles.
impl HashState for Handle {
    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.write_u64(self.to_raw());
    }

    fn hash_state_slice(items: &[Self], hasher: &mut StateHasher) {
        hasher.write_handles(items);
    }
}

/// `major`, `minor` y `patch` como `u16`, en ese orden.
impl HashState for Version {
    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.write_u16(self.major);
        hasher.write_u16(self.minor);
        hasher.write_u16(self.patch);
    }
}

/// Un byte 0 si es `None`; un byte 1 y los bytes de `T` si es `Some`.
impl<T: HashState> HashState for Option<T> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        match self {
            None => hasher.write_u8(0),
            Some(value) => {
                hasher.write_u8(1);
                value.hash_state(hasher);
            }
        }
    }
}

/// La longitud como `u64` y después `T::hash_state_slice`.
impl<T: HashState> HashState for [T] {
    fn hash_state(&self, hasher: &mut StateHasher) {
        hasher.write_u64(len_u64(self.len()));
        T::hash_state_slice(self, hasher);
    }
}

/// La longitud como `u64` y después `T::hash_state_slice`, igual que `[T]`.
impl<T: HashState> HashState for Vec<T> {
    fn hash_state(&self, hasher: &mut StateHasher) {
        self.as_slice().hash_state(hasher);
    }
}

/// `T::hash_state_slice`, sin longitud: la anchura ya está en el tipo.
impl<T: HashState, const N: usize> HashState for [T; N] {
    fn hash_state(&self, hasher: &mut StateHasher) {
        T::hash_state_slice(self, hasher);
    }
}

/// Los dos componentes en orden.
impl<A: HashState, B: HashState> HashState for (A, B) {
    fn hash_state(&self, hasher: &mut StateHasher) {
        self.0.hash_state(hasher);
        self.1.hash_state(hasher);
    }
}

/// Los tres componentes en orden.
impl<A: HashState, B: HashState, C: HashState> HashState for (A, B, C) {
    fn hash_state(&self, hasher: &mut StateHasher) {
        self.0.hash_state(hasher);
        self.1.hash_state(hasher);
        self.2.hash_state(hasher);
    }
}

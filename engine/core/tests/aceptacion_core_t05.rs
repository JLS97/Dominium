//! Tests de aceptación de P01-T05: `hash` y `DataHash::of_bytes`.
//!
//! Cubren CA-6, CA-7 y CA-8 de `specs/architecture/engine_core.md`. Solo usan la
//! API pública de `engine_core` y no los modifica el implementador. Toda
//! comparación es entre `finish()` de dos hashers construidos por caminos
//! distintos, o contra la implementación de referencia `xxhash_rust::xxh3::xxh3_64`
//! y los valores publicados de XXH3 que la spec fija en CA-8. Nada sale de la
//! aleatoriedad del sistema: los bytes pseudoaleatorios de CA-6 y CA-8 salen de un
//! xorshift con semilla fija escrito aquí mismo, y las columnas de CA-7 las genera
//! `proptest` con la misma semilla fija, así que cada ejecución repite los mismos casos.

use core::fmt::Debug;
use core::num::NonZeroU32;

use engine_core::data::DataHash;
use engine_core::handle::Handle;
use engine_core::hash::{HashState, StateHash, StateHasher, bytemuck, hash_of};
use engine_core::math::{IVec2, IVec3, UVec2, UVec3, Vec2, Vec3, Vec4};
use engine_core::rng::Seed;
use engine_core::time::{Tick, Ticks};
use engine_core::version::Version;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestError, TestRunner};
use xxhash_rust::xxh3::xxh3_64;

/// Valor publicado de XXH3 de 64 bits para la entrada vacía (CA-8).
const XXH3_EMPTY: u64 = 0x2D06_8005_38D3_94C2;
/// Valor publicado de XXH3 de 64 bits para `b"a"` (CA-8).
const XXH3_A: u64 = 0xE6C6_32B6_1E96_4E1F;
/// Tamaño del bloque de CA-6: 1 MB.
const BLOCK_1MB: usize = 1 << 20;
/// Longitud de las columnas de CA-7.
const COLUMN_LEN: usize = 15_000;
/// Casos de `proptest` por fila de la tabla en CA-7. Cada caso ya es una columna de
/// 15.000 elementos, así que 32 casos son 480.000 elementos por fila, con los valores
/// especiales (ceros negativos y NaN con carga) forzados por peso en la estrategia.
const COLUMN_CASES: u32 = 32;
/// Tope de iteraciones de reducción en CA-7. Las columnas tienen longitud fija, así que
/// `proptest` solo puede simplificar elemento a elemento (15.000 elementos por unos 32 pasos
/// cada uno) y el caso mínimo no aporta nada a una propiedad de igualdad de bytes.
const COLUMN_SHRINK_ITERS: u32 = 200;
/// Semilla del generador pseudoaleatorio de los tests. Cualquier valor distinto de 0 vale.
const GENERATOR_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Generador xorshift64 con semilla fija: bytes pseudoaleatorios reproducibles sin tocar el sistema.
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

    /// Rellena `buf` con bytes pseudoaleatorios.
    fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }

    /// Un `Vec<u8>` de `len` bytes pseudoaleatorios.
    fn bytes(&mut self, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        self.fill(&mut buf);
        buf
    }

    /// Un valor en `0..=max`.
    fn up_to(&mut self, max: usize) -> usize {
        let max = u64::try_from(max).expect("el máximo cabe en u64");
        usize::try_from(self.next_u64() % (max + 1)).expect("el valor cabe en usize")
    }

    /// Un handle con slot pseudoaleatorio y generación pseudoaleatoria no nula.
    fn handle(&mut self) -> Handle {
        let word = self.next_u64();
        // Los 32 bits altos son el slot; la generación es los 32 bits bajos, con el 1 forzado
        // para que nunca sea 0.
        let slot = u32::try_from(word >> 32).expect("32 bits altos");
        let generation = u32::try_from(word & 0xFFFF_FFFF).expect("32 bits bajos") | 1;
        Handle::new(slot, nz(generation))
    }
}

/// Generación no nula a partir de un entero; los tests solo la piden con valores válidos.
fn nz(generation: u32) -> NonZeroU32 {
    NonZeroU32::new(generation).expect("la generación de un handle nunca es 0")
}

/// Longitud como `u64`, que es como entra en el hash la longitud de un slice.
fn len_u64(len: usize) -> u64 {
    u64::try_from(len).expect("la longitud cabe en u64")
}

/// `finish()` de un hasher nuevo sobre el que se ejecuta `write_calls`.
fn hash_with(write_calls: impl FnOnce(&mut StateHasher)) -> StateHash {
    let mut hasher = StateHasher::new();
    write_calls(&mut hasher);
    hasher.finish()
}

/// `finish()` de un hasher nuevo tras un solo `write(bytes)`.
fn hash_bytes(bytes: &[u8]) -> StateHash {
    hash_with(|hasher| hasher.write(bytes))
}

/// Una fila de la tabla de `HashState`: `hash_of(value)` y `write_value(value)` tienen que
/// dar el mismo `finish()` que `write` de los bytes `expected`, escritos a mano según la tabla.
fn assert_row<T: HashState + ?Sized>(label: &str, value: &T, expected: &[u8]) {
    let by_bytes = hash_bytes(expected);
    assert_eq!(hash_of(value), by_bytes, "fila `{label}` por `hash_of`");
    assert_eq!(
        hash_with(|hasher| hasher.write_value(value)),
        by_bytes,
        "fila `{label}` por `write_value`"
    );
}

// CA-6: sobre un bloque de 1 MB de bytes pseudoaleatorios: `write` de todo de una vez,
// `write` en trozos de 1, 7 y 4.096 bytes y `write_u8` byte a byte dan el mismo `finish()`;
// `finish()` tras más escrituras cambia; `reset()` devuelve el hash de `new()`.
#[test]
fn ca_06_hash_estado_independiente_del_troceo() {
    let block = XorShift64::new(GENERATOR_SEED).bytes(BLOCK_1MB);
    let empty = StateHasher::new().finish();

    let whole = hash_bytes(&block);
    assert_ne!(
        whole, empty,
        "el bloque de 1 MB no puede dar el hash del vacío"
    );

    for chunk_len in [1_usize, 7, 4_096] {
        let chunked = hash_with(|hasher| {
            for chunk in block.chunks(chunk_len) {
                hasher.write(chunk);
            }
        });
        assert_eq!(chunked, whole, "trozos de {chunk_len} bytes");
    }

    let byte_by_byte = hash_with(|hasher| {
        for &byte in &block {
            hasher.write_u8(byte);
        }
    });
    assert_eq!(byte_by_byte, whole, "`write_u8` byte a byte");

    // `finish()` no consume ni reinicia: se puede leer varias veces y seguir escribiendo.
    let mut hasher = StateHasher::new();
    hasher.write(&block);
    assert_eq!(hasher.finish(), whole);
    assert_eq!(hasher.finish(), whole, "`finish()` dos veces seguidas");
    hasher.write_u8(0);
    let extended = hasher.finish();
    assert_ne!(extended, whole, "`finish()` tras más escrituras cambia");
    hasher.write(&block);
    assert_ne!(
        hasher.finish(),
        extended,
        "`finish()` tras más escrituras cambia"
    );
    assert_ne!(
        hash_with(|hasher| hasher.write_u8(0)),
        empty,
        "un byte más que el vacío"
    );

    // `reset()` vuelve al estado de `new()`, y desde ahí se reconstruye el mismo hash.
    hasher.reset();
    assert_eq!(
        hasher.finish(),
        empty,
        "`reset()` devuelve el hash de `new()`"
    );
    hasher.write(&block);
    assert_eq!(
        hasher.finish(),
        whole,
        "tras `reset()` el hasher es como uno nuevo"
    );

    // `Default` es `new()`, y un clon a mitad de secuencia sigue igual que el original.
    assert_eq!(StateHasher::default().finish(), empty);
    let (head, tail) = block.split_at(BLOCK_1MB / 2);
    let mut original = StateHasher::new();
    original.write(head);
    let mut cloned = original.clone();
    original.write(tail);
    cloned.write(tail);
    assert_eq!(original.finish(), whole);
    assert_eq!(
        cloned.finish(),
        whole,
        "un clon a mitad de secuencia continúa igual"
    );
}

/// CA-6, primitivos: cada `write_*` entra en little-endian con su anchura exacta.
fn assert_primitive_writes() {
    assert_eq!(hash_with(|h| h.write_u8(7)), hash_bytes(&[0x07]));
    assert_eq!(
        hash_with(|h| h.write_u16(0x1234)),
        hash_bytes(&[0x34, 0x12])
    );
    assert_eq!(
        hash_with(|h| h.write_u32(0x1234_5678)),
        hash_bytes(&[0x78, 0x56, 0x34, 0x12])
    );
    assert_eq!(
        hash_with(|h| h.write_u64(0x0102_0304_0506_0708)),
        hash_bytes(&[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01])
    );
    assert_eq!(hash_with(|h| h.write_i8(-2)), hash_bytes(&[0xFE]));
    assert_eq!(hash_with(|h| h.write_i16(-2)), hash_bytes(&[0xFE, 0xFF]));
    assert_eq!(
        hash_with(|h| h.write_i32(-2)),
        hash_bytes(&[0xFE, 0xFF, 0xFF, 0xFF])
    );
    assert_eq!(
        hash_with(|h| h.write_i64(-2)),
        hash_bytes(&[0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
    );
    assert_eq!(
        hash_with(|h| h.write_f32(1.5)),
        hash_bytes(&[0x00, 0x00, 0xC0, 0x3F])
    );
    assert_eq!(
        hash_with(|h| h.write_f64(1.5)),
        hash_bytes(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F])
    );
    assert_eq!(hash_with(|h| h.write_bool(true)), hash_bytes(&[0x01]));
    assert_eq!(hash_with(|h| h.write_bool(false)), hash_bytes(&[0x00]));
    // Varias escrituras seguidas son la concatenación de sus bytes.
    assert_eq!(
        hash_with(|h| {
            h.write_u8(1);
            h.write_u16(2);
            h.write_u32(3);
        }),
        hash_bytes(&[0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00])
    );
}

/// CA-6, filas de enteros, `bool` y flotantes: el valor en little-endian con su anchura; `to_bits()` en los flotantes.
fn assert_scalar_rows() {
    assert_row("7u8", &7_u8, &[0x07]);
    assert_row("0x1234u16", &0x1234_u16, &[0x34, 0x12]);
    assert_row("0x12345678u32", &0x1234_5678_u32, &[0x78, 0x56, 0x34, 0x12]);
    assert_row(
        "0x0102030405060708u64",
        &0x0102_0304_0506_0708_u64,
        &[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
    );
    assert_row("-2i8", &-2_i8, &[0xFE]);
    assert_row("-2i16", &-2_i16, &[0xFE, 0xFF]);
    assert_row("-2i32", &-2_i32, &[0xFE, 0xFF, 0xFF, 0xFF]);
    assert_row(
        "-2i64",
        &-2_i64,
        &[0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    );
    assert_row("true", &true, &[0x01]);
    assert_row("false", &false, &[0x00]);
    assert_row("1.5f32", &1.5_f32, &[0x00, 0x00, 0xC0, 0x3F]);
    assert_row(
        "1.5f64",
        &1.5_f64,
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF8, 0x3F],
    );
    assert_row("-0.0f32", &-0.0_f32, &[0x00, 0x00, 0x00, 0x80]);
}

/// CA-6, filas de `Tick`, `Ticks`, `StateHash`, `DataHash`, `Seed`, `Handle` y `Version`.
fn assert_newtype_rows() {
    assert_row(
        "Tick(300)",
        &Tick(300),
        &[0x2C, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    );
    let five = [0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_row("Ticks(5)", &Ticks(5), &five);
    assert_row("StateHash(5)", &StateHash(5), &five);
    assert_row("DataHash(5)", &DataHash(5), &five);
    assert_row("Seed(5)", &Seed(5), &five);
    assert_row(
        "Handle::new(5, 7)",
        &Handle::new(5, nz(7)),
        &[0x07, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00],
    );
    assert_row(
        "Handle::new(0x01020304, 0x05060708)",
        &Handle::new(0x0102_0304, nz(0x0506_0708)),
        &[0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01],
    );
    assert_row(
        "Version::new(0, 1, 0)",
        &Version::new(0, 1, 0),
        &[0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
    );
    assert_row(
        "Version::new(0x0102, 0x0304, 0x0506)",
        &Version::new(0x0102, 0x0304, 0x0506),
        &[0x02, 0x01, 0x04, 0x03, 0x06, 0x05],
    );
}

/// CA-6, filas compuestas: `Option`, `[T]`, `Vec<T>`, `[T; N]` y tuplas, también anidadas.
fn assert_composite_rows() {
    assert_row("Some(7u8)", &Some(7_u8), &[0x01, 0x07]);
    assert_row("None::<u8>", &None::<u8>, &[0x00]);
    assert_row(
        "Some(Handle::new(5, 7))",
        &Some(Handle::new(5, nz(7))),
        &[0x01, 0x07, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00],
    );

    let three_bytes = [
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03,
    ];
    assert_row("&[1u8, 2, 3][..]", &[1_u8, 2, 3][..], &three_bytes);
    assert_row("vec![1u8, 2, 3]", &vec![1_u8, 2, 3], &three_bytes);
    let empty_u32: &[u32] = &[];
    assert_row("&[][..] como &[u32]", empty_u32, &[0x00; 8]);
    assert_row("Vec::<u32>::new()", &Vec::<u32>::new(), &[0x00; 8]);
    assert_row("[1u8, 2, 3]", &[1_u8, 2, 3], &[0x01, 0x02, 0x03]);
    assert_row("[1u16, 2]", &[1_u16, 2], &[0x01, 0x00, 0x02, 0x00]);
    assert_row("[0u8; 0]", &[0_u8; 0], &[]);

    assert_row("(1u8, 2u16)", &(1_u8, 2_u16), &[0x01, 0x02, 0x00]);
    assert_row(
        "(1u8, 2u16, 3u32)",
        &(1_u8, 2_u16, 3_u32),
        &[0x01, 0x02, 0x00, 0x03, 0x00, 0x00, 0x00],
    );

    // Anidadas: la regla de cada fila se aplica dentro de la otra.
    assert_row(
        "vec![Some(1u8), None]",
        &vec![Some(1_u8), None],
        &[
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00,
        ],
    );
    assert_row(
        "[Tick(1), Tick(2)]",
        &[Tick(1), Tick(2)],
        &[
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
    );
    assert_row(
        "&[Handle; 2][..]",
        &[Handle::new(5, nz(7)), Handle::new(1, nz(1))][..],
        &[
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x05, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        ],
    );
    assert_row(
        "(Some(1u8), [2u8, 3])",
        &(Some(1_u8), [2_u8, 3]),
        &[0x01, 0x01, 0x02, 0x03],
    );
}

/// CA-6, filas de `glam`: los componentes en orden `x, y, z, w`, cada uno como su escalar.
fn assert_glam_rows() {
    assert_row(
        "Vec2::new(1.0, 2.0)",
        &Vec2::new(1.0, 2.0),
        &[0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x40],
    );
    assert_row(
        "Vec3::new(1.0, 2.0, 3.0)",
        &Vec3::new(1.0, 2.0, 3.0),
        &[
            0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x40,
        ],
    );
    assert_row(
        "Vec4::new(1.0, 2.0, 3.0, 4.0)",
        &Vec4::new(1.0, 2.0, 3.0, 4.0),
        &[
            0x00, 0x00, 0x80, 0x3F, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0x40, 0x40, 0x00, 0x00,
            0x80, 0x40,
        ],
    );
    assert_row(
        "IVec2::new(-1, 2)",
        &IVec2::new(-1, 2),
        &[0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00],
    );
    assert_row(
        "IVec3::new(-1, 2, -3)",
        &IVec3::new(-1, 2, -3),
        &[
            0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xFF, 0xFF, 0xFF,
        ],
    );
    assert_row(
        "UVec2::new(1, 2)",
        &UVec2::new(1, 2),
        &[0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00],
    );
    assert_row(
        "UVec3::new(1, 2, 3)",
        &UVec3::new(1, 2, 3),
        &[
            0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
        ],
    );
    // Una columna de vectores lleva la longitud delante y después los componentes seguidos.
    assert_row(
        "vec![Vec2::new(1.0, 2.0), Vec2::new(-0.0, 1.5)]",
        &vec![Vec2::new(1.0, 2.0), Vec2::new(-0.0, 1.5)],
        &[
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F, 0x00, 0x00,
            0x00, 0x40, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0xC0, 0x3F,
        ],
    );
}

// CA-6: para cada fila de la tabla de `HashState`, con al menos un valor por tipo, `hash_of(&valor)`
// es igual a `write` de la secuencia de bytes escrita a mano en el test según la tabla. Entre otros:
// `7u8` → `07`; `0x1234u16` → `34 12`; `-2i16` → `FE FF`; `true` → `01`; `1.5f32` → `00 00 C0 3F`;
// `1.5f64` → `00 00 00 00 00 00 F8 3F`; `Tick(300)` → `2C 01 00 00 00 00 00 00`; `Ticks(5)`,
// `StateHash(5)`, `DataHash(5)` y `Seed(5)` → `05` y siete `00`; `Handle::new(5, 7)` →
// `07 00 00 00 05 00 00 00`; `Version::new(0, 1, 0)` → `00 00 01 00 00 00`; `Some(7u8)` → `01 07`
// y `None::<u8>` → `00`; `&[1u8, 2, 3][..]` y `vec![1u8, 2, 3]` → `03` y siete `00` y `01 02 03`;
// `&[][..]` como `&[u32]` → `00` por ocho; `[1u8, 2, 3]` → `01 02 03`; `(1u8, 2u16)` → `01 02 00`;
// `(1u8, 2u16, 3u32)` → `01 02 00 03 00 00 00`; `Vec2::new(1.0, 2.0)` → `00 00 80 3F 00 00 00 40`;
// `IVec2::new(-1, 2)` → `FF FF FF FF 02 00 00 00`; `UVec3::new(1, 2, 3)` →
// `01 00 00 00 02 00 00 00 03 00 00 00`; y los `Vec3`, `Vec4`, `IVec3` y `UVec2` análogos.
#[test]
fn ca_06_hash_estado_tabla_fila_a_fila() {
    assert_primitive_writes();
    assert_scalar_rows();
    assert_newtype_rows();
    assert_composite_rows();
    assert_glam_rows();
}

/// Configuración de `proptest` para las columnas de CA-7. La semilla va fija: con
/// `RngSeed::Random` (el valor por defecto de `Config`) el generador se sembraría desde la
/// entropía del sistema y las columnas cambiarían en cada ejecución.
fn column_config() -> Config {
    Config {
        cases: COLUMN_CASES,
        max_shrink_iters: COLUMN_SHRINK_ITERS,
        source_file: Some(file!()),
        rng_seed: RngSeed::Fixed(GENERATOR_SEED),
        ..Config::default()
    }
}

/// Ejecuta `property` sobre columnas de `COLUMN_LEN` elementos de `element`. Si falla, el pánico
/// lleva la fila y los primeros elementos de la columna, no los 15.000.
fn check_column<T, S>(
    label: &str,
    element: S,
    property: impl Fn(Vec<T>) -> Result<(), TestCaseError>,
) where
    T: Debug,
    S: Strategy<Value = T>,
{
    let mut runner = TestRunner::new(column_config());
    match runner.run(&proptest::collection::vec(element, COLUMN_LEN), property) {
        Ok(()) => {}
        Err(TestError::Fail(reason, column)) => {
            let head = &column[..column.len().min(8)];
            panic!("fila `{label}`: {reason}; primeros elementos: {head:?}");
        }
        Err(TestError::Abort(reason)) => panic!("fila `{label}` abortada: {reason}"),
    }
}

/// Vía en bloque de una fila con `hash_state_slice` propio: para una columna `v` de 15.000
/// elementos, `hash_of(&v[..])`, `write_u64(15_000)` más `write(bytemuck::cast_slice(&v))`, y
/// `write_u64(15_000)` seguido de `hash_state` elemento a elemento dan el mismo `finish()`.
/// El límite es `NoUninit`, que es lo que exige `cast_slice`: `bool` lo cumple pero no es `Pod`.
fn check_block_path<T>(label: &str, element: impl Strategy<Value = T>)
where
    T: HashState + bytemuck::NoUninit + Debug,
{
    check_column(label, element, |column: Vec<T>| {
        let len = len_u64(column.len());
        let by_slice = hash_of(&column[..]);
        let by_block = hash_with(|hasher| {
            hasher.write_u64(len);
            hasher.write(bytemuck::cast_slice(&column));
        });
        let one_by_one = hash_with(|hasher| {
            hasher.write_u64(len);
            for item in &column {
                item.hash_state(hasher);
            }
        });
        let by_slice_hook = hash_with(|hasher| {
            hasher.write_u64(len);
            T::hash_state_slice(&column, hasher);
        });
        prop_assert_eq!(by_block, one_by_one, "bloque frente a elemento a elemento");
        prop_assert_eq!(by_slice, by_block, "`hash_of(&v[..])` frente a bloque");
        prop_assert_eq!(
            by_slice_hook,
            by_block,
            "`hash_state_slice` frente a bloque"
        );
        prop_assert_eq!(
            hash_of(&column),
            by_block,
            "`hash_of(&Vec<T>)` frente a bloque"
        );
        Ok(())
    });
}

/// `f32` con todos los patrones de bits, y con peso extra para `0.0`, `-0.0` y NaN de ambos
/// signos con carga arbitraria no nula.
fn any_f32_bits() -> impl Strategy<Value = f32> {
    prop_oneof![
        6 => any::<u32>().prop_map(f32::from_bits),
        1 => Just(0.0_f32),
        1 => Just(-0.0_f32),
        1 => (any::<bool>(), 1_u32..=0x007F_FFFF).prop_map(|(negative, payload)| {
            let sign_and_exponent = if negative { 0xFF80_0000 } else { 0x7F80_0000 };
            f32::from_bits(sign_and_exponent | payload)
        }),
    ]
}

/// `f64` con todos los patrones de bits, y con peso extra para `0.0`, `-0.0` y NaN de ambos
/// signos con carga arbitraria no nula.
fn any_f64_bits() -> impl Strategy<Value = f64> {
    prop_oneof![
        6 => any::<u64>().prop_map(f64::from_bits),
        1 => Just(0.0_f64),
        1 => Just(-0.0_f64),
        1 => (any::<bool>(), 1_u64..=0x000F_FFFF_FFFF_FFFF).prop_map(|(negative, payload)| {
            let sign_and_exponent = if negative {
                0xFFF0_0000_0000_0000
            } else {
                0x7FF0_0000_0000_0000
            };
            f64::from_bits(sign_and_exponent | payload)
        }),
    ]
}

/// Handles con cualquier slot y cualquier generación no nula.
fn any_handle() -> impl Strategy<Value = Handle> {
    (any::<u32>(), 1..=u32::MAX).prop_map(|(slot, generation)| Handle::new(slot, nz(generation)))
}

/// CA-7, primera parte: los flotantes entran bit a bit, sin normalizar.
fn assert_floats_bit_exact() {
    assert_ne!(hash_of(&0.0_f32), hash_of(&-0.0_f32), "0.0 y -0.0 en f32");
    assert_ne!(hash_of(&0.0_f64), hash_of(&-0.0_f64), "0.0 y -0.0 en f64");

    let nan_a = f32::from_bits(0x7FC0_0001);
    let nan_b = f32::from_bits(0x7FC0_0002);
    let nan_negative = f32::from_bits(0xFFC0_0001);
    assert!(nan_a.is_nan() && nan_b.is_nan() && nan_negative.is_nan());
    assert_ne!(
        hash_of(&nan_a),
        hash_of(&nan_b),
        "dos NaN de f32 con distinta carga"
    );
    assert_ne!(
        hash_of(&nan_a),
        hash_of(&nan_negative),
        "dos NaN de f32 con distinto signo"
    );
    assert_eq!(
        hash_of(&nan_a),
        hash_with(|h| h.write_u32(0x7FC0_0001)),
        "NaN es sus bits"
    );

    let nan_c = f64::from_bits(0x7FF8_0000_0000_0001);
    let nan_d = f64::from_bits(0x7FF8_0000_0000_0002);
    assert!(nan_c.is_nan() && nan_d.is_nan());
    assert_ne!(
        hash_of(&nan_c),
        hash_of(&nan_d),
        "dos NaN de f64 con distinta carga"
    );
    assert_eq!(
        hash_of(&nan_c),
        hash_with(|h| h.write_u64(0x7FF8_0000_0000_0001)),
        "NaN es sus bits"
    );

    assert_eq!(
        hash_of(&[1.5_f32, -2.25][..]),
        hash_with(|h| {
            h.write_u64(2);
            h.write_u32(1.5_f32.to_bits());
            h.write_u32((-2.25_f32).to_bits());
        }),
        "`hash_of(&[1.5f32, -2.25][..])`"
    );
}

/// CA-7, handles: `write_handles` es la vía en bloque y da los bytes de `write_u64(to_raw())`
/// por elemento; con un slice vacío no escribe nada; y el mismo resultado se mantiene en todas
/// las longitudes de 0 a 1.024, que cruzan cualquier tamaño razonable del buffer de pila.
fn assert_handles_block_path() {
    // Vacío: deja el hash como estaba, tanto en un hasher nuevo como a mitad de secuencia.
    let empty = StateHasher::new().finish();
    assert_eq!(
        hash_with(|h| h.write_handles(&[])),
        empty,
        "`write_handles(&[])` vacío"
    );
    let mut hasher = StateHasher::new();
    hasher.write_u64(42);
    let before = hasher.finish();
    hasher.write_handles(&[]);
    assert_eq!(
        hasher.finish(),
        before,
        "`write_handles(&[])` a mitad de secuencia"
    );

    // Barrido de longitudes: 0..=1024 cruza los límites de cualquier buffer de pila.
    let mut generator = XorShift64::new(GENERATOR_SEED);
    for len in 0..=1_024_usize {
        let handles: Vec<Handle> = (0..len).map(|_| generator.handle()).collect();
        let by_block = hash_with(|h| h.write_handles(&handles));
        let one_by_one = hash_with(|h| {
            for handle in &handles {
                h.write_u64(handle.to_raw());
            }
        });
        assert_eq!(by_block, one_by_one, "`write_handles` con {len} handles");
    }

    check_column("Handle", any_handle(), |column: Vec<Handle>| {
        let len = len_u64(column.len());
        let by_slice = hash_of(&column[..]);
        let by_block = hash_with(|hasher| {
            hasher.write_u64(len);
            hasher.write_handles(&column);
        });
        let one_by_one = hash_with(|hasher| {
            hasher.write_u64(len);
            for handle in &column {
                hasher.write_u64(handle.to_raw());
            }
        });
        prop_assert_eq!(
            by_block,
            one_by_one,
            "`write_handles` frente a `write_u64` por elemento"
        );
        prop_assert_eq!(
            by_slice,
            by_block,
            "`hash_of(&h[..])` frente a `write_handles`"
        );
        prop_assert_eq!(hash_of(&column), by_block, "`hash_of(&Vec<Handle>)`");
        Ok(())
    });

    check_column(
        "Option<Handle>",
        proptest::option::of(any_handle()),
        |column: Vec<Option<Handle>>| {
            let by_slice = hash_of(&column[..]);
            let by_hand = hash_with(|hasher| {
                hasher.write_u64(len_u64(column.len()));
                for item in &column {
                    if let Some(handle) = item {
                        hasher.write_u8(1);
                        hasher.write_u64(handle.to_raw());
                    } else {
                        hasher.write_u8(0);
                    }
                }
            });
            prop_assert_eq!(by_slice, by_hand, "regla de `Option` escrita a mano");
            prop_assert_eq!(hash_of(&column), by_hand, "`hash_of(&Vec<Option<Handle>>)`");
            Ok(())
        },
    );
}

// CA-7: `hash_of(&0.0f32) != hash_of(&-0.0f32)`; dos NaN con distinta carga dan hashes distintos;
// `hash_of(&[1.5f32, -2.25][..])` es igual a `write_u64(2)`, `write_u32(1.5f32.to_bits())`,
// `write_u32((-2.25f32).to_bits())`. Vía en bloque, para cada fila de la tabla con
// `hash_state_slice` propio (`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `bool`, `f32`,
// `f64`, `Vec2`, `Vec3`, `Vec4`, `IVec2`, `IVec3`, `UVec2`, `UVec3`): para un `Vec<T>` `v` de
// 15.000 elementos generados con `proptest` (en los flotantes, con ceros negativos y NaN
// incluidos), `hash_of(&v[..])`, `write_u64(15_000)` más `write(bytemuck::cast_slice(&v))`, y
// `write_u64(15_000)` seguido de `hash_state` elemento a elemento dan el mismo `finish()`
// (Q-017); para un `Vec<Handle>` `h` de 15.000 elementos, `hash_of(&h[..])`, `write_u64(15_000)`
// más `write_handles(&h)`, y `write_u64(15_000)` seguido de `write_u64(x.to_raw())` por elemento
// dan el mismo `finish()`; `write_handles(&[])` deja el hash como estaba; y para un
// `Vec<Option<Handle>>` de 15.000 elementos, `hash_of` es igual a la secuencia de la regla de
// `Option` escrita a mano.
#[test]
fn ca_07_hash_estado_flotantes_bit_a_bit_y_via_en_bloque() {
    assert_floats_bit_exact();

    check_block_path("u8", any::<u8>());
    check_block_path("u16", any::<u16>());
    check_block_path("u32", any::<u32>());
    check_block_path("u64", any::<u64>());
    check_block_path("i8", any::<i8>());
    check_block_path("i16", any::<i16>());
    check_block_path("i32", any::<i32>());
    check_block_path("i64", any::<i64>());
    check_block_path("bool", any::<bool>());
    check_block_path("f32", any_f32_bits());
    check_block_path("f64", any_f64_bits());
    check_block_path(
        "Vec2",
        (any_f32_bits(), any_f32_bits()).prop_map(|(x, y)| Vec2::new(x, y)),
    );
    check_block_path(
        "Vec3",
        (any_f32_bits(), any_f32_bits(), any_f32_bits()).prop_map(|(x, y, z)| Vec3::new(x, y, z)),
    );
    check_block_path(
        "Vec4",
        (
            any_f32_bits(),
            any_f32_bits(),
            any_f32_bits(),
            any_f32_bits(),
        )
            .prop_map(|(x, y, z, w)| Vec4::new(x, y, z, w)),
    );
    check_block_path(
        "IVec2",
        (any::<i32>(), any::<i32>()).prop_map(|(x, y)| IVec2::new(x, y)),
    );
    check_block_path(
        "IVec3",
        (any::<i32>(), any::<i32>(), any::<i32>()).prop_map(|(x, y, z)| IVec3::new(x, y, z)),
    );
    check_block_path(
        "UVec2",
        (any::<u32>(), any::<u32>()).prop_map(|(x, y)| UVec2::new(x, y)),
    );
    check_block_path(
        "UVec3",
        (any::<u32>(), any::<u32>(), any::<u32>()).prop_map(|(x, y, z)| UVec3::new(x, y, z)),
    );

    assert_handles_block_path();
}

// CA-8: `StateHasher::new().finish() == StateHash(0x2D06_8005_38D3_94C2)` y
// `DataHash::of_bytes(&[]) == DataHash(0x2D06_8005_38D3_94C2)`, el valor publicado de XXH3 de 64
// bits para la entrada vacía; `DataHash::of_bytes(b"a") == DataHash(0xE6C6_32B6_1E96_4E1F)`; para
// 100 bloques pseudoaleatorios de longitud entre 0 y 10.000 bytes, `StateHasher` con `write` y
// `DataHash::of_bytes` coinciden con `xxhash_rust::xxh3::xxh3_64` sobre los mismos bytes;
// `Display` de ambos es el hexadecimal de 16 dígitos en minúsculas del `u64`
// (`2d06800538d394c2` para el vacío); y `postcard::to_allocvec` de `Seed(1)` da `[0x01]`, el de
// `DataHash(u64::MAX)` y el de `StateHash(u64::MAX)` dan nueve `0xFF` seguidos de `0x01`, y los
// tres vuelven idénticos con `postcard::from_bytes`.
#[test]
fn ca_08_hash_estado_y_datos_coinciden_con_xxh3_de_referencia() {
    // Valores publicados de XXH3.
    assert_eq!(StateHasher::new().finish(), StateHash(XXH3_EMPTY));
    assert_eq!(DataHash::of_bytes(&[]), DataHash(XXH3_EMPTY));
    assert_eq!(DataHash::of_bytes(b"a"), DataHash(XXH3_A));
    assert_eq!(hash_bytes(b"a"), StateHash(XXH3_A));

    // 100 bloques de 0 a 10.000 bytes, con los dos extremos forzados, contra la referencia.
    let mut generator = XorShift64::new(GENERATOR_SEED);
    for index in 0..100_usize {
        let len = match index {
            0 => 0,
            1 => 10_000,
            _ => generator.up_to(10_000),
        };
        let bytes = generator.bytes(len);
        let reference = xxh3_64(&bytes);
        assert_eq!(
            hash_bytes(&bytes),
            StateHash(reference),
            "`StateHasher` en el bloque {index} de {len} bytes"
        );
        assert_eq!(
            DataHash::of_bytes(&bytes),
            DataHash(reference),
            "`DataHash::of_bytes` en el bloque {index} de {len} bytes"
        );
        // En streaming, troceado, sigue siendo el mismo xxh3.
        let streamed = hash_with(|hasher| {
            for chunk in bytes.chunks(1_000) {
                hasher.write(chunk);
            }
        });
        assert_eq!(
            streamed,
            StateHash(reference),
            "`StateHasher` en streaming en el bloque {index} de {len} bytes"
        );
    }

    // `Display`: 16 dígitos hexadecimales en minúsculas, con ceros a la izquierda.
    assert_eq!(StateHasher::new().finish().to_string(), "2d06800538d394c2");
    assert_eq!(DataHash::of_bytes(&[]).to_string(), "2d06800538d394c2");
    assert_eq!(StateHash(XXH3_A).to_string(), "e6c632b61e964e1f");
    assert_eq!(DataHash(XXH3_A).to_string(), "e6c632b61e964e1f");
    assert_eq!(StateHash(0).to_string(), "0000000000000000");
    assert_eq!(DataHash(0).to_string(), "0000000000000000");
    assert_eq!(StateHash(0xABC).to_string(), "0000000000000abc");
    assert_eq!(DataHash(0xABC).to_string(), "0000000000000abc");
    assert_eq!(StateHash(u64::MAX).to_string(), "ffffffffffffffff");
    assert_eq!(DataHash(u64::MAX).to_string(), "ffffffffffffffff");

    // `postcard`: varint de `u64`, y vuelta idéntica.
    let seed_bytes = postcard::to_allocvec(&Seed(1)).expect("codificar `Seed`");
    assert_eq!(seed_bytes, vec![0x01]);
    assert_eq!(
        postcard::from_bytes::<Seed>(&seed_bytes).expect("decodificar `Seed`"),
        Seed(1)
    );

    let nine_ff_then_one = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
    let data_bytes = postcard::to_allocvec(&DataHash(u64::MAX)).expect("codificar `DataHash`");
    assert_eq!(data_bytes, nine_ff_then_one);
    assert_eq!(
        postcard::from_bytes::<DataHash>(&data_bytes).expect("decodificar `DataHash`"),
        DataHash(u64::MAX)
    );
    let state_bytes = postcard::to_allocvec(&StateHash(u64::MAX)).expect("codificar `StateHash`");
    assert_eq!(state_bytes, nine_ff_then_one);
    assert_eq!(
        postcard::from_bytes::<StateHash>(&state_bytes).expect("decodificar `StateHash`"),
        StateHash(u64::MAX)
    );
}

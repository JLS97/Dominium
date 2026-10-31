//! Tests de aceptación de P01-T08: `save`.
//!
//! Cubren CA-11 a CA-14 de `specs/architecture/engine_core.md`. Solo usan la API
//! pública de `engine_core` y no los modifica el implementador.
//!
//! Todo ocurre en memoria: ningún test toca el disco. El único vector dorado es el de
//! 16 bytes de "Datos y formatos"; el resto se compara entre caminos (`encode` frente a
//! `encode_to_vec`, ida y vuelta) o con la referencia construida a mano según la tabla
//! de posiciones: `MAGIC`, `ENVELOPE_VERSION`, `postcard::to_allocvec` de la cabecera y
//! `postcard::to_allocvec` del cuerpo. Los bytes pseudoaleatorios de la struct de 3 MB
//! salen de un xorshift64 con semilla fija escrito aquí mismo; `proptest` va con semilla
//! fija y sin archivo de regresiones. Nada sale de la aleatoriedad del sistema.
//!
//! Sobre los errores: `SaveError` deriva `PartialEq`, así que donde la spec fija los campos
//! se compara el error entero con `assert_eq!`. `Corrupt(String)` y `Encode(String)` llevan
//! el texto de `postcard`, que no se fija: se comprueban con `matches!`.

use core::fmt;
use core::num::NonZeroU32;

use engine_core::data::DataHash;
use engine_core::handle::Handle;
use engine_core::save::{
    ENVELOPE_VERSION, Expected, Header, Kind, MAGIC, SaveError, decode, decode_body, decode_header,
    encode, encode_to_vec,
};
use engine_core::version::{GAME_VERSION, Version};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};
use serde::{Deserialize, Serialize, Serializer};

/// Semilla fija del xorshift y de `proptest`. Misma constante que en T05 y T06.
const GENERATOR_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Cota de la cabecera completa, de la posición 0 al inicio del cuerpo: 8 + 1 + 1 + 5 + 9 + 10.
const MAX_HEADER_LEN: usize = 34;

/// Cabecera del ejemplo fijado de "Datos y formatos".
const GOLDEN_HEADER: Header = Header {
    kind: Kind::Replay,
    format_version: 300,
    game_version: Version::new(0, 1, 0),
    data_hash: DataHash(1),
};

/// Los 16 bytes del ejemplo fijado: `MAGIC`, `ENVELOPE_VERSION`, `Replay` (índice 1), 300 en
/// varint (`AC 02`), `0.1.0` (`00 01 00`), `DataHash(1)` (`01`) y el cuerpo `()`, que no ocupa.
const GOLDEN_BYTES: [u8; 16] = [
    0x50, 0x4C, 0x55, 0x53, 0x55, 0x4C, 0x54, 0x52, 0x01, 0x01, 0xAC, 0x02, 0x00, 0x01, 0x00, 0x01,
];

/// Cabecera con todos los campos en su máximo: la que más bytes ocupa en `postcard`.
const WIDEST_HEADER: Header = Header {
    kind: Kind::Replay,
    format_version: u32::MAX,
    game_version: Version::new(u16::MAX, u16::MAX, u16::MAX),
    data_hash: DataHash(u64::MAX),
};

/// Elementos por columna de la struct de 3 MB: cuatro columnas de 4 bytes y una de 8 bytes,
/// 131.072 elementos cada una, son 4 × 512 KiB + 1 MiB = 3 MiB en memoria.
const COLUMN_LEN: usize = 131_072;

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

    /// Los 32 bits bajos de la siguiente palabra.
    fn next_u32(&mut self) -> u32 {
        let [a, b, c, d, ..] = self.next_u64().to_le_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    /// Un `f32` finito y sin NaN, para que la ida y vuelta se pueda comparar con `==`.
    fn next_f32(&mut self) -> f32 {
        let [a, b, ..] = self.next_u64().to_le_bytes();
        f32::from(u16::from_le_bytes([a, b])) / 16.0 - 2_048.0
    }

    /// Un handle con generación nunca nula.
    fn next_handle(&mut self) -> Handle {
        let slot = self.next_u32();
        let generation = NonZeroU32::new(self.next_u32()).unwrap_or(Handle::FIRST_GENERATION);
        Handle::new(slot, generation)
    }
}

/// Struct de prueba de unos 3 MB: varias columnas SoA como las que guardará `engine_sim`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Columns {
    position_x: Vec<f32>,
    position_y: Vec<f32>,
    health: Vec<u32>,
    unit_kind: Vec<u32>,
    target: Vec<Handle>,
}

impl Columns {
    /// Rellena las cinco columnas con el xorshift de semilla fija.
    fn generate(seed: u64) -> Columns {
        let mut rng = XorShift64::new(seed);
        Columns {
            position_x: (0..COLUMN_LEN).map(|_| rng.next_f32()).collect(),
            position_y: (0..COLUMN_LEN).map(|_| rng.next_f32()).collect(),
            health: (0..COLUMN_LEN).map(|_| rng.next_u32()).collect(),
            unit_kind: (0..COLUMN_LEN).map(|_| rng.next_u32()).collect(),
            target: (0..COLUMN_LEN).map(|_| rng.next_handle()).collect(),
        }
    }

    /// Tamaño en memoria de las columnas: elementos por tamaño del elemento, sin contar los `Vec`.
    fn bytes_in_memory(&self) -> usize {
        self.position_x.len() * size_of::<f32>()
            + self.position_y.len() * size_of::<f32>()
            + self.health.len() * size_of::<u32>()
            + self.unit_kind.len() * size_of::<u32>()
            + self.target.len() * size_of::<Handle>()
    }
}

/// Cuerpo cuyo `Serialize` siempre falla, para provocar `SaveError::Encode`.
struct FailingBody;

impl Serialize for FailingBody {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "cuerpo de prueba que no se deja serializar",
        ))
    }
}

/// Versión 1 del cuerpo de CA-13.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct V1 {
    a: u32,
}

/// Versión 2 del cuerpo de CA-13: añade `b`.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct V2 {
    a: u32,
    b: u16,
}

/// Valor que la migración da a `b` cuando el cuerpo viene de `V1`.
const MIGRATED_B: u16 = 7;

impl From<V1> for V2 {
    fn from(old: V1) -> V2 {
        V2 {
            a: old.a,
            b: MIGRATED_B,
        }
    }
}

/// Handle con generación no nula, para escribir los casos sin ruido.
fn handle(slot: u32, generation: u32) -> Handle {
    Handle::new(
        slot,
        NonZeroU32::new(generation).expect("la generación de prueba no es 0"),
    )
}

/// La `Expected` que acepta exactamente `header`.
fn expected_for(header: &Header) -> Expected {
    Expected {
        kind: header.kind,
        format_version: header.format_version,
        data_hash: header.data_hash,
    }
}

/// Referencia construida a mano según la tabla de "Datos y formatos": `MAGIC`,
/// `ENVELOPE_VERSION`, la cabecera en `postcard` y el cuerpo en `postcard`.
fn reference_envelope<T: Serialize + ?Sized>(header: &Header, body: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&MAGIC);
    bytes.push(ENVELOPE_VERSION);
    bytes.extend_from_slice(&postcard::to_allocvec(header).expect("cabecera en postcard"));
    bytes.extend_from_slice(&postcard::to_allocvec(body).expect("cuerpo en postcard"));
    bytes
}

/// Longitud de la cabecera completa (de la posición 0 al inicio del cuerpo), según la tabla.
fn reference_header_len(header: &Header) -> usize {
    MAGIC.len()
        + 1
        + postcard::to_allocvec(header)
            .expect("cabecera en postcard")
            .len()
}

/// Longitud del cuerpo en `postcard`, según la tabla.
fn reference_body_len<T: Serialize + ?Sized>(body: &T) -> usize {
    postcard::to_allocvec(body)
        .expect("cuerpo en postcard")
        .len()
}

/// Afirma que `result` es `Corrupt`, sin fijar el texto de `postcard`.
fn assert_corrupt<T: fmt::Debug>(result: &Result<T, SaveError>, context: &str) {
    assert!(
        matches!(result, Err(SaveError::Corrupt(_))),
        "{context}: se esperaba Corrupt y se obtuvo {result:?}"
    );
}

/// Afirma que `result` es `Encode`, sin fijar el texto de `postcard`.
fn assert_encode_error<T: fmt::Debug>(result: &Result<T, SaveError>, context: &str) {
    assert!(
        matches!(result, Err(SaveError::Encode(_))),
        "{context}: se esperaba Encode y se obtuvo {result:?}"
    );
}

/// Cabecera y cuerpo de partida de CA-12: un sobre válido con `game_version` igual a la del
/// juego y un cuerpo `Vec<Handle>` de ocho elementos (17 bytes en `postcard`).
fn valid_envelope() -> (Header, Vec<Handle>, Vec<u8>) {
    let header = Header {
        kind: Kind::Replay,
        format_version: 300,
        game_version: GAME_VERSION,
        data_hash: DataHash(0x1234),
    };
    let body: Vec<Handle> = (1..=8).map(|i| handle(i, i)).collect();
    let bytes = encode_to_vec(&header, &body).expect("codificar el sobre válido de partida");
    (header, body, bytes)
}

// CA-11: El ejemplo fijado de "Datos y formatos" (`Kind::Replay`, `format_version: 300`,
// `game_version: Version::new(0, 1, 0)`, `DataHash(1)`, cuerpo `()`) codifica exactamente a
// los 16 bytes `50 4C 55 53 55 4C 54 52 01 01 AC 02 00 01 00 01`, y `decode::<()>` de esos
// 16 bytes con la `Expected` correspondiente devuelve esa cabecera y `()`. Con una struct de
// prueba de unos 3 MB (varias columnas `Vec<f32>`, `Vec<u32>` y `Vec<Handle>`): `encode`
// seguido de `decode` con la `Expected` correspondiente devuelve la misma cabecera y el mismo
// cuerpo; `decode_header` devuelve la cabecera y un resto cuya longitud es la del cuerpo; la
// cabecera completa ocupa ≤ 34 bytes para cualquier `Header`, comprobado con
// `format_version: u32::MAX`, `Version::new(u16::MAX, u16::MAX, u16::MAX)` y
// `DataHash(u64::MAX)`; y el archivo entero pesa menos del doble del tamaño en memoria de las
// columnas.
#[test]
fn ca_11_sobre_ida_y_vuelta_y_cabecera_fijada() {
    // El ejemplo fijado, por los dos caminos de codificación.
    let golden = encode_to_vec(&GOLDEN_HEADER, &()).expect("codificar el ejemplo fijado");
    assert_eq!(golden, GOLDEN_BYTES, "encode_to_vec del ejemplo fijado");
    let mut out = vec![0xEE; 40];
    encode(&GOLDEN_HEADER, &(), &mut out).expect("encode del ejemplo fijado");
    assert_eq!(
        out, GOLDEN_BYTES,
        "encode vacía `out` y escribe el ejemplo fijado"
    );

    let (header, body) = decode::<()>(&GOLDEN_BYTES, &expected_for(&GOLDEN_HEADER))
        .expect("decodificar el ejemplo fijado");
    assert_eq!(header, GOLDEN_HEADER);
    let () = body;
    let (header, rest) = decode_header(&GOLDEN_BYTES).expect("cabecera del ejemplo fijado");
    assert_eq!(header, GOLDEN_HEADER);
    assert!(
        rest.is_empty(),
        "el cuerpo `()` no ocupa bytes: {rest:02X?}"
    );

    // La struct de 3 MB: ida y vuelta con `encode` y con `encode_to_vec`.
    let columns = Columns::generate(GENERATOR_SEED);
    assert_eq!(
        columns.bytes_in_memory(),
        3 * 1024 * 1024,
        "la struct ocupa 3 MiB en memoria"
    );
    let columns_header = Header {
        kind: Kind::Save,
        format_version: 7,
        game_version: GAME_VERSION,
        data_hash: DataHash(0xDEAD_BEEF_CAFE_F00D),
    };
    let expected = expected_for(&columns_header);

    let mut encoded = Vec::new();
    encode(&columns_header, &columns, &mut encoded).expect("encode de la struct de 3 MB");
    let encoded_to_vec =
        encode_to_vec(&columns_header, &columns).expect("encode_to_vec de la struct de 3 MB");
    assert_eq!(
        encoded, encoded_to_vec,
        "encode y encode_to_vec producen los mismos bytes"
    );
    assert_eq!(
        encoded,
        reference_envelope(&columns_header, &columns),
        "el sobre sigue la tabla de posiciones de la spec"
    );

    let (decoded_header, decoded_body) =
        decode::<Columns>(&encoded, &expected).expect("decode de la struct de 3 MB");
    assert_eq!(decoded_header, columns_header);
    assert_eq!(decoded_body, columns);

    // `decode_header` devuelve el resto intacto: tan largo como el cuerpo y con sus bytes.
    let (decoded_header, rest) = decode_header(&encoded).expect("decode_header de 3 MB");
    assert_eq!(decoded_header, columns_header);
    assert_eq!(
        rest.len(),
        reference_body_len(&columns),
        "el resto mide lo que el cuerpo"
    );
    assert_eq!(
        encoded.len() - rest.len(),
        reference_header_len(&columns_header),
        "la cabecera completa mide lo que dice la tabla"
    );
    assert_eq!(
        rest,
        postcard::to_allocvec(&columns).expect("cuerpo en postcard"),
        "el resto es el cuerpo en postcard sin tocar"
    );
    let body_again: Columns = decode_body(rest).expect("decode_body del resto");
    assert_eq!(body_again, columns);

    // La cabecera completa ocupa ≤ 34 bytes con todos los campos en su máximo.
    let widest = encode_to_vec(&WIDEST_HEADER, &()).expect("codificar la cabecera más ancha");
    assert!(
        widest.len() <= MAX_HEADER_LEN,
        "la cabecera más ancha ocupa {} bytes, más de {MAX_HEADER_LEN}",
        widest.len()
    );
    let (header, rest) = decode_header(&widest).expect("decodificar la cabecera más ancha");
    assert_eq!(header, WIDEST_HEADER);
    assert_eq!(
        rest,
        [],
        "la cabecera más ancha con cuerpo `()` no deja resto"
    );
    assert!(reference_header_len(&columns_header) <= MAX_HEADER_LEN);
    assert!(reference_header_len(&GOLDEN_HEADER) <= MAX_HEADER_LEN);

    // El archivo entero pesa menos del doble del tamaño en memoria de las columnas.
    assert!(
        encoded.len() < 2 * columns.bytes_in_memory(),
        "el archivo pesa {} bytes y las columnas {} en memoria",
        encoded.len(),
        columns.bytes_in_memory()
    );
}

proptest! {
    #![proptest_config(Config {
        cases: 256,
        source_file: Some(file!()),
        rng_seed: RngSeed::Fixed(GENERATOR_SEED),
        failure_persistence: None,
        ..Config::default()
    })]

    // CA-11 (propiedad): para cualquier `Header` y un cuerpo `Vec<u32>` pequeño, el sobre sigue
    // la tabla de posiciones, la cabecera completa ocupa ≤ 34 bytes, `decode_header` devuelve
    // la cabecera y el cuerpo en `postcard` sin tocar, y `decode` con la `Expected`
    // correspondiente devuelve la misma cabecera y el mismo cuerpo.
    #[test]
    fn ca_11_cabecera_cualquiera_ida_y_vuelta_propiedad(
        kind in prop_oneof![Just(Kind::Save), Just(Kind::Replay)],
        format_version in any::<u32>(),
        major in any::<u16>(),
        minor in any::<u16>(),
        patch in any::<u16>(),
        data_hash in any::<u64>(),
        body in proptest::collection::vec(any::<u32>(), 0..16),
    ) {
        let header = Header {
            kind,
            format_version,
            game_version: Version::new(major, minor, patch),
            data_hash: DataHash(data_hash),
        };
        let bytes = encode_to_vec(&header, &body).expect("codificar");
        prop_assert_eq!(&bytes, &reference_envelope(&header, &body));

        let (decoded_header, rest) = decode_header(&bytes).expect("decode_header");
        prop_assert_eq!(decoded_header, header);
        prop_assert!(bytes.len() - rest.len() <= MAX_HEADER_LEN);
        prop_assert_eq!(rest, &postcard::to_allocvec(&body).expect("cuerpo en postcard")[..]);

        let (decoded_header, decoded_body) =
            decode::<Vec<u32>>(&bytes, &expected_for(&header)).expect("decode");
        prop_assert_eq!(decoded_header, header);
        prop_assert_eq!(decoded_body, body);
    }
}

// CA-12: Partiendo de un sobre válido: cambiar un byte de `MAGIC` da `BadMagic`; un archivo
// de 7 bytes da `BadMagic`; un archivo de exactamente 8 bytes iguales a `MAGIC` da `Corrupt`;
// poner el byte de la posición 8 a `2` da `UnsupportedEnvelope { found: 2 }`; un `kind` con
// índice 7 en la cabecera da `Corrupt`; codificar con `Kind::Save` y exigir `Kind::Replay` da
// `UnexpectedKind`; `format_version` distinta da `UnsupportedFormat`; `data_hash` distinto da
// `DataHashMismatch` con los dos valores (regla 18); una `game_version` distinta de
// `GAME_VERSION` no es error; añadir un byte al final da `TrailingBytes { count: 1 }`; truncar
// el cuerpo a la mitad da `Corrupt`; truncar dentro de la cabecera da `Corrupt`; un cuerpo
// `Vec<Handle>` cuyos bytes llevan un handle con generación 0 da `Corrupt`; un cuerpo cuyo
// `Serialize` devuelve error da `Encode`; y con una cabecera que falla en todo, `decode`
// devuelve `UnexpectedKind` antes que `UnsupportedFormat` y este antes que `DataHashMismatch`.
// La spec fija un solo test para todos los casos de CA-12, así que aquí van todos.
#[expect(
    clippy::too_many_lines,
    reason = "un test por criterio, como fija la spec"
)]
#[test]
fn ca_12_sobre_rechaza_magia_tipo_version_y_hash_de_datos() {
    let (header, body, bytes) = valid_envelope();
    let expected = expected_for(&header);
    let header_len = reference_header_len(&header);
    let body_len = reference_body_len(&body);
    assert_eq!(
        bytes.len(),
        header_len + body_len,
        "el sobre de partida sigue la tabla"
    );

    // El sobre de partida se carga tal cual: un cuerpo que termina justo donde acaba el
    // archivo no da `TrailingBytes`.
    assert_eq!(
        decode::<Vec<Handle>>(&bytes, &expected),
        Ok((header, body.clone()))
    );

    // Cambiar cualquiera de los 8 bytes de `MAGIC` da `BadMagic`.
    for position in 0..MAGIC.len() {
        let mut damaged = bytes.clone();
        damaged[position] ^= 0xFF;
        assert_eq!(
            decode::<Vec<Handle>>(&damaged, &expected),
            Err(SaveError::BadMagic),
            "byte {position} de MAGIC cambiado"
        );
        assert_eq!(
            decode_header(&damaged).map(|(h, _)| h),
            Err(SaveError::BadMagic),
            "byte {position} de MAGIC cambiado, en decode_header"
        );
    }

    // Un archivo de 7 bytes da `BadMagic`, aunque sean los 7 primeros de `MAGIC`.
    assert_eq!(
        decode::<Vec<Handle>>(&bytes[..7], &expected),
        Err(SaveError::BadMagic)
    );
    assert_eq!(
        decode_header(&bytes[..7]).map(|(h, _)| h),
        Err(SaveError::BadMagic)
    );
    assert_eq!(
        decode::<Vec<Handle>>(&[], &expected),
        Err(SaveError::BadMagic)
    );

    // Exactamente los 8 bytes de `MAGIC`, sin byte de versión del sobre: `Corrupt`.
    assert_corrupt(&decode::<Vec<Handle>>(&MAGIC, &expected), "solo MAGIC");
    assert_corrupt(&decode_header(&MAGIC), "solo MAGIC, en decode_header");

    // Byte de la posición 8 a 2: `UnsupportedEnvelope { found: 2 }`.
    let mut other_envelope = bytes.clone();
    other_envelope[8] = 2;
    assert_eq!(
        decode::<Vec<Handle>>(&other_envelope, &expected),
        Err(SaveError::UnsupportedEnvelope { found: 2 })
    );
    assert_eq!(
        decode_header(&other_envelope).map(|(h, _)| h),
        Err(SaveError::UnsupportedEnvelope { found: 2 })
    );

    // `kind` con índice 7: la posición 9 es el primer byte de la cabecera, el índice de variante.
    let mut unknown_kind = bytes.clone();
    unknown_kind[9] = 7;
    assert_corrupt(
        &decode::<Vec<Handle>>(&unknown_kind, &expected),
        "kind con índice 7",
    );
    assert_corrupt(
        &decode_header(&unknown_kind),
        "kind con índice 7, en decode_header",
    );

    // Codificar con `Kind::Save` y exigir `Kind::Replay`: `UnexpectedKind`.
    let save_header = Header {
        kind: Kind::Save,
        ..header
    };
    let save_bytes = encode_to_vec(&save_header, &body).expect("codificar como Save");
    assert_eq!(
        decode::<Vec<Handle>>(&save_bytes, &expected),
        Err(SaveError::UnexpectedKind {
            expected: Kind::Replay,
            found: Kind::Save,
        })
    );
    assert_eq!(
        save_header.check(&expected),
        Err(SaveError::UnexpectedKind {
            expected: Kind::Replay,
            found: Kind::Save,
        })
    );

    // `format_version` distinta: `UnsupportedFormat` con las dos versiones.
    let other_format = Expected {
        format_version: 301,
        ..expected
    };
    assert_eq!(
        decode::<Vec<Handle>>(&bytes, &other_format),
        Err(SaveError::UnsupportedFormat {
            expected: 301,
            found: 300,
        })
    );

    // `data_hash` distinto: `DataHashMismatch` con los dos valores (regla 18).
    let other_data = Expected {
        data_hash: DataHash(0x5678),
        ..expected
    };
    assert_eq!(
        decode::<Vec<Handle>>(&bytes, &other_data),
        Err(SaveError::DataHashMismatch {
            expected: DataHash(0x5678),
            found: DataHash(0x1234),
        })
    );

    // Una `game_version` distinta de `GAME_VERSION` no es error: es informativa.
    let other_game_version = Version::new(9, 9, 9);
    assert_ne!(other_game_version, GAME_VERSION);
    let old_header = Header {
        game_version: other_game_version,
        ..header
    };
    let old_bytes = encode_to_vec(&old_header, &body).expect("codificar con otra game_version");
    assert_eq!(
        decode::<Vec<Handle>>(&old_bytes, &expected),
        Ok((old_header, body.clone())),
        "otra game_version se carga y se devuelve tal cual"
    );
    assert_eq!(old_header.check(&expected), Ok(()));

    // Un byte de más al final: `TrailingBytes { count: 1 }`, también en `decode_body`.
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        decode::<Vec<Handle>>(&trailing, &expected),
        Err(SaveError::TrailingBytes { count: 1 })
    );
    assert_eq!(
        decode_body::<Vec<Handle>>(&trailing[header_len..]),
        Err(SaveError::TrailingBytes { count: 1 })
    );
    let mut trailing_three = bytes.clone();
    trailing_three.extend_from_slice(&[0, 0, 0]);
    assert_eq!(
        decode::<Vec<Handle>>(&trailing_three, &expected),
        Err(SaveError::TrailingBytes { count: 3 })
    );

    // Truncar el cuerpo a la mitad: `Corrupt`.
    let half_body = &bytes[..header_len + body_len / 2];
    assert_corrupt(
        &decode::<Vec<Handle>>(half_body, &expected),
        "cuerpo a la mitad",
    );
    assert_corrupt(
        &decode_body::<Vec<Handle>>(&half_body[header_len..]),
        "cuerpo a la mitad, en decode_body",
    );
    // `decode_header` no mira el cuerpo: devuelve la cabecera y lo que queda.
    let (truncated_header, rest) =
        decode_header(half_body).expect("decode_header no lee el cuerpo");
    assert_eq!(truncated_header, header);
    assert_eq!(rest.len(), body_len / 2);

    // Truncar dentro de la cabecera: `Corrupt`, por los dos caminos.
    assert!(
        header_len > 12,
        "el corte a 12 bytes queda dentro de la cabecera"
    );
    let inside_header = &bytes[..12];
    assert_corrupt(
        &decode::<Vec<Handle>>(inside_header, &expected),
        "cabecera a medias",
    );
    assert_corrupt(
        &decode_header(inside_header),
        "cabecera a medias, en decode_header",
    );
    let before_body = &bytes[..header_len - 1];
    assert_corrupt(&decode_header(before_body), "cabecera sin su último byte");

    // Un handle con generación 0 en el cuerpo: `Corrupt`. `Handle` no deja construirlo, así que
    // los bytes salen de una struct espejo `(u32, u32)`, que `postcard` codifica igual que
    // `Handle` (sus dos campos en orden, como varint). Primero se comprueba que el espejo con
    // generación 1 sí se lee como `Handle`, para que el `Corrupt` sea por la generación 0.
    let mirror_ok: Vec<(u32, u32)> = vec![(5, 1), (6, 2)];
    let mirror_ok_bytes = encode_to_vec(&header, &mirror_ok).expect("codificar el espejo");
    assert_eq!(
        decode::<Vec<Handle>>(&mirror_ok_bytes, &expected),
        Ok((header, vec![handle(5, 1), handle(6, 2)])),
        "el espejo (u32, u32) con generación no nula se lee como Handle"
    );
    let mirror_zero: Vec<(u32, u32)> = vec![(5, 1), (6, 0)];
    let mirror_zero_bytes = encode_to_vec(&header, &mirror_zero).expect("codificar el espejo");
    assert_corrupt(
        &decode::<Vec<Handle>>(&mirror_zero_bytes, &expected),
        "handle con generación 0",
    );
    assert_corrupt(
        &decode_body::<Vec<Handle>>(&mirror_zero_bytes[header_len..]),
        "handle con generación 0, en decode_body",
    );

    // Un cuerpo cuyo `Serialize` devuelve error: `Encode`, por los dos caminos.
    assert_encode_error(&encode_to_vec(&header, &FailingBody), "encode_to_vec");
    let mut out = Vec::new();
    assert_encode_error(&encode(&header, &FailingBody, &mut out), "encode");

    // Una cabecera que falla en todo: `UnexpectedKind` antes que `UnsupportedFormat` y este
    // antes que `DataHashMismatch`. Se corrige un campo cada vez y el error pasa al siguiente.
    let all_wrong = Expected {
        kind: Kind::Save,
        format_version: 301,
        data_hash: DataHash(0x5678),
    };
    let wrong_format_and_hash = Expected {
        kind: Kind::Replay,
        ..all_wrong
    };
    let wrong_hash = Expected {
        format_version: 300,
        ..wrong_format_and_hash
    };
    for (label, exp, err) in [
        (
            "todo mal",
            all_wrong,
            SaveError::UnexpectedKind {
                expected: Kind::Save,
                found: Kind::Replay,
            },
        ),
        (
            "versión y hash mal",
            wrong_format_and_hash,
            SaveError::UnsupportedFormat {
                expected: 301,
                found: 300,
            },
        ),
        (
            "solo hash mal",
            wrong_hash,
            SaveError::DataHashMismatch {
                expected: DataHash(0x5678),
                found: DataHash(0x1234),
            },
        ),
    ] {
        assert_eq!(
            header.check(&exp).as_ref(),
            Err(&err),
            "Header::check, {label}"
        );
        assert_eq!(
            decode::<Vec<Handle>>(&bytes, &exp),
            Err(err),
            "decode, {label}"
        );
    }
    assert_eq!(header.check(&expected), Ok(()));
}

// CA-13: Un cuerpo codificado como `V1 { a: u32 }` con `format_version: 1` se carga con
// `decode_header`, se comprueba la versión, se lee con `decode_body::<V1>` y se convierte a
// `V2 { a: u32, b: u16 }`, mientras `decode::<V2>` con `format_version: 2` exigida devuelve
// `UnsupportedFormat { expected: 2, found: 1 }`; y en ese mismo camino de migración,
// `Header::check` con una `Expected` de versión 1 y `data_hash` distinto devuelve
// `DataHashMismatch` (la migración no esquiva la regla 18).
#[test]
fn ca_13_migracion_explicita_por_version_de_formato() {
    let data_hash = DataHash(0xABCD);
    let header_v1 = Header {
        kind: Kind::Save,
        format_version: 1,
        game_version: GAME_VERSION,
        data_hash,
    };
    let body_v1 = V1 { a: 42 };
    let bytes = encode_to_vec(&header_v1, &body_v1).expect("codificar el cuerpo V1");

    // El camino normal con la versión 2 exigida no carga: toca migrar.
    let expected_v2 = Expected {
        kind: Kind::Save,
        format_version: 2,
        data_hash,
    };
    assert_eq!(
        decode::<V2>(&bytes, &expected_v2),
        Err(SaveError::UnsupportedFormat {
            expected: 2,
            found: 1,
        })
    );

    // Migración explícita: cabecera, versión, cuerpo con la struct antigua y conversión.
    let (header, rest) = decode_header(&bytes).expect("decode_header del cuerpo V1");
    assert_eq!(header, header_v1);
    assert_eq!(
        header.format_version, 1,
        "la cabecera declara la versión 1 del cuerpo"
    );
    let expected_v1 = Expected {
        format_version: 1,
        ..expected_v2
    };
    assert_eq!(header.check(&expected_v1), Ok(()));
    let migrated: V2 = decode_body::<V1>(rest)
        .expect("decode_body con la struct antigua")
        .into();
    assert_eq!(
        migrated,
        V2 {
            a: 42,
            b: MIGRATED_B
        }
    );

    // Leer el cuerpo V1 con la struct nueva sin migrar no cuela: faltan bytes.
    assert_corrupt(&decode_body::<V2>(rest), "cuerpo V1 leído como V2");

    // Y el cuerpo migrado se vuelve a guardar con la versión 2 y se carga por el camino normal.
    let header_v2 = Header {
        format_version: 2,
        ..header_v1
    };
    let bytes_v2 = encode_to_vec(&header_v2, &migrated).expect("codificar el cuerpo V2");
    assert_eq!(
        decode::<V2>(&bytes_v2, &expected_v2),
        Ok((
            header_v2,
            V2 {
                a: 42,
                b: MIGRATED_B
            }
        ))
    );

    // La migración no esquiva la regla 18: con la versión 1 aceptada pero otro `data_hash`,
    // `Header::check` devuelve `DataHashMismatch` con los dos valores.
    let other_data_v1 = Expected {
        data_hash: DataHash(0x0BAD),
        ..expected_v1
    };
    assert_eq!(
        header.check(&other_data_v1),
        Err(SaveError::DataHashMismatch {
            expected: DataHash(0x0BAD),
            found: data_hash,
        })
    );
}

// CA-14: Tras un primer `encode` en un `Vec` vacío, un segundo `encode` del mismo cuerpo en el
// mismo `Vec` deja `capacity()` igual y `allocation_counter::measure` cuenta 0 asignaciones
// durante la segunda llamada.
#[test]
fn ca_14_encode_reutiliza_el_buffer() {
    let columns = Columns::generate(GENERATOR_SEED);
    let header = Header {
        kind: Kind::Save,
        format_version: 7,
        game_version: GAME_VERSION,
        data_hash: DataHash(0xDEAD_BEEF_CAFE_F00D),
    };

    let mut out = Vec::new();
    assert_eq!(
        out.capacity(),
        0,
        "el Vec de partida está vacío y sin capacidad"
    );
    encode(&header, &columns, &mut out).expect("primer encode");
    let first = out.clone();
    let capacity_after_first = out.capacity();
    assert!(capacity_after_first >= first.len());

    // La segunda llamada no asigna nada en este hilo y deja la capacidad como estaba.
    let mut second: Result<(), SaveError> = Ok(());
    let info = allocation_counter::measure(|| {
        second = encode(&header, &columns, &mut out);
    });
    second.expect("segundo encode");
    assert_eq!(
        info.count_total, 0,
        "el segundo encode asignó {} veces ({} bytes)",
        info.count_total, info.bytes_total
    );
    assert_eq!(
        out.capacity(),
        capacity_after_first,
        "la capacidad no cambia"
    );
    assert_eq!(out, first, "el segundo encode escribe los mismos bytes");

    // Y lo mismo con el sobre pequeño del ejemplo fijado sobre un `Vec` ya usado: `encode`
    // vacía `out` antes de escribir, sin soltar la capacidad.
    let mut small: Result<(), SaveError> = Ok(());
    let info = allocation_counter::measure(|| {
        small = encode(&GOLDEN_HEADER, &(), &mut out);
    });
    small.expect("encode del ejemplo fijado en un Vec con capacidad");
    assert_eq!(info.count_total, 0);
    assert_eq!(out, GOLDEN_BYTES);
    assert_eq!(out.capacity(), capacity_after_first);
}

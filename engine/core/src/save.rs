//! Sobre binario de guardados y replays (ADR-0007, regla 18): `MAGIC`,
//! `ENVELOPE_VERSION`, la cabecera `Header`, lo que exige el llamante
//! (`Expected`), `SaveError` y las funciones de codificación en `postcard`.
//!
//! Este crate crea la cabecera y la codificación, no el cuerpo: `engine_sim`
//! define el cuerpo del replay y el del guardado, y `game_rules` sus structs
//! canónicas. Quien define el cuerpo lo versiona con `format_version` y migra
//! con `decode_header` más `decode_body`; este crate no migra nada.
// Contrato B2: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T08 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T08")]

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::data::DataHash;
use crate::version::Version;

/// Los 8 primeros bytes (posiciones 0 a 7) de todo guardado y replay.
pub const MAGIC: [u8; 8] = *b"PLUSULTR";
/// Versión del sobre, en la posición 8: la del propio `Header`. Sube solo si cambia la struct `Header`.
/// La del cuerpo va dentro del `Header`.
pub const ENVELOPE_VERSION: u8 = 1;

/// Qué contiene el cuerpo. Variantes nuevas solo al final, porque se codifican por índice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Partida guardada: el estado canónico de la simulación.
    Save,
    /// Repetición: la semilla y las órdenes selladas por tick.
    Replay,
}

/// Cabecera del ADR-0007: tipo, versión del formato del cuerpo, versión del juego y hash de los datos con los que se jugó.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// Qué contiene el cuerpo.
    pub kind: Kind,
    /// La fija quien define el cuerpo (`engine_sim` o `game_rules`) y la sube con cada cambio de struct canónica.
    pub format_version: u32,
    /// Informativa: no decide por sí sola si el archivo se puede cargar.
    pub game_version: Version,
    /// Hash de los datos con los que se jugó (regla 18).
    pub data_hash: DataHash,
}

impl Header {
    /// Comprueba, en este orden, `kind`, `format_version` y `data_hash`, y devuelve el primer error.
    pub fn check(&self, expected: &Expected) -> Result<(), SaveError> {
        todo!("P01-T08")
    }
}

/// Lo que el llamante exige a una cabecera para cargarla sin migración.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    /// Tipo de archivo que se quiere cargar.
    pub kind: Kind,
    /// Versión del formato del cuerpo con la que se va a leer.
    pub format_version: u32,
    /// Hash de los datos cargados en este proceso.
    pub data_hash: DataHash,
}

/// Mensajes en español. `PartialEq` para que los tests comparen errores enteros.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    /// Menos de 8 bytes, o los 8 primeros no son `MAGIC`.
    #[error("el archivo no es un guardado ni un replay: faltan los 8 bytes iniciales `PLUSULTR`")]
    BadMagic,
    /// El byte de la posición 8 no es `ENVELOPE_VERSION`.
    #[error(
        "versión del sobre no admitida: se encontró la {found} y esta build lee la {}",
        ENVELOPE_VERSION
    )]
    UnsupportedEnvelope {
        /// Byte de la posición 8 del archivo.
        found: u8,
    },
    /// El archivo es de otro tipo: por ejemplo, un replay donde se esperaba un guardado.
    #[error("tipo de archivo inesperado: se esperaba {expected:?} y se encontró {found:?}")]
    UnexpectedKind {
        /// Tipo que exigía el llamante.
        expected: Kind,
        /// Tipo que declara la cabecera.
        found: Kind,
    },
    /// La versión del formato del cuerpo no es la que el llamante sabe leer; toca migrar.
    #[error(
        "versión del formato del cuerpo no admitida: se esperaba la {expected} y se encontró la {found}"
    )]
    UnsupportedFormat {
        /// Versión del formato que exigía el llamante.
        expected: u32,
        /// Versión del formato que declara la cabecera.
        found: u32,
    },
    /// Regla 18: los datos con los que se jugó no son los cargados.
    #[error("los datos con los que se jugó ({found}) no son los cargados ({expected})")]
    DataHashMismatch {
        /// Hash de los datos cargados en este proceso.
        expected: DataHash,
        /// Hash de los datos que declara la cabecera.
        found: DataHash,
    },
    /// Sobran `count` bytes después del cuerpo.
    #[error("después del cuerpo sobran {count} bytes")]
    TrailingBytes {
        /// Cuántos bytes quedan sin consumir.
        count: usize,
    },
    /// Truncado o mal formado: un archivo de exactamente 8 bytes, una cabecera a medias, un `Kind` desconocido,
    /// un handle con generación 0 en el cuerpo; `String` con el detalle del codificador.
    #[error("archivo truncado o mal formado: {0}")]
    Corrupt(String),
    /// El cuerpo no se pudo serializar; `String` con el detalle del codificador.
    #[error("no se pudo serializar el cuerpo: {0}")]
    Encode(String),
}

/// Vacía `out` y escribe en él `MAGIC`, `ENVELOPE_VERSION`, la cabecera y el cuerpo, en `postcard`.
/// Reutiliza la capacidad de `out`: llamado dos veces con el mismo cuerpo, no vuelve a asignar.
// El contrato pide `&mut Vec<u8>` para vaciarlo y crecer en él; clippy solo ve un slice porque el cuerpo
// es un marcador. En cuanto P01-T08 use `clear` y `extend_from_slice`, este `expect` sobra y hay que quitarlo.
#[expect(
    clippy::ptr_arg,
    reason = "la firma es la del contrato y el cuerpo pendiente no usa aún la capacidad del Vec"
)]
pub fn encode<T: Serialize + ?Sized>(
    header: &Header,
    body: &T,
    out: &mut Vec<u8>,
) -> Result<(), SaveError> {
    todo!("P01-T08")
}

/// `encode` sobre un `Vec` nuevo.
pub fn encode_to_vec<T: Serialize + ?Sized>(
    header: &Header,
    body: &T,
) -> Result<Vec<u8>, SaveError> {
    todo!("P01-T08")
}

/// Valida `MAGIC` y `ENVELOPE_VERSION`, decodifica la cabecera y devuelve el resto, que es el cuerpo sin tocar.
/// Es el primer paso de una migración: se mira `format_version` y se elige con qué struct leer el cuerpo.
pub fn decode_header(bytes: &[u8]) -> Result<(Header, &[u8]), SaveError> {
    todo!("P01-T08")
}

/// Decodifica un cuerpo entero. Exige consumirlo todo; si sobran bytes, `TrailingBytes`.
pub fn decode_body<T: DeserializeOwned>(body: &[u8]) -> Result<T, SaveError> {
    todo!("P01-T08")
}

/// `decode_header`, `Header::check(expected)` y `decode_body`, en ese orden. El camino normal de carga.
pub fn decode<T: DeserializeOwned>(
    bytes: &[u8],
    expected: &Expected,
) -> Result<(Header, T), SaveError> {
    todo!("P01-T08")
}

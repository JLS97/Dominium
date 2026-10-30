//! Datos de autoría en RON (regla 11) y su hash (regla 18): `DataHash`, el
//! listado ordenado de archivos de datos, `parse_ron` y `load_ron`.
//!
//! Es el único módulo del crate que toca el sistema de archivos, y nunca en un
//! camino caliente. Qué archivos existen y qué structs contienen lo deciden las
//! specs de features y de `game_rules`.
// Contrato B1: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T05 (`of_bytes` y `Display`) y a P01-T07 (el resto) a retirar esta línea.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T05 y P01-T07")]

use core::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Hash de los datos con los que se juega (regla 18): xxh3 de 64 bits (Q-006). `Display` en hexadecimal de 16 dígitos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DataHash(pub u64);

impl fmt::Display for DataHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("P01-T05")
    }
}

impl DataHash {
    /// `xxh3_64` con semilla 0 de `bytes`.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> DataHash {
        todo!("P01-T05")
    }
}

/// Extensión de los archivos de datos. Solo estos entran en el listado y en el hash (Q-010).
pub const DATA_EXTENSION: &str = "ron";

/// Un archivo de datos: su ruta relativa a la raíz, con `/` como separador, y su ruta completa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataFile {
    /// Ruta relativa a la raíz del árbol de datos, con `/` como separador en cualquier plataforma.
    pub relative: String,
    /// Ruta completa con la que se abre el archivo.
    pub path: PathBuf,
}

/// Todos los archivos regulares con extensión `ron` (sin distinguir mayúsculas) bajo `root`, recursivamente,
/// ordenados por los bytes UTF-8 de `relative`. No sigue enlaces simbólicos. El orden no depende del sistema de archivos.
/// `root` inexistente da `DataError::Io`; un nombre que no es UTF-8, `DataError::InvalidPath`.
pub fn list_data_files(root: &Path) -> Result<Vec<DataFile>, DataError> {
    todo!("P01-T07")
}

/// Hash del árbol de datos. Para cada archivo de `list_data_files`, en su orden, entran: los bytes UTF-8 de
/// `relative`, un byte 0, la longitud del contenido como `u64` little-endian y el contenido tal cual.
/// Un árbol sin archivos da el hash de la secuencia vacía.
pub fn hash_data_tree(root: &Path) -> Result<DataHash, DataError> {
    todo!("P01-T07")
}

/// Deserializa RON con las opciones por defecto de `ron`. Quita una marca de orden de bytes UTF-8 inicial si la hay.
/// Las extensiones de RON se activan dentro del propio archivo (`#![enable(implicit_some)]`), no aquí.
pub fn parse_ron<T: DeserializeOwned>(text: &str) -> Result<T, DataError> {
    todo!("P01-T07")
}

/// Lee el archivo y llama a `parse_ron`. Los errores llevan la ruta. Un contenido que no es UTF-8 da `Io` con
/// `source.kind() == InvalidData`.
pub fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T, DataError> {
    todo!("P01-T07")
}

/// Mensajes en español.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    /// Fallo al leer un archivo o recorrer una carpeta. `source` conserva el error del sistema.
    #[error("error de entrada/salida en `{}`: {source}", .path.display())]
    Io {
        /// Ruta del archivo o carpeta que falló.
        path: PathBuf,
        /// Error del sistema que lo causó.
        #[source]
        source: std::io::Error,
    },
    /// Un nombre de archivo o de carpeta que no es UTF-8.
    #[error("ruta de datos que no es UTF-8: `{}`", .path.display())]
    InvalidPath {
        /// Ruta con el nombre que no se pudo convertir.
        path: PathBuf,
    },
    /// `line` y `column` empiezan en 1. `path` es `None` cuando viene de `parse_ron`.
    #[error(
        "error de sintaxis RON{} en la línea {line}, columna {column}: {message}",
        describe_path(.path.as_deref())
    )]
    Parse {
        /// Ruta del archivo, o `None` si el texto no venía de un archivo.
        path: Option<PathBuf>,
        /// Línea del error, desde 1.
        line: u32,
        /// Columna del error, desde 1.
        column: u32,
        /// Mensaje del analizador de RON.
        message: String,
    },
}

/// ` en `ruta`` si hay ruta, y nada si no. Solo para el mensaje de `DataError::Parse`.
fn describe_path(path: Option<&Path>) -> String {
    path.map_or_else(String::new, |path| format!(" en `{}`", path.display()))
}

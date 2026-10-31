//! Datos de autoría en RON (regla 11) y su hash (regla 18): `DataHash`, el
//! listado ordenado de archivos de datos, `parse_ron` y `load_ron`.
//!
//! Es el único módulo del crate que toca el sistema de archivos, y nunca en un
//! camino caliente. Qué archivos existen y qué structs contienen lo deciden las
//! specs de features y de `game_rules`.

use core::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::hash::StateHasher;

/// Hash de los datos con los que se juega (regla 18): xxh3 de 64 bits (Q-006). `Display` en hexadecimal de 16 dígitos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DataHash(pub u64);

impl fmt::Display for DataHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl DataHash {
    /// `xxh3_64` con semilla 0 de `bytes`.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> DataHash {
        DataHash(xxhash_rust::xxh3::xxh3_64(bytes))
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
    let mut files = Vec::new();
    walk_data_dir(root, &mut String::new(), &mut files)?;
    // `read_dir` devuelve las entradas en el orden del sistema de archivos, que cambia entre plataformas y
    // entre ejecuciones. El contrato fija el orden por los bytes UTF-8 de `relative`. Las rutas relativas son
    // únicas dentro de un árbol, así que la ordenación inestable da siempre el mismo resultado.
    files.sort_unstable_by(|a, b| a.relative.as_bytes().cmp(b.relative.as_bytes()));
    Ok(files)
}

/// Recorre `dir` y añade a `files` sus archivos de datos, bajando por las subcarpetas. `prefix` es la ruta
/// relativa de `dir` con `/` final, o vacía para la raíz; al volver queda como estaba.
fn walk_data_dir(
    dir: &Path,
    prefix: &mut String,
    files: &mut Vec<DataFile>,
) -> Result<(), DataError> {
    let dir_error = |source| DataError::Io {
        path: dir.to_path_buf(),
        source,
    };
    for entry in fs::read_dir(dir).map_err(dir_error)? {
        let entry = entry.map_err(dir_error)?;
        // `DirEntry::file_type` no sigue enlaces simbólicos: un enlace se ve como tal y se descarta, apunte
        // a un archivo o a una carpeta. Así el árbol de datos no sale nunca de `root`.
        let file_type = entry.file_type().map_err(|source| DataError::Io {
            path: entry.path(),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(DataError::InvalidPath { path: entry.path() });
        };
        let prefix_len = prefix.len();
        prefix.push_str(name);
        if file_type.is_dir() {
            prefix.push('/');
            walk_data_dir(&entry.path(), prefix, files)?;
        } else if file_type.is_file() && has_data_extension(name) {
            files.push(DataFile {
                relative: prefix.clone(),
                path: entry.path(),
            });
        }
        prefix.truncate(prefix_len);
    }
    Ok(())
}

/// Si `name` termina en `.ron` sin distinguir mayúsculas de minúsculas (`B.RON` cuenta).
fn has_data_extension(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(DATA_EXTENSION))
}

/// Hash del árbol de datos. Para cada archivo de `list_data_files`, en su orden, entran: los bytes UTF-8 de
/// `relative`, un byte 0, la longitud del contenido como `u64` little-endian y el contenido tal cual.
/// Un árbol sin archivos da el hash de la secuencia vacía.
pub fn hash_data_tree(root: &Path) -> Result<DataHash, DataError> {
    let mut hasher = StateHasher::new();
    for file in list_data_files(root)? {
        let content = fs::read(&file.path).map_err(|source| DataError::Io {
            path: file.path,
            source,
        })?;
        hasher.write(file.relative.as_bytes());
        hasher.write_u8(0);
        // `usize` no entra nunca en un hash (ADR-0009): la longitud va como `u64` para que el valor sea el
        // mismo en cualquier plataforma. En las soportadas `usize` cabe en `u64`, así que no satura.
        hasher.write_u64(u64::try_from(content.len()).unwrap_or(u64::MAX));
        hasher.write(&content);
    }
    Ok(DataHash(hasher.finish().0))
}

/// Deserializa RON con las opciones por defecto de `ron`. Quita una marca de orden de bytes UTF-8 inicial si la hay.
/// Las extensiones de RON se activan dentro del propio archivo (`#![enable(implicit_some)]`), no aquí.
pub fn parse_ron<T: DeserializeOwned>(text: &str) -> Result<T, DataError> {
    parse_ron_from(text, None)
}

/// `parse_ron` con la ruta que llevará `DataError::Parse`: `None` desde `parse_ron`, `Some` desde `load_ron`.
fn parse_ron_from<T: DeserializeOwned>(text: &str, path: Option<&Path>) -> Result<T, DataError> {
    // Los editores de Windows suelen anteponer la marca de orden de bytes, y `ron` no la trata como espacio.
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
    ron::from_str(text).map_err(|error| DataError::Parse {
        path: path.map(Path::to_path_buf),
        // La posición es el inicio del span: donde empieza lo que el analizador no entendió. Las posiciones de
        // `ron` ya empiezan en 1, como las del contrato.
        line: saturate_u32(error.span.start.line),
        column: saturate_u32(error.span.start.col),
        message: error.code.to_string(),
    })
}

/// `usize` a `u32` sin pánico: un texto con más de cuatro mil millones de líneas o columnas satura.
fn saturate_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Lee el archivo y llama a `parse_ron`. Los errores llevan la ruta. Un contenido que no es UTF-8 da `Io` con
/// `source.kind() == InvalidData`.
pub fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T, DataError> {
    let bytes = fs::read(path).map_err(|source| DataError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let text = String::from_utf8(bytes).map_err(|error| DataError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, error),
    })?;
    parse_ron_from(&text, Some(path))
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

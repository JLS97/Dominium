//! Versión del juego, leída del manifiesto al compilar.
// Contrato A: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T02 a retirar esta línea al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T02")]

use core::fmt;

use serde::{Deserialize, Serialize};

/// Versión semántica reducida. El orden es el lexicográfico de `(major, minor, patch)`. `Display` escribe `0.1.0`.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Version {
    /// Componente mayor.
    pub major: u16,
    /// Componente menor.
    pub minor: u16,
    /// Componente de parche.
    pub patch: u16,
}

impl Version {
    /// Construye una versión a partir de sus tres componentes.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Version {
        panic!("P01-T02: pendiente de implementar")
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("P01-T02")
    }
}

/// La versión de `workspace.package.version`, leída de `CARGO_PKG_VERSION_MAJOR`, `_MINOR` y `_PATCH` al compilar.
pub const GAME_VERSION: Version = Version {
    major: parse_u16(env!("CARGO_PKG_VERSION_MAJOR")),
    minor: parse_u16(env!("CARGO_PKG_VERSION_MINOR")),
    patch: parse_u16(env!("CARGO_PKG_VERSION_PATCH")),
};

/// Convierte una componente de versión decimal, tal como la entrega Cargo, en `u16`.
/// Se evalúa al compilar: un texto vacío, un carácter que no sea un dígito o un valor
/// que no quepa en `u16` detienen la compilación.
const fn parse_u16(text: &str) -> u16 {
    let bytes = text.as_bytes();
    assert!(!bytes.is_empty(), "componente de versión vacía");
    let mut value: u16 = 0;
    let mut i = 0;
    while i < bytes.len() {
        let digit = bytes[i];
        assert!(digit.is_ascii_digit(), "componente de versión no numérica");
        let Some(shifted) = value.checked_mul(10) else {
            panic!("componente de versión fuera de rango");
        };
        let Some(sum) = shifted.checked_add((digit - b'0') as u16) else {
            panic!("componente de versión fuera de rango");
        };
        value = sum;
        i += 1;
    }
    value
}

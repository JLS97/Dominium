//! Comprobaciones de `check-arch` como funciones puras sobre datos ya
//! extraídos del workspace (CA-23 de `specs/architecture/engine_core.md`).
//!
//! `main.rs` lee el workspace con `cargo metadata` y el sistema de archivos, y
//! aquí se decide, sin tocar nada externo, qué viola la constitución o las
//! decisiones del autor:
//!
//! - [`direct_dependency_violations`]: solo `engine_core` declara los crates de
//!   hashing del ADR-0009, y `engine_sim` y `game_rules` no declaran
//!   colecciones con hasher aleatorio ni crates de aleatoriedad (Q-014).
//! - [`cargo_config_violations`]: `.cargo/config.toml` no activa `-C target-cpu`
//!   ni `-C target-feature`, que cambiarían los resultados en flotante entre
//!   máquinas (Q-011).
//! - [`allow_disallowed_types_violations`]: ningún `.rs` de `engine/core`,
//!   `engine/sim` ni `game/rules` permite el lint `clippy::disallowed_types`
//!   salvo `engine/core/src/collections.rs`, donde se definen los alias con
//!   hasher determinista (Q-014).
//!
//! Los tests de aceptación de `xtask/tests/aceptacion_check_arch_t10.rs`
//! ejercitan estas tres funciones con entradas construidas a mano.
#![forbid(unsafe_code)]

/// Tipo de una dependencia directa, como la clasifica `cargo metadata`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DepKind {
    /// Dependencia normal: la usa el código del crate.
    Normal,
    /// Dependencia de compilación: la usa `build.rs`.
    Build,
    /// Dependencia de desarrollo: la usan tests, ejemplos y benchmarks.
    Dev,
}

impl DepKind {
    /// Nombre en español para los mensajes de violación.
    fn describe(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Build => "de compilación",
            Self::Dev => "de desarrollo",
        }
    }
}

/// Dependencia directa de un miembro del workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectDep {
    /// Nombre del paquete dependido, no el alias con el que se renombre.
    pub name: String,
    /// Tipo de la dependencia.
    pub kind: DepKind,
}

/// Miembro del workspace con sus dependencias directas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Nombre del paquete miembro.
    pub name: String,
    /// Sus dependencias directas, una entrada por declaración en el manifiesto.
    pub deps: Vec<DirectDep>,
}

/// Archivo fuente con su ruta relativa a la raíz del repositorio, con `/` como
/// separador, y su texto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFile {
    /// Ruta relativa a la raíz del workspace, con `/` como separador.
    pub path: String,
    /// Contenido del archivo.
    pub text: String,
}

/// Único miembro que declara los crates del ADR-0009 y que reexporta lo que
/// los demás necesitan de ellos.
const HASHING_OWNER: &str = "engine_core";
/// Crates de hashing que solo declara [`HASHING_OWNER`] (ADR-0009).
const ADR_0009_CRATES: &[&str] = &["rustc-hash", "xxhash-rust", "bytemuck"];
/// Miembros de simulación sometidos al veto de Q-014.
const SIMULATION_MEMBERS: &[&str] = &["engine_sim", "game_rules"];
/// Colecciones con hasher aleatorio vetadas en los miembros de simulación
/// (Q-014). Los crates de aleatoriedad se detectan por el prefijo `rand`.
const Q_014_CRATES: &[&str] = &["hashbrown", "ahash", "foldhash", "indexmap"];
/// Prefijo de los crates de aleatoriedad vetados por Q-014.
const RANDOM_PREFIX: &str = "rand";
/// Cadenas de `.cargo/config.toml` prohibidas por Q-011.
const FORBIDDEN_RUSTFLAGS: &[&str] = &["target-cpu", "target-feature"];
/// Único archivo que puede permitir `clippy::disallowed_types` (Q-014).
const DISALLOWED_TYPES_EXEMPT: &str = "engine/core/src/collections.rs";
/// Texto del atributo que se busca en los archivos fuente.
const ALLOW_DISALLOWED_TYPES: &str = "allow(clippy::disallowed_types)";

/// Violaciones del ADR-0009 y de Q-014 en las dependencias directas de los
/// miembros del workspace.
///
/// Devuelve un mensaje por cada declaración de `rustc-hash`, `xxhash-rust` o
/// `bytemuck` en un miembro distinto de `engine_core`, y uno por cada
/// declaración de `hashbrown`, `ahash`, `foldhash`, `indexmap` o un crate cuyo
/// nombre empiece por `rand` en `engine_sim` o `game_rules`. Los tres tipos de
/// dependencia cuentan igual: una dependencia de desarrollo también activa
/// features por unificación y también puede colarse en un test de determinismo.
/// Los mensajes salen en el orden de los miembros y de sus declaraciones.
#[must_use]
pub fn direct_dependency_violations(members: &[Member]) -> Vec<String> {
    let mut violations = Vec::new();
    for member in members {
        let member_name = member.name.as_str();
        for dep in &member.deps {
            let crate_name = dep.name.as_str();
            let kind = dep.kind.describe();
            if member_name != HASHING_OWNER && ADR_0009_CRATES.contains(&crate_name) {
                violations.push(format!(
                    "ADR-0009: `{member_name}` declara `{crate_name}` como dependencia {kind}; \
                     ese crate solo lo declara el núcleo del motor, que reexporta lo necesario"
                ));
            }
            if SIMULATION_MEMBERS.contains(&member_name)
                && (Q_014_CRATES.contains(&crate_name) || crate_name.starts_with(RANDOM_PREFIX))
            {
                violations.push(format!(
                    "Q-014: `{member_name}` declara `{crate_name}` como dependencia {kind}; \
                     sin colecciones con hasher aleatorio ni crates de aleatoriedad en sim ni en rules"
                ));
            }
        }
    }
    violations
}

/// Violaciones de Q-011 en el texto de `.cargo/config.toml`.
///
/// Devuelve un mensaje por cada cadena prohibida presente (`target-cpu`,
/// `target-feature`), sin interpretar el TOML: basta con que aparezca, porque
/// no hay ningún uso legítimo de esas cadenas en ese archivo hasta que el hito 2
/// mida.
#[must_use]
pub fn cargo_config_violations(text: &str) -> Vec<String> {
    FORBIDDEN_RUSTFLAGS
        .iter()
        .filter(|flag| text.contains(*flag))
        .map(|flag| {
            format!(
                "Q-011: `.cargo/config.toml` contiene `{flag}`; `-C target-cpu` y \
                 `-C target-feature` cambian los resultados en flotante entre máquinas y \
                 están prohibidos hasta que el hito 2 mida"
            )
        })
        .collect()
}

/// Violaciones de Q-014 por permitir el lint `clippy::disallowed_types`.
///
/// Devuelve un mensaje por cada archivo distinto de
/// `engine/core/src/collections.rs` cuyo texto contenga la cadena
/// `allow(clippy::disallowed_types)`, en cualquier forma (`#![allow(...)]` en la
/// raíz o `#[allow(...)]` sobre un elemento o dentro de una función). Los
/// mensajes salen en el orden de los archivos.
#[must_use]
pub fn allow_disallowed_types_violations(files: &[SourceFile]) -> Vec<String> {
    files
        .iter()
        .filter(|file| file.path != DISALLOWED_TYPES_EXEMPT)
        .filter(|file| file.text.contains(ALLOW_DISALLOWED_TYPES))
        .map(|file| {
            format!(
                "Q-014: `{}` contiene `{ALLOW_DISALLOWED_TYPES}`; solo el módulo de \
                 colecciones del núcleo puede permitir el lint, porque ahí se definen los \
                 alias con hasher determinista",
                file.path
            )
        })
        .collect()
}

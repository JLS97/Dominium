//! Tareas de repositorio. Uso: `cargo xtask check-arch`.
//!
//! Comprueba las reglas de la constitución que se pueden leer del workspace:
//!
//! - Regla 1: las dependencias van de `game_*` a `engine_*`, nunca al revés.
//! - Regla 2: `engine_sim` y `game_rules` no dependen de nada gráfico ni de
//!   ventana, ni directa ni transitivamente.
//! - Regla 3: todos los crates salvo `engine_render` llevan
//!   `#![forbid(unsafe_code)]` en la raíz de cada target.
//! - Regla 16: solo `game_tools` puede ser una herramienta con conceptos de
//!   juego; se cubre con la regla 1, porque `engine_tools` no puede depender
//!   de `game_*`.
//!
//! Y las decisiones de `specs/architecture/engine_core.md` (CA-23), con las
//! funciones puras de la biblioteca `xtask`:
//!
//! - ADR-0009 y Q-014: dependencias directas vigiladas por miembro.
//! - Q-011: sin `target-cpu` ni `target-feature` en `.cargo/config.toml`.
//! - Q-014: sin `allow(clippy::disallowed_types)` fuera de
//!   `engine/core/src/collections.rs` en `engine/core`, `engine/sim` ni
//!   `game/rules`.
//!
//! La CI lo ejecuta en cada cambio. Falla con la lista de violaciones.
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Node, Package, PackageId};
use xtask::{
    DepKind, DirectDep, Member, SourceFile, allow_disallowed_types_violations,
    cargo_config_violations, direct_dependency_violations,
};

/// Prefijo de los crates del motor.
const ENGINE_PREFIX: &str = "engine_";
/// Prefijo de los crates del juego.
const GAME_PREFIX: &str = "game_";
/// Único crate con permiso para `unsafe` (regla 3).
const UNSAFE_ALLOWED: &str = "engine_render";
/// Crates que no pueden conocer gráficos ni ventana (regla 2).
const NO_GRAPHICS: &[&str] = &["engine_sim", "game_rules"];
/// Dependencias que delatan gráficos o ventana.
const GRAPHICS_CRATES: &[&str] = &[
    "engine_render",
    "wgpu",
    "wgpu-core",
    "wgpu-hal",
    "naga",
    "winit",
    "egui",
];
/// Atributo que exige la regla 3.
const FORBID_UNSAFE: &str = "#![forbid(unsafe_code)]";
/// Configuración de Cargo vigilada por Q-011, relativa a la raíz del workspace.
const CARGO_CONFIG: &str = ".cargo/config.toml";
/// Carpetas cuyos `.rs` se revisan en busca de `allow(clippy::disallowed_types)`
/// (Q-014), relativas a la raíz del workspace.
const DISALLOWED_TYPES_DIRS: &[&str] = &["engine/core", "engine/sim", "game/rules"];
/// Carpeta de artefactos de compilación, que no se recorre.
const TARGET_DIR: &str = "target";

fn main() -> ExitCode {
    let command = std::env::args().nth(1).unwrap_or_default();
    if command != "check-arch" {
        eprintln!("uso: cargo xtask check-arch");
        return ExitCode::from(2);
    }
    match check_arch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn check_arch() -> Result<()> {
    let meta = MetadataCommand::new()
        .exec()
        .context("no se pudo ejecutar `cargo metadata`")?;
    let resolve = meta
        .resolve
        .as_ref()
        .context("`cargo metadata` no devolvió el grafo de dependencias")?;
    let nodes: HashMap<&PackageId, &Node> = resolve.nodes.iter().map(|n| (&n.id, n)).collect();

    let mut errors: Vec<String> = Vec::new();
    let mut members: Vec<Member> = Vec::with_capacity(meta.workspace_members.len());

    for id in &meta.workspace_members {
        let pkg = package(&meta, id)?;
        let name: &str = &pkg.name;

        let dep_ids = transitive_deps(id, &nodes);
        let dep_names: HashSet<String> = dep_ids
            .iter()
            .filter_map(|dep| package(&meta, dep).ok())
            .map(|dep| dep.name.clone())
            .collect();

        if name.starts_with(ENGINE_PREFIX) {
            let mut offenders: Vec<&String> = dep_names
                .iter()
                .filter(|d| d.starts_with(GAME_PREFIX))
                .collect();
            offenders.sort();
            for dep in offenders {
                errors.push(format!(
                    "regla 1: `{name}` depende de `{dep}`; el motor no conoce el juego"
                ));
            }
        }

        if NO_GRAPHICS.contains(&name) {
            for graphics in GRAPHICS_CRATES {
                if dep_names.contains(*graphics) {
                    errors.push(format!(
                        "regla 2: `{name}` depende de `{graphics}`; sin gráficos en sim ni en rules"
                    ));
                }
            }
        }

        if name != UNSAFE_ALLOWED {
            check_forbid_unsafe(pkg, &mut errors)?;
        }

        members.push(member_of(pkg));
    }

    // CA-23: dependencias directas vigiladas (ADR-0009, Q-014).
    errors.extend(direct_dependency_violations(&members));

    // CA-23: `.cargo/config.toml` sin `target-cpu` ni `target-feature` (Q-011).
    let config_path = meta.workspace_root.join(CARGO_CONFIG);
    if config_path.is_file() {
        let text = fs::read_to_string(&config_path)
            .with_context(|| format!("no se pudo leer {config_path}"))?;
        errors.extend(cargo_config_violations(&text));
    }

    // CA-23: `allow(clippy::disallowed_types)` solo en `collections.rs` (Q-014).
    let files = source_files(&meta.workspace_root)?;
    errors.extend(allow_disallowed_types_violations(&files));

    if errors.is_empty() {
        println!(
            "check-arch: {} crates del workspace cumplen las reglas 1, 2, 3 y 16 y CA-23",
            meta.workspace_members.len()
        );
        return Ok(());
    }

    eprintln!("check-arch: violaciones de la constitución:");
    for error in &errors {
        eprintln!("  - {error}");
    }
    anyhow::bail!("{} violaciones", errors.len())
}

/// Busca un paquete por identificador en los metadatos.
fn package<'a>(meta: &'a Metadata, id: &PackageId) -> Result<&'a Package> {
    meta.packages
        .iter()
        .find(|p| &p.id == id)
        .with_context(|| format!("paquete desconocido en los metadatos: {id}"))
}

/// Dependencias transitivas de un paquete según el grafo resuelto. Incluye
/// dependencias normales, de compilación y de desarrollo: la regla es estricta.
fn transitive_deps<'a>(
    root: &'a PackageId,
    nodes: &HashMap<&'a PackageId, &'a Node>,
) -> HashSet<&'a PackageId> {
    let mut seen: HashSet<&'a PackageId> = HashSet::new();
    let mut queue: VecDeque<&'a PackageId> = VecDeque::from([root]);
    while let Some(id) = queue.pop_front() {
        if let Some(&node) = nodes.get(id) {
            for dep in &node.deps {
                if seen.insert(&dep.pkg) {
                    queue.push_back(&dep.pkg);
                }
            }
        }
    }
    seen
}

/// Comprueba que la raíz de cada target del paquete lleva el atributo de la
/// regla 3. Solo mira `src/lib.rs` y `src/main.rs`; los tests de integración y
/// los benchmarks heredan `unsafe_code = "deny"` del workspace.
fn check_forbid_unsafe(pkg: &Package, errors: &mut Vec<String>) -> Result<()> {
    let Some(dir) = pkg.manifest_path.parent() else {
        return Ok(());
    };
    for root in ["src/lib.rs", "src/main.rs"] {
        let path = dir.join(root);
        if !path.exists() {
            continue;
        }
        let source =
            fs::read_to_string(&path).with_context(|| format!("no se pudo leer {path}"))?;
        if !source.contains(FORBID_UNSAFE) {
            errors.push(format!("regla 3: `{path}` no lleva `{FORBID_UNSAFE}`"));
        }
    }
    Ok(())
}

/// Miembro del workspace con sus dependencias directas tal como las declara su
/// manifiesto: una entrada por declaración, con el nombre del paquete (no el
/// alias) y su tipo. No resuelve transitivas; los crates del propio workspace
/// también cuentan como dependencias directas.
fn member_of(pkg: &Package) -> Member {
    let deps = pkg
        .dependencies
        .iter()
        .map(|dep| DirectDep {
            name: dep.name.clone(),
            kind: match dep.kind {
                DependencyKind::Build => DepKind::Build,
                DependencyKind::Development => DepKind::Dev,
                // `Normal`, y cualquier tipo que esta versión de `cargo_metadata`
                // no conozca: sigue siendo una declaración y se vigila igual.
                _ => DepKind::Normal,
            },
        })
        .collect();
    Member {
        name: pkg.name.clone(),
        deps,
    }
}

/// Archivos `.rs` bajo las carpetas vigiladas por Q-014, con la ruta relativa a
/// la raíz del workspace y `/` como separador, en orden de ruta. No sigue
/// enlaces simbólicos ni entra en `target/`.
fn source_files(root: &Utf8Path) -> Result<Vec<SourceFile>> {
    let mut paths: Vec<Utf8PathBuf> = Vec::new();
    for dir in DISALLOWED_TYPES_DIRS {
        let dir = root.join(dir);
        if dir.is_dir() {
            collect_rs_files(&dir, &mut paths)?;
        }
    }
    paths.sort();

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("{path} no está bajo la raíz del workspace {root}"))?;
        let text = fs::read_to_string(&path).with_context(|| format!("no se pudo leer {path}"))?;
        files.push(SourceFile {
            path: relative.as_str().replace('\\', "/"),
            text,
        });
    }
    Ok(files)
}

/// Añade a `out` los `.rs` bajo `dir`, recursivamente, sin seguir enlaces
/// simbólicos y sin entrar en `target/`.
fn collect_rs_files(dir: &Utf8Path, out: &mut Vec<Utf8PathBuf>) -> Result<()> {
    let entries = dir
        .read_dir_utf8()
        .with_context(|| format!("no se pudo listar {dir}"))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("no se pudo leer una entrada de {dir}"))?;
        let path = entry.into_path();
        let file_type = path
            .symlink_metadata()
            .with_context(|| format!("no se pudo consultar {path}"))?
            .file_type();
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if path.file_name() != Some(TARGET_DIR) {
                collect_rs_files(&path, out)?;
            }
        } else if path.extension() == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

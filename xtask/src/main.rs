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
//! La CI lo ejecuta en cada cambio. Falla con la lista de violaciones.
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Node, Package, PackageId};

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
    }

    if errors.is_empty() {
        println!(
            "check-arch: {} crates del workspace cumplen las reglas 1, 2, 3 y 16",
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

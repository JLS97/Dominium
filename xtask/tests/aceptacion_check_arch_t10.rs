//! Tests de aceptación de P01-T10: extensión de `check-arch` en `xtask`.
//!
//! Cubren CA-23 de `specs/architecture/engine_core.md` con entradas construidas
//! a mano, sobre las tres funciones puras de `xtask/src/lib.rs`. Solo usan la API
//! pública del crate y no los modifica el implementador. CA-24 es de `deny.toml`
//! y lo comprueban la puerta (`cargo deny check`) y las dos pruebas negativas del
//! orquestador, no un test. La prueba negativa de CA-23 sobre el workspace real
//! (`rustc-hash.workspace = true` en `engine/sim/Cargo.toml`) también la hace y
//! retira el orquestador.
//!
//! Los mensajes son en español y la spec no fija su texto: aquí se comprueba que
//! cada uno contiene la cita que exige la spec (`ADR-0009`, `Q-014` o `Q-011`),
//! el nombre del miembro y el del crate ofensor, o la ruta del archivo.

use xtask::{
    DepKind, DirectDep, Member, SourceFile, allow_disallowed_types_violations,
    cargo_config_violations, direct_dependency_violations,
};

/// Texto real de `.cargo/config.toml` del repositorio, vigilado por CA-23: no
/// debe producir violaciones. La ruta es relativa a este archivo de tests.
const CARGO_CONFIG_TOML: &str = include_str!("../../.cargo/config.toml");

/// Los tres crates que solo `engine_core` puede declarar (ADR-0009).
const ADR_0009_CRATES: &[&str] = &["rustc-hash", "xxhash-rust", "bytemuck"];
/// Crates de colecciones y aleatoriedad vetados en `engine_sim` y `game_rules`
/// (Q-014). Los tres últimos cubren "un crate cuyo nombre empiece por `rand`".
const Q_014_CRATES: &[&str] = &[
    "hashbrown",
    "ahash",
    "foldhash",
    "indexmap",
    "rand",
    "rand_xoshiro",
    "rand_chacha",
];
/// Los dos crates de simulación a los que se aplica el veto de Q-014.
const Q_014_MEMBERS: &[&str] = &["engine_sim", "game_rules"];
/// Los tres tipos de dependencia directa que distingue `cargo metadata`.
const ALL_KINDS: [DepKind; 3] = [DepKind::Normal, DepKind::Build, DepKind::Dev];

/// Dependencia directa con nombre y tipo.
fn dep(name: &str, kind: DepKind) -> DirectDep {
    DirectDep {
        name: name.to_owned(),
        kind,
    }
}

/// Miembro del workspace con las dependencias indicadas.
fn member(name: &str, deps: &[(&str, DepKind)]) -> Member {
    Member {
        name: name.to_owned(),
        deps: deps.iter().map(|(n, k)| dep(n, *k)).collect(),
    }
}

/// Archivo fuente con ruta relativa a la raíz y separador `/`.
fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

/// Comprueba que un mensaje cita todos los fragmentos pedidos.
fn assert_cites(message: &str, fragments: &[&str]) {
    for fragment in fragments {
        assert!(
            message.contains(fragment),
            "el mensaje {message:?} no cita {fragment:?}"
        );
    }
}

/// Mensajes que contienen un fragmento dado.
fn citing<'a>(messages: &'a [String], fragment: &str) -> Vec<&'a String> {
    messages.iter().filter(|m| m.contains(fragment)).collect()
}

// CA-23: "un mensaje que cite el ADR-0009 por cada miembro distinto de
// `engine_core` que declare `rustc-hash`, `xxhash-rust` o `bytemuck` [...] en
// cualquiera de los tres tipos de dependencia". Lo demuestra "un miembro ajeno
// con cada uno de los tres crates del ADR-0009 en cada uno de los tres tipos de
// dependencia da error". Nueve casos, uno por combinación crate × tipo, en
// `engine_sim`: ninguno de estos tres crates está en la lista de Q-014, así que
// cada caso debe dar exactamente un mensaje, y ese mensaje cita el ADR-0009.
#[test]
fn ca_23_miembro_ajeno_con_cada_crate_del_adr_0009_en_cada_tipo_da_error() {
    for crate_name in ADR_0009_CRATES {
        for kind in ALL_KINDS {
            let members = [member("engine_sim", &[(crate_name, kind)])];
            let violations = direct_dependency_violations(&members);
            assert_eq!(
                violations.len(),
                1,
                "`engine_sim` con `{crate_name}` como {kind:?} debe dar exactamente una \
                 violación, dio {violations:?}"
            );
            assert_cites(&violations[0], &["ADR-0009", "engine_sim", crate_name]);
        }
    }
}

// CA-23: el veto del ADR-0009 vale para todo miembro distinto de `engine_core`,
// no solo para los de simulación: `game_app` y `engine_render` con `bytemuck`
// también dan error, y el mensaje nombra al miembro y al crate.
#[test]
fn ca_23_cualquier_miembro_ajeno_con_crate_del_adr_0009_da_error() {
    for member_name in [
        "game_app",
        "engine_render",
        "engine_tools",
        "game_tools",
        "xtask",
    ] {
        let members = [member(member_name, &[("bytemuck", DepKind::Normal)])];
        let violations = direct_dependency_violations(&members);
        assert_eq!(
            violations.len(),
            1,
            "`{member_name}` con `bytemuck` debe dar una violación, dio {violations:?}"
        );
        assert_cites(&violations[0], &["ADR-0009", member_name, "bytemuck"]);
    }
}

// CA-23: "`engine_core` con los tres no da error". Se comprueba con los tres
// como dependencia normal, que es como los declara el contrato A, y además con
// los tres en los tres tipos: la exención es del miembro, no del tipo.
#[test]
fn ca_23_engine_core_con_los_tres_crates_del_adr_0009_no_da_error() {
    let normal: Vec<(&str, DepKind)> = ADR_0009_CRATES
        .iter()
        .map(|c| (*c, DepKind::Normal))
        .collect();
    let members = [member("engine_core", &normal)];
    assert_eq!(
        direct_dependency_violations(&members),
        Vec::<String>::new(),
        "`engine_core` es el único que declara los crates del ADR-0009"
    );

    let all_kinds: Vec<(&str, DepKind)> = ADR_0009_CRATES
        .iter()
        .flat_map(|c| ALL_KINDS.into_iter().map(move |k| (*c, k)))
        .collect();
    let members = [member("engine_core", &all_kinds)];
    assert_eq!(
        direct_dependency_violations(&members),
        Vec::<String>::new(),
        "la exención de `engine_core` cubre los tres tipos de dependencia"
    );
}

// CA-23: "un mensaje que cite Q-014 por cada vez que `engine_sim` o `game_rules`
// declare `hashbrown`, `ahash`, `foldhash`, `indexmap` o un crate cuyo nombre
// empiece por `rand`, en cualquiera de los tres tipos de dependencia". Lo
// demuestra "`engine_sim` con `hashbrown` o con `rand_xoshiro` da error"; aquí se
// recorren los dos miembros, los siete crates (incluido `rand` a secas) y los
// tres tipos: cada combinación da exactamente un mensaje que cita Q-014.
#[test]
fn ca_23_engine_sim_y_game_rules_con_crates_de_q_014_en_cada_tipo_dan_error() {
    for member_name in Q_014_MEMBERS {
        for crate_name in Q_014_CRATES {
            for kind in ALL_KINDS {
                let members = [member(member_name, &[(crate_name, kind)])];
                let violations = direct_dependency_violations(&members);
                assert_eq!(
                    violations.len(),
                    1,
                    "`{member_name}` con `{crate_name}` como {kind:?} debe dar exactamente \
                     una violación, dio {violations:?}"
                );
                assert_cites(&violations[0], &["Q-014", member_name, crate_name]);
            }
        }
    }
}

// CA-23: "`game_app` con ellos no [da error]". El veto de Q-014 es solo para
// `engine_sim` y `game_rules`; los demás miembros pueden declarar `hashbrown` o
// `rand_xoshiro` en cualquier tipo (los traerán `wgpu` y `egui` en el hito 1).
#[test]
fn ca_23_game_app_con_hashbrown_o_rand_xoshiro_no_da_error() {
    for member_name in [
        "game_app",
        "engine_render",
        "engine_tools",
        "game_tools",
        "xtask",
    ] {
        for crate_name in Q_014_CRATES {
            for kind in ALL_KINDS {
                let members = [member(member_name, &[(crate_name, kind)])];
                assert_eq!(
                    direct_dependency_violations(&members),
                    Vec::<String>::new(),
                    "`{member_name}` con `{crate_name}` como {kind:?} no está vetado por Q-014"
                );
            }
        }
    }
}

// CA-23: "un workspace sin ellos no da error". Un workspace parecido al real
// tras el hito 1, con las dependencias permitidas de `engine_core` en su sitio,
// gráficos en `engine_render` y `game_app`, y dependencias de desarrollo
// (`proptest`, `criterion`, `allocation-counter`) en los crates de simulación.
// Incluye `operand` en `engine_sim`: contiene "rand" pero no empieza por él, así
// que no es un crate de aleatoriedad y no debe dar error.
#[test]
fn ca_23_workspace_sin_crates_vigilados_no_da_error() {
    use DepKind::{Build, Dev, Normal};
    let members = [
        member(
            "engine_core",
            &[
                ("glam", Normal),
                ("serde", Normal),
                ("postcard", Normal),
                ("ron", Normal),
                ("rustc-hash", Normal),
                ("xxhash-rust", Normal),
                ("bytemuck", Normal),
                ("thiserror", Normal),
                ("tracy-client", Normal),
                ("proptest", Dev),
                ("criterion", Dev),
                ("allocation-counter", Dev),
            ],
        ),
        member(
            "engine_sim",
            &[
                ("engine_core", Normal),
                ("rayon", Normal),
                ("operand", Normal),
                ("proptest", Dev),
                ("criterion", Dev),
                ("allocation-counter", Dev),
            ],
        ),
        member(
            "engine_render",
            &[("engine_core", Normal), ("wgpu", Normal), ("naga", Build)],
        ),
        member(
            "engine_tools",
            &[("engine_core", Normal), ("image", Normal)],
        ),
        member(
            "game_rules",
            &[
                ("engine_core", Normal),
                ("engine_sim", Normal),
                ("serde", Normal),
                ("ron", Normal),
                ("proptest", Dev),
            ],
        ),
        member(
            "game_app",
            &[
                ("engine_core", Normal),
                ("engine_sim", Normal),
                ("engine_render", Normal),
                ("game_rules", Normal),
                ("winit", Normal),
                ("egui", Normal),
            ],
        ),
        member("game_tools", &[("game_rules", Normal), ("egui", Normal)]),
        member("xtask", &[("anyhow", Normal), ("cargo_metadata", Normal)]),
    ];
    assert_eq!(direct_dependency_violations(&members), Vec::<String>::new());
    assert_eq!(direct_dependency_violations(&[]), Vec::<String>::new());
}

// CA-23: las dos listas son independientes y se informa de todas las violaciones
// a la vez, no solo de la primera. `engine_sim` con `rustc-hash`, `hashbrown` como
// dependencia normal y `hashbrown` otra vez como dependencia de desarrollo, más
// `game_app` con `xxhash-rust`: los mensajes de ADR-0009 citan a los dos miembros
// y sus dos crates, y hay un mensaje de Q-014 "por cada vez" que `engine_sim`
// declara `hashbrown`, es decir, dos.
#[test]
fn ca_23_varias_violaciones_se_informan_todas() {
    let members = [
        member(
            "engine_sim",
            &[
                ("engine_core", DepKind::Normal),
                ("rustc-hash", DepKind::Normal),
                ("hashbrown", DepKind::Normal),
                ("hashbrown", DepKind::Dev),
            ],
        ),
        member("game_app", &[("xxhash-rust", DepKind::Normal)]),
    ];
    let violations = direct_dependency_violations(&members);

    let adr = citing(&violations, "ADR-0009");
    assert!(
        adr.iter()
            .any(|m| m.contains("engine_sim") && m.contains("rustc-hash")),
        "falta la violación del ADR-0009 de `engine_sim` con `rustc-hash`: {violations:?}"
    );
    assert!(
        adr.iter()
            .any(|m| m.contains("game_app") && m.contains("xxhash-rust")),
        "falta la violación del ADR-0009 de `game_app` con `xxhash-rust`: {violations:?}"
    );

    let q014 = citing(&violations, "Q-014");
    assert_eq!(
        q014.len(),
        2,
        "`hashbrown` declarado dos veces en `engine_sim` da dos mensajes de Q-014: {violations:?}"
    );
    for message in &q014 {
        assert_cites(message, &["engine_sim", "hashbrown"]);
    }
    assert!(
        !violations.iter().any(|m| m.contains("engine_core")),
        "`engine_core` como dependencia de `engine_sim` no es una violación: {violations:?}"
    );
}

// CA-23: "`cargo_config_violations(text)`, sobre el texto de `.cargo/config.toml`,
// devuelve un mensaje que cite Q-011 si contiene `target-cpu`". Formas habituales
// de activarlo: `[build] rustflags` y `[target.<triple>] rustflags`.
#[test]
fn ca_23_config_con_target_cpu_da_error_citando_q_011() {
    let texts = [
        "[build]\nrustflags = [\"-C\", \"target-cpu=native\"]\n",
        "[target.x86_64-pc-windows-msvc]\nrustflags = [\"-Ctarget-cpu=x86-64-v3\"]\n",
        // El alias real más una línea añadida al final.
        &format!("{CARGO_CONFIG_TOML}\n[build]\nrustflags = [\"-C\", \"target-cpu=native\"]\n"),
    ];
    for text in texts {
        let violations = cargo_config_violations(text);
        assert!(
            !violations.is_empty(),
            "un `.cargo/config.toml` con `target-cpu` debe dar error: {text:?}"
        );
        for message in &violations {
            assert_cites(message, &["Q-011"]);
        }
    }
}

// CA-23: "[...] si contiene `target-cpu` o `target-feature`". Mismas formas con
// `target-feature`.
#[test]
fn ca_23_config_con_target_feature_da_error_citando_q_011() {
    let texts = [
        "[build]\nrustflags = [\"-C\", \"target-feature=+avx2,+fma\"]\n",
        "[target.x86_64-pc-windows-msvc]\nrustflags = [\"-Ctarget-feature=+fma\"]\n",
        &format!("{CARGO_CONFIG_TOML}\n[build]\nrustflags = [\"-C\", \"target-feature=+fma\"]\n"),
    ];
    for text in texts {
        let violations = cargo_config_violations(text);
        assert!(
            !violations.is_empty(),
            "un `.cargo/config.toml` con `target-feature` debe dar error: {text:?}"
        );
        for message in &violations {
            assert_cites(message, &["Q-011"]);
        }
    }
}

// CA-23: "el texto del `.cargo/config.toml` actual no [da error]". Se lee el
// archivo real del repositorio con `include_str!`; antes se comprueba que la
// ruta resolvió al archivo esperado (el alias de `xtask`) y que ese archivo no
// contiene ya las cadenas vetadas, para que el test no pase por accidente. Un
// texto vacío tampoco da error.
#[test]
fn ca_23_config_actual_del_repositorio_no_da_error() {
    assert!(
        CARGO_CONFIG_TOML.contains("[alias]") && CARGO_CONFIG_TOML.contains("xtask"),
        "`include_str!` no resolvió al `.cargo/config.toml` del repositorio"
    );
    assert!(
        !CARGO_CONFIG_TOML.contains("target-cpu") && !CARGO_CONFIG_TOML.contains("target-feature"),
        "el `.cargo/config.toml` del repositorio ya contiene una cadena vetada por Q-011"
    );
    assert_eq!(
        cargo_config_violations(CARGO_CONFIG_TOML),
        Vec::<String>::new()
    );
    assert_eq!(cargo_config_violations(""), Vec::<String>::new());
    // Un `rustflags` sin las dos cadenas vetadas tampoco se rechaza.
    assert_eq!(
        cargo_config_violations("[build]\nrustflags = [\"-D\", \"warnings\"]\n"),
        Vec::<String>::new()
    );
}

// CA-23: "`allow_disallowed_types_violations(files)` [...] devuelve un mensaje por
// cada archivo distinto de `engine/core/src/collections.rs` cuyo texto contenga
// `allow(clippy::disallowed_types)` (Q-014)". Lo demuestra "un archivo
// `engine/sim/src/x.rs` con `#[allow(clippy::disallowed_types)]` da error": el
// mensaje cita Q-014 y la ruta del archivo.
#[test]
fn ca_23_archivo_de_sim_con_allow_disallowed_types_da_error() {
    let files = [source(
        "engine/sim/src/x.rs",
        "//! Módulo de prueba.\n\n#[allow(clippy::disallowed_types)]\nfn f() {}\n",
    )];
    let violations = allow_disallowed_types_violations(&files);
    assert_eq!(
        violations.len(),
        1,
        "un archivo con el atributo da exactamente una violación: {violations:?}"
    );
    assert_cites(&violations[0], &["Q-014", "engine/sim/src/x.rs"]);
}

// CA-23: el atributo puede ir en la raíz del crate (`#![allow(...)]`) o dentro de
// una función; la spec habla del texto que lo "contenga", así que ambas formas
// se detectan en las tres carpetas vigiladas (`engine/core`, `engine/sim`,
// `game/rules`), también en archivos que no son `collections.rs` de `engine_core`.
#[test]
fn ca_23_allow_disallowed_types_se_detecta_en_cualquier_forma_y_carpeta() {
    let cases = [
        (
            "game/rules/src/economy.rs",
            "#![allow(clippy::disallowed_types)]\nuse std::collections::HashMap;\n",
        ),
        (
            "engine/core/src/hash.rs",
            "fn g() {\n    #[allow(clippy::disallowed_types)]\n    let m = std::collections::HashMap::<u8, u8>::new();\n    drop(m);\n}\n",
        ),
        (
            "engine/sim/src/collections.rs",
            "#[allow(clippy::disallowed_types)]\npub struct Grid;\n",
        ),
        (
            "engine/sim/tests/aceptacion_sim_t01.rs",
            "#[allow(clippy::disallowed_types)]\nfn h() {}\n",
        ),
    ];
    for (path, text) in cases {
        let files = [source(path, text)];
        let violations = allow_disallowed_types_violations(&files);
        assert_eq!(
            violations.len(),
            1,
            "`{path}` con el atributo da exactamente una violación: {violations:?}"
        );
        assert_cites(&violations[0], &["Q-014", path]);
    }
}

// CA-23: "`engine/core/src/collections.rs` con la misma línea no [da error]". Es
// el único archivo que puede permitir el lint, porque ahí se definen los alias
// `FxHashMap` y `FxHashSet` sobre los tipos de `std` vetados por `clippy.toml`.
#[test]
fn ca_23_collections_rs_de_engine_core_con_allow_disallowed_types_no_da_error() {
    let files = [source(
        "engine/core/src/collections.rs",
        "//! Colecciones con hasher determinista.\n\n#[allow(clippy::disallowed_types)]\npub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;\n",
    )];
    assert_eq!(
        allow_disallowed_types_violations(&files),
        Vec::<String>::new()
    );
}

// CA-23: hay un mensaje por archivo ofensor y ninguno por los demás. Mezcla de
// archivos limpios, el exento y dos ofensores: exactamente dos mensajes, uno por
// cada ruta ofensora. Un archivo que menciona el lint en un comentario sin la
// forma `allow(clippy::disallowed_types)` no cuenta.
//
// Queda fuera, y se documenta aquí sin exigirlo: la variante con espacios
// `#[allow( clippy::disallowed_types )]`. La spec dice "contenga
// `allow(clippy::disallowed_types)`", así que esa variante no tiene por qué
// detectarse; `rustfmt` la normaliza a la forma sin espacios en cualquier caso.
#[test]
fn ca_23_un_mensaje_por_archivo_ofensor_y_ninguno_por_los_limpios() {
    let files = [
        source(
            "engine/core/src/lib.rs",
            "#![forbid(unsafe_code)]\n#![deny(missing_docs)]\npub mod collections;\n",
        ),
        source(
            "engine/core/src/collections.rs",
            "#[allow(clippy::disallowed_types)]\npub type FxHashMap<K, V> = std::collections::HashMap<K, V, FxBuildHasher>;\n",
        ),
        source(
            "engine/sim/src/lib.rs",
            "// `clippy::disallowed_types` veta `HashMap` aquí; usa `FxHashMap`.\n",
        ),
        source(
            "engine/sim/src/grid.rs",
            "#[allow(clippy::disallowed_types)]\npub struct Grid;\n",
        ),
        source(
            "game/rules/src/economy.rs",
            "#![allow(clippy::disallowed_types)]\n",
        ),
    ];
    let violations = allow_disallowed_types_violations(&files);
    assert_eq!(
        violations.len(),
        2,
        "dos archivos ofensores dan dos violaciones: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|m| m.contains("engine/sim/src/grid.rs")),
        "falta la violación de `engine/sim/src/grid.rs`: {violations:?}"
    );
    assert!(
        violations
            .iter()
            .any(|m| m.contains("game/rules/src/economy.rs")),
        "falta la violación de `game/rules/src/economy.rs`: {violations:?}"
    );
    for message in &violations {
        assert_cites(message, &["Q-014"]);
        assert!(
            !message.contains("engine/core/src/collections.rs")
                && !message.contains("engine/sim/src/lib.rs"),
            "un archivo limpio o el exento aparece en una violación: {message:?}"
        );
    }
    assert_eq!(allow_disallowed_types_violations(&[]), Vec::<String>::new());
}

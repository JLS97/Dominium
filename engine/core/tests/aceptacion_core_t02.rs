//! Tests de aceptación de P01-T02: `time`, `handle`, `version`, `collections` y `math`.
//!
//! Cubren CA-1, CA-2, CA-3, CA-4, CA-5, CA-20 y CA-22 de
//! `specs/architecture/engine_core.md`. Solo usan la API pública de `engine_core`
//! y no los modifica el implementador. CA-1 y CA-2 se ejecutan también con
//! `cargo test -p engine_core --release`: los pánicos no dependen del perfil.

use core::num::NonZeroU32;
use core::time::Duration;
use std::fs;
use std::hint::black_box;
use std::path::Path;

use engine_core::collections::{
    FxBuildHasher, FxHashMap, FxHashSet, fx_map_with_capacity, fx_set_with_capacity,
};
use engine_core::handle::{Handle, raw_or_zero};
use engine_core::time::{TICK_DURATION, TICK_RATE_HZ, TICKS_PER_SECOND, Tick, Ticks};
use engine_core::version::{GAME_VERSION, Version};
use proptest::prelude::*;

/// Texto de `time.rs`, vigilado por CA-2: no puede contener flotantes (regla 17).
const TIME_RS: &str = include_str!("../src/time.rs");
/// Texto de `handle.rs`, vigilado por CA-2: no puede contener flotantes (regla 17).
const HANDLE_RS: &str = include_str!("../src/handle.rs");
/// `clippy.toml` de este crate, vigilado por CA-22.
const CORE_CLIPPY_TOML: &str = include_str!("../clippy.toml");

/// Generación no nula a partir de un entero; los tests solo la piden con valores válidos.
fn nz(generation: u32) -> NonZeroU32 {
    NonZeroU32::new(generation).expect("la generación de un handle nunca es 0")
}

/// Handle a partir de dos enteros, para abreviar los tests.
fn handle(slot: u32, generation: u32) -> Handle {
    Handle::new(slot, nz(generation))
}

/// Construye un `FxHashMap` con 10.000 inserciones fijas y devuelve su secuencia de iteración
/// (CA-5). Claves dispersas, para que el orden dependa del hasher y no del de inserción.
fn build_and_iterate() -> Vec<(u64, u64)> {
    let mut map: FxHashMap<u64, u64> = FxHashMap::default();
    for i in 0..10_000_u64 {
        map.insert(i.wrapping_mul(0x9E37_79B9_7F4A_7C15), i);
    }
    map.into_iter().collect()
}

// CA-1: `Tick::ZERO.next() == Tick(1)`; `Tick::ZERO.since(Tick::ZERO) == Ticks::ZERO`;
// `Tick(10).since(Tick(3)) == Ticks(7)`; `Tick(3) + Ticks(4) == Tick(7)`;
// `Tick(7) - Tick(3) == Ticks(4)`; `Tick(7) - Ticks(7) == Tick::ZERO`;
// `Ticks(2) + Ticks(3) == Ticks(5)`; `Ticks(9) - Ticks(4) == Ticks(5)`;
// `Ticks(3) * 4 == Ticks(12)` y `4 * Ticks(3) == Ticks(12)`; `+=` sobre `Tick` y `Ticks`;
// `Tick(3).checked_since(Tick(4)) == None`; `Ticks(3).checked_sub(Ticks(4)) == None`;
// `Tick(0).is_multiple_of(Ticks(5))` y `Tick(10).is_multiple_of(Ticks(5))` ciertos,
// `Tick(7).is_multiple_of(Ticks(5))` falso.
#[test]
fn ca_01_tick_aritmetica_entera() {
    assert_eq!(Tick::ZERO.next(), Tick(1));
    assert_eq!(Tick(41).next(), Tick(42));

    assert_eq!(Tick::ZERO.since(Tick::ZERO), Ticks::ZERO);
    assert_eq!(Tick(10).since(Tick(3)), Ticks(7));
    assert_eq!(Tick(10).since(Tick(10)), Ticks::ZERO);

    assert_eq!(Tick(3) + Ticks(4), Tick(7));
    assert_eq!(Tick(7) - Tick(3), Ticks(4));
    assert_eq!(Tick(7) - Ticks(7), Tick::ZERO);
    assert_eq!(Tick(7) - Ticks::ZERO, Tick(7));

    assert_eq!(Ticks(2) + Ticks(3), Ticks(5));
    assert_eq!(Ticks(9) - Ticks(4), Ticks(5));
    assert_eq!(Ticks(3) * 4, Ticks(12));
    assert_eq!(4 * Ticks(3), Ticks(12));
    assert_eq!(Ticks(3) * black_box(0), Ticks::ZERO);
    assert_eq!(Ticks::ONE, Ticks(1));

    // `+=` sobre `Tick` y sobre `Ticks`.
    let mut tick = Tick(3);
    tick += Ticks(4);
    assert_eq!(tick, Tick(7));
    tick += Ticks::ZERO;
    assert_eq!(tick, Tick(7));
    let mut ticks = Ticks(2);
    ticks += Ticks(3);
    assert_eq!(ticks, Ticks(5));

    // Variantes `checked_*`: `None` al retroceder, `Some` en el caso normal.
    assert_eq!(Tick(3).checked_since(Tick(4)), None);
    assert_eq!(Tick(4).checked_since(Tick(3)), Some(Ticks(1)));
    assert_eq!(Tick(4).checked_since(Tick(4)), Some(Ticks::ZERO));
    assert_eq!(Ticks(3).checked_sub(Ticks(4)), None);
    assert_eq!(Ticks(4).checked_sub(Ticks(3)), Some(Ticks(1)));
    assert_eq!(Ticks(4).checked_sub(Ticks(4)), Some(Ticks::ZERO));

    // El tick 0 es múltiplo de cualquier periodo.
    assert!(Tick(0).is_multiple_of(Ticks(5)));
    assert!(Tick(10).is_multiple_of(Ticks(5)));
    assert!(!Tick(7).is_multiple_of(Ticks(5)));
    assert!(Tick(7).is_multiple_of(Ticks::ONE));
}

// CA-1: `Tick(3).since(Tick(4))` hace pánico `retroceso de ticks`.
// `black_box` impide que el compilador deduzca el pánico al compilar: debe ocurrir al ejecutar,
// en dev y en release.
#[test]
#[should_panic(expected = "retroceso de ticks")]
fn ca_01_since_retroceso_panica() {
    let _ = Tick(black_box(3)).since(Tick(black_box(4)));
}

// CA-1: `Tick(3) - Ticks(4)` hace pánico `retroceso de ticks`.
#[test]
#[should_panic(expected = "retroceso de ticks")]
fn ca_01_resta_de_ticks_panica() {
    let _ = Tick(black_box(3)) - Ticks(black_box(4));
}

// CA-1: `Ticks(3) - Ticks(4)` hace pánico `retroceso de ticks`.
#[test]
#[should_panic(expected = "retroceso de ticks")]
fn ca_01_resta_de_duraciones_panica() {
    let _ = Ticks(black_box(3)) - Ticks(black_box(4));
}

// CA-1: `Tick(1).is_multiple_of(Ticks(0))` hace pánico `periodo cero`.
#[test]
#[should_panic(expected = "periodo cero")]
fn ca_01_periodo_cero_panica() {
    let _ = Tick(black_box(1)).is_multiple_of(Ticks(black_box(0)));
}

// CA-1: `Tick(u64::MAX).next()` hace pánico `desbordamiento de ticks`.
#[test]
#[should_panic(expected = "desbordamiento de ticks")]
fn ca_01_next_desbordamiento_panica() {
    let _ = Tick(black_box(u64::MAX)).next();
}

// CA-1: `Tick(u64::MAX) + Ticks::ONE` hace pánico `desbordamiento de ticks`.
#[test]
#[should_panic(expected = "desbordamiento de ticks")]
fn ca_01_suma_desbordamiento_panica() {
    let _ = Tick(black_box(u64::MAX)) + Ticks::ONE;
}

// CA-1: `Ticks(u64::MAX) * 2` hace pánico `desbordamiento de ticks`.
#[test]
#[should_panic(expected = "desbordamiento de ticks")]
fn ca_01_producto_desbordamiento_panica() {
    let _ = Ticks(black_box(u64::MAX)) * black_box(2);
}

// CA-2: `TICK_RATE_HZ == 20`; `TICK_DURATION == Duration::from_millis(50)`;
// `TICKS_PER_SECOND.to_wall_duration() == Duration::from_secs(1)`;
// `Ticks(u64::MAX).to_wall_duration()` no hace pánico;
// `(TICKS_PER_SECOND * 3_600 * 4) == Ticks(288_000)`; y el texto de
// `engine/core/src/time.rs` y `engine/core/src/handle.rs` no contiene `f32` ni `f64` (regla 17).
#[test]
fn ca_02_frecuencia_y_duracion_del_tick_sin_flotantes() {
    assert_eq!(TICK_RATE_HZ, 20);
    assert_eq!(TICKS_PER_SECOND, Ticks(20));
    assert_eq!(TICK_DURATION, Duration::from_millis(50));

    assert_eq!(TICKS_PER_SECOND.to_wall_duration(), Duration::from_secs(1));
    assert_eq!(Ticks::ZERO.to_wall_duration(), Duration::ZERO);
    assert_eq!(Ticks(3).to_wall_duration(), Duration::from_millis(150));

    // `u64::MAX` ticks no desbordan `Duration`. El valor exacto es `self * TICK_DURATION`,
    // calculado aquí sin pasar por un producto que desborde `u64`.
    let max_wall = Ticks(u64::MAX).to_wall_duration();
    let rest_nanos =
        u32::try_from((u64::MAX % 20) * 50_000_000).expect("resto menor que un segundo");
    let expected = Duration::new(u64::MAX / 20, rest_nanos);
    assert_eq!(max_wall, expected);

    // Cuatro horas de partida en ticks.
    assert_eq!(TICKS_PER_SECOND * 3_600 * 4, Ticks(288_000));

    // Regla 17: ningún flotante en el módulo de tiempo ni en el de handles.
    for (name, text) in [("time.rs", TIME_RS), ("handle.rs", HANDLE_RS)] {
        assert!(!text.is_empty(), "{name} está vacío");
        assert!(!text.contains("f32"), "{name} contiene `f32` (regla 17)");
        assert!(!text.contains("f64"), "{name} contiene `f64` (regla 17)");
    }
}

// CA-3: `size_of::<Handle>() == 8` y `size_of::<Option<Handle>>() == 8`; `new(5, 7)` devuelve
// `slot() == 5`, `index() == 5`, `generation() == 7`, `to_raw() == (5 << 32) | 7`;
// `from_raw(to_raw()) == Some(h)`; `from_raw(5 << 32) == None`; `next_generation(1) == 2` y
// `next_generation(u32::MAX) == 1`; `a < b` equivale a `a.to_raw() < b.to_raw()`;
// `raw_or_zero(None) == 0` y `raw_or_zero(Some(h)) == h.to_raw()`.
#[test]
fn ca_03_handle_ocho_bytes_y_orden_igual_al_raw() {
    assert_eq!(size_of::<Handle>(), 8);
    assert_eq!(size_of::<Option<Handle>>(), 8);

    let h = handle(5, 7);
    assert_eq!(h.slot(), 5);
    assert_eq!(h.index(), 5);
    assert_eq!(h.generation(), nz(7));
    assert_eq!(h.to_raw(), (5 << 32) | 7);
    assert_eq!(Handle::from_raw(h.to_raw()), Some(h));
    assert_eq!(Handle::from_raw((5 << 32) | 7), Some(h));

    // Generación 0 no es un handle.
    assert_eq!(Handle::from_raw(5 << 32), None);
    assert_eq!(Handle::from_raw(0), None);

    // La generación sigue de 1 en 1 y de `u32::MAX` vuelve a 1, nunca a 0.
    assert_eq!(Handle::FIRST_GENERATION, nz(1));
    assert_eq!(Handle::next_generation(nz(1)), nz(2));
    assert_eq!(Handle::next_generation(nz(u32::MAX - 1)), nz(u32::MAX));
    assert_eq!(Handle::next_generation(nz(u32::MAX)), nz(1));

    // Caso límite: mismo slot y distinta generación son distintos y se ordenan por generación.
    let old = handle(5, 1);
    let new = handle(5, 2);
    assert_ne!(old, new);
    assert!(old < new);
    assert!(old.to_raw() < new.to_raw());

    // El slot manda sobre la generación, igual que en el entero `to_raw()`.
    let low_slot = handle(4, u32::MAX);
    let high_slot = handle(5, 1);
    assert!(low_slot < high_slot);
    assert!(low_slot.to_raw() < high_slot.to_raw());

    // Extremos: slot y generación al máximo caben en el `u64`.
    let top = handle(u32::MAX, u32::MAX);
    assert_eq!(top.to_raw(), u64::MAX);
    assert_eq!(Handle::from_raw(u64::MAX), Some(top));

    assert_eq!(raw_or_zero(None), 0);
    assert_eq!(raw_or_zero(Some(h)), h.to_raw());
    assert_eq!(raw_or_zero(Some(top)), u64::MAX);
}

proptest! {
    // CA-3 (propiedad): para cualquier handle `from_raw(to_raw()) == Some(h)`, `to_raw()` es
    // `(slot << 32) | generation`, nunca 0, y para dos handles cualesquiera `a < b` equivale a
    // `a.to_raw() < b.to_raw()`; `raw_or_zero(Some(h)) == h.to_raw()`; `from_raw` con los 32
    // bits bajos a 0 devuelve `None`.
    #[test]
    fn ca_03_handle_raw_ida_y_vuelta_propiedad(
        slot_a in any::<u32>(),
        generation_a in 1..=u32::MAX,
        slot_b in any::<u32>(),
        generation_b in 1..=u32::MAX,
    ) {
        let a = handle(slot_a, generation_a);
        let b = handle(slot_b, generation_b);

        prop_assert_eq!(a.slot(), slot_a);
        prop_assert_eq!(a.index(), slot_a as usize);
        prop_assert_eq!(a.generation(), nz(generation_a));
        prop_assert_eq!(a.to_raw(), (u64::from(slot_a) << 32) | u64::from(generation_a));
        prop_assert_ne!(a.to_raw(), 0);
        prop_assert_eq!(Handle::from_raw(a.to_raw()), Some(a));
        prop_assert_eq!(raw_or_zero(Some(a)), a.to_raw());

        // Orden y igualdad coinciden con los del entero.
        prop_assert_eq!(a < b, a.to_raw() < b.to_raw());
        prop_assert_eq!(a == b, a.to_raw() == b.to_raw());
        prop_assert_eq!(a.cmp(&b), a.to_raw().cmp(&b.to_raw()));

        // Generación 0 nunca es un handle, sea cual sea el slot.
        prop_assert_eq!(Handle::from_raw(u64::from(slot_a) << 32), None);
    }
}

// CA-4: `postcard::to_allocvec` de `Tick(300)` y de `Ticks(300)` da `[0xAC, 0x02]`; el de
// `Handle::new(5, 7)` da `[0x05, 0x07]`; el de `Version::new(0, 1, 0)` da `[0x00, 0x01, 0x00]`;
// y todos vuelven idénticos con `postcard::from_bytes`. Deserializar `[0x05, 0x00]` como
// `Handle` falla.
#[test]
fn ca_04_tick_handle_y_version_postcard_con_bytes_conocidos() {
    let tick = Tick(300);
    let tick_bytes = postcard::to_allocvec(&tick).expect("serializar Tick");
    assert_eq!(tick_bytes, [0xAC, 0x02]);
    assert_eq!(
        postcard::from_bytes::<Tick>(&tick_bytes).expect("deserializar Tick"),
        tick
    );

    let span = Ticks(300);
    let span_bytes = postcard::to_allocvec(&span).expect("serializar Ticks");
    assert_eq!(span_bytes, [0xAC, 0x02]);
    assert_eq!(
        postcard::from_bytes::<Ticks>(&span_bytes).expect("deserializar Ticks"),
        span
    );

    let h = handle(5, 7);
    let handle_bytes = postcard::to_allocvec(&h).expect("serializar Handle");
    assert_eq!(handle_bytes, [0x05, 0x07]);
    assert_eq!(
        postcard::from_bytes::<Handle>(&handle_bytes).expect("deserializar Handle"),
        h
    );

    let version = Version::new(0, 1, 0);
    let version_bytes = postcard::to_allocvec(&version).expect("serializar Version");
    assert_eq!(version_bytes, [0x00, 0x01, 0x00]);
    assert_eq!(
        postcard::from_bytes::<Version>(&version_bytes).expect("deserializar Version"),
        version
    );

    // Una generación 0 es un error del deserializador, no un handle.
    assert!(postcard::from_bytes::<Handle>(&[0x05, 0x00]).is_err());
}

// CA-5: `size_of::<FxBuildHasher>() == 0`; dos `FxHashMap` construidos con `default()` en dos
// hilos distintos, con las mismas 10.000 inserciones en el mismo orden, producen la misma
// secuencia al iterar; `fx_map_with_capacity::<u64, u64>(100).capacity() >= 100` y lo mismo
// para `fx_set_with_capacity`.
#[test]
fn ca_05_colecciones_fx_sin_estado_aleatorio() {
    // Sin estado: el hasher no guarda ninguna semilla por proceso ni por hilo.
    assert_eq!(size_of::<FxBuildHasher>(), 0);

    let in_thread_one = std::thread::spawn(build_and_iterate)
        .join()
        .expect("hilo 1");
    let in_thread_two = std::thread::spawn(build_and_iterate)
        .join()
        .expect("hilo 2");
    let in_main_thread = build_and_iterate();
    assert_eq!(in_thread_one.len(), 10_000);
    assert_eq!(in_thread_one, in_thread_two);
    assert_eq!(in_thread_one, in_main_thread);

    let map: FxHashMap<u64, u64> = fx_map_with_capacity(100);
    assert!(
        map.capacity() >= 100,
        "capacidad del mapa: {}",
        map.capacity()
    );
    assert!(map.is_empty());

    let set: FxHashSet<u64> = fx_set_with_capacity(100);
    assert!(
        set.capacity() >= 100,
        "capacidad del conjunto: {}",
        set.capacity()
    );
    assert!(set.is_empty());

    // Capacidad 0 también es válida y no reserva nada obligatorio.
    let empty: FxHashMap<u64, u64> = fx_map_with_capacity(0);
    assert!(empty.is_empty());
}

// CA-20: `GAME_VERSION` coincide con `env!("CARGO_PKG_VERSION")` del test parseado en tiempo
// de ejecución, y `GAME_VERSION.to_string()` coincide con esa misma cadena.
#[test]
fn ca_20_version_del_juego_igual_a_cargo() {
    let cargo_version = env!("CARGO_PKG_VERSION");
    let mut components = cargo_version.split('.').map(|component| {
        component
            .parse::<u16>()
            .expect("componente de versión numérica")
    });
    let expected = Version {
        major: components.next().expect("major"),
        minor: components.next().expect("minor"),
        patch: components.next().expect("patch"),
    };
    assert!(
        components.next().is_none(),
        "CARGO_PKG_VERSION tiene más de tres componentes"
    );

    assert_eq!(GAME_VERSION, expected);
    assert_eq!(GAME_VERSION.to_string(), cargo_version);

    // `new` rellena los tres campos y `Display` escribe `major.minor.patch` sin relleno.
    assert_eq!(
        Version::new(1, 2, 3),
        Version {
            major: 1,
            minor: 2,
            patch: 3
        }
    );
    assert_eq!(Version::new(0, 1, 0).to_string(), "0.1.0");
    assert_eq!(Version::new(1, 20, 300).to_string(), "1.20.300");
    assert_eq!(
        Version::new(u16::MAX, u16::MAX, u16::MAX).to_string(),
        format!("{0}.{0}.{0}", u16::MAX)
    );
}

// CA-22: existen `engine/core/clippy.toml`, `engine/sim/clippy.toml` y `game/rules/clippy.toml`,
// idénticos, con `disallowed-types` para `std::collections::HashMap`,
// `std::collections::HashSet` y `std::hash::RandomState` con un motivo que cite la regla 13 y el
// ADR-0009, y para `glam::Vec3A`, `glam::Mat3A` y `glam::Affine3A` con un motivo que cite la
// regla 6 (Q-016, P01-T11). El propio se lee con `include_str!`, los otros dos desde
// `CARGO_MANIFEST_DIR`. Las dos pruebas negativas con `cargo clippy` las hace el orquestador.
#[test]
fn ca_22_tres_clippy_toml_identicos() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sim_path = manifest_dir.join("..").join("sim").join("clippy.toml");
    let rules_path = manifest_dir
        .join("..")
        .join("..")
        .join("game")
        .join("rules")
        .join("clippy.toml");
    let sim = fs::read_to_string(&sim_path)
        .unwrap_or_else(|e| panic!("no se puede leer {}: {e}", sim_path.display()));
    let rules = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("no se puede leer {}: {e}", rules_path.display()));

    assert!(
        !CORE_CLIPPY_TOML.trim().is_empty(),
        "engine/core/clippy.toml está vacío"
    );
    assert_eq!(
        CORE_CLIPPY_TOML, sim,
        "engine/sim/clippy.toml difiere de engine/core/clippy.toml"
    );
    assert_eq!(
        CORE_CLIPPY_TOML, rules,
        "game/rules/clippy.toml difiere de engine/core/clippy.toml"
    );

    assert!(CORE_CLIPPY_TOML.contains("disallowed-types"));
    // Seis rutas desde P01-T11 (Q-016): tres de `std` por la regla 13 y tres de `glam` por la regla 6.
    for forbidden in [
        "std::collections::HashMap",
        "std::collections::HashSet",
        "std::hash::RandomState",
        "glam::Vec3A",
        "glam::Mat3A",
        "glam::Affine3A",
    ] {
        assert!(
            CORE_CLIPPY_TOML.contains(forbidden),
            "falta `{forbidden}` en clippy.toml"
        );
    }

    // Cada tipo prohibido lleva un motivo: los de `std` citan la regla 13 y el ADR-0009; los de `glam`, la regla 6.
    let entries: Vec<&str> = CORE_CLIPPY_TOML
        .lines()
        .filter(|line| line.contains("path = \""))
        .collect();
    assert_eq!(
        entries.len(),
        6,
        "se esperan exactamente seis tipos prohibidos"
    );
    for entry in entries {
        assert!(entry.contains("reason = \""), "entrada sin motivo: {entry}");
        if entry.contains("path = \"std::") {
            assert!(
                entry.contains("regla 13"),
                "el motivo no cita la regla 13: {entry}"
            );
            assert!(
                entry.contains("ADR-0009"),
                "el motivo no cita el ADR-0009: {entry}"
            );
        } else {
            assert!(
                entry.contains("path = \"glam::"),
                "ruta prohibida fuera de `std` y `glam`: {entry}"
            );
            assert!(
                entry.contains("regla 6"),
                "el motivo no cita la regla 6: {entry}"
            );
        }
    }
}

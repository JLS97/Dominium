//! Tests de aceptación de P01-T06: `rng`.
//!
//! Cubren CA-9 y CA-10 de `specs/architecture/engine_core.md`. Solo usan la API
//! pública de `engine_core` y no los modifica el implementador. La referencia es
//! la implementación `xxhash_rust::xxh3::xxh3_64` sobre los 32 bytes que fija el
//! ADR-0009, y el valor dorado de CA-9 para las cuatro entradas a cero. Nada sale
//! de la aleatoriedad del sistema: `proptest` va con semilla fija y el generador
//! de los 1.000 casos de sensibilidad es un xorshift con semilla fija escrito aquí.
//!
//! Los `const fn` del contrato (`Purpose::new`, `nth`, `id`, `index`,
//! `Draw::from_bits`, `bits`, `below`, `chance`, `split`) se llaman desde `const fn`
//! de este archivo. Si la implementación dejara de ser `const`, el test dejaría de
//! compilar. No se usan `const _: () = assert!(...)` porque evaluarían el cuerpo en
//! tiempo de compilación, y con el contrato pendiente (`panic!`) el archivo no
//! compilaría; la llamada desde un `const fn` exige la constancia sin evaluar nada
//! hasta que el test corre.

use core::num::NonZeroU32;

use engine_core::handle::{Handle, raw_or_zero};
use engine_core::rng::{Draw, Purpose, Seed, draw, random_u64};
use engine_core::time::Tick;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestError, TestRunner};
use xxhash_rust::xxh3::xxh3_64;

/// Valor fijado por CA-9: `xxh3_64` de 32 bytes a cero.
const XXH3_32_ZEROS: u64 = 0xA057_271C_9071_C99D;
/// Semilla fija de `proptest` y del xorshift. Misma constante que en T05 para que cada
/// ejecución repita los mismos casos.
const GENERATOR_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
/// Combinaciones de CA-9 generadas con `proptest`.
const COMBINATION_CASES: u32 = 1_000;
/// Casos de sensibilidad de CA-9: cambiar un solo campo cambia el valor en al menos
/// `SENSITIVITY_MIN_CHANGED` de ellos.
const SENSITIVITY_CASES: u32 = 1_000;
/// Cota de CA-9 para la sensibilidad: 999 de 1.000. Con un hash de 64 bits, una colisión
/// tiene probabilidad del orden de 2^-64 por caso; la cota solo deja margen a una.
const SENSITIVITY_MIN_CHANGED: u32 = 999;
/// Casos de `proptest` para las propiedades de rango de `Draw`.
const RANGE_CASES: u32 = 1_000;
/// CA-10: ticks por tabla.
const TABLE_TICKS: u64 = 10_000;
/// CA-10: sujetos por tabla, `None` y 15 handles distintos.
const TABLE_SUBJECTS: usize = 16;
/// CA-10: propósitos por tabla.
const TABLE_PURPOSES: usize = 4;
/// CA-10: hilos de `std::thread` que calculan la tabla.
const TABLE_THREADS: usize = 4;
/// CA-10: semilla común a los cinco cálculos.
const TABLE_SEED: Seed = Seed(0x5EED_C0DE_F00D_BA5E);
/// 2^24 y 2^53, los divisores de `unit_f32` y `unit_f64`.
const TWO_POW_24: u64 = 1 << 24;
const TWO_POW_53: u64 = 1 << 53;

/// Generación no nula a partir de un entero; los tests solo la piden con valores válidos.
fn nz(generation: u32) -> NonZeroU32 {
    NonZeroU32::new(generation).expect("la generación de un handle nunca es 0")
}

/// Generador xorshift64 con semilla fija para los casos de sensibilidad. No es el
/// generador del motor: solo produce entradas repetibles para el test.
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> XorShift64 {
        XorShift64(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Un `u64` distinto de 0, para usar como delta de un XOR que cambie el valor.
    fn nonzero_u64(&mut self) -> u64 {
        loop {
            let value = self.next_u64();
            if value != 0 {
                return value;
            }
        }
    }

    /// Un `u32` distinto de 0.
    fn nonzero_u32(&mut self) -> u32 {
        loop {
            let value = u32::try_from(self.next_u64() >> 32).expect("32 bits altos");
            if value != 0 {
                return value;
            }
        }
    }

    /// Un handle con slot arbitrario y generación no nula.
    fn handle(&mut self) -> Handle {
        let slot = u32::try_from(self.next_u64() >> 32).expect("32 bits altos");
        Handle::new(slot, nz(self.nonzero_u32()))
    }

    /// `None` una de cada cuatro veces; si no, un handle arbitrario.
    fn subject(&mut self) -> Option<Handle> {
        if self.next_u64().is_multiple_of(4) {
            None
        } else {
            Some(self.handle())
        }
    }
}

/// Configuración de `proptest`: semilla fija, sin archivo de regresiones (los casos ya son
/// repetibles por la semilla, y así una ejecución que falle no deja archivos en el worktree).
fn config(cases: u32) -> Config {
    Config {
        cases,
        source_file: Some(file!()),
        rng_seed: RngSeed::Fixed(GENERATOR_SEED),
        failure_persistence: None,
        ..Config::default()
    }
}

/// Ejecuta `property` sobre `cases` valores de `strategy` y hace pánico con el caso mínimo
/// si falla.
fn check<T, S>(
    label: &str,
    cases: u32,
    strategy: S,
    property: impl Fn(T) -> Result<(), TestCaseError>,
) where
    T: core::fmt::Debug,
    S: Strategy<Value = T>,
{
    let mut runner = TestRunner::new(config(cases));
    match runner.run(&strategy, property) {
        Ok(()) => {}
        Err(TestError::Fail(reason, value)) => panic!("{label}: {reason}; caso mínimo: {value:?}"),
        Err(TestError::Abort(reason)) => panic!("{label} abortado: {reason}"),
    }
}

/// Handles con cualquier slot y cualquier generación no nula.
fn any_handle() -> impl Strategy<Value = Handle> {
    (any::<u32>(), 1..=u32::MAX).prop_map(|(slot, generation)| Handle::new(slot, nz(generation)))
}

/// Sujetos: `None` con peso 1 y un handle arbitrario con peso 4, para que los 1.000 casos
/// incluyan las dos ramas.
fn any_subject() -> impl Strategy<Value = Option<Handle>> {
    prop_oneof![1 => Just(None), 4 => any_handle().prop_map(Some)]
}

/// Los 32 bytes que `random_u64` hashea según el ADR-0009: `seed.0`, `tick.0`,
/// `raw_or_zero(subject)` y `purpose.0`, cada uno como `u64` little-endian y en ese orden.
fn reference_bytes(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&seed.0.to_le_bytes());
    bytes[8..16].copy_from_slice(&tick.0.to_le_bytes());
    bytes[16..24].copy_from_slice(&raw_or_zero(subject).to_le_bytes());
    bytes[24..32].copy_from_slice(&purpose.0.to_le_bytes());
    bytes
}

/// La referencia completa: `xxh3_64` con semilla 0 de los 32 bytes.
fn reference_random_u64(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> u64 {
    xxh3_64(&reference_bytes(seed, tick, subject, purpose))
}

// Envoltorios `const fn`: obligan a que las funciones del contrato sigan siendo `const`
// (ver la cabecera del archivo). Se evalúan en tiempo de ejecución.

/// `Purpose::new(7).nth(3)`, construido en contexto `const`.
const fn purpose_7_nth_3() -> Purpose {
    Purpose::new(7).nth(3)
}

/// `(id, index)` de un propósito, leídos en contexto `const`.
const fn purpose_parts(purpose: Purpose) -> (u32, u32) {
    (purpose.id(), purpose.index())
}

/// `Purpose::new(id).nth(index)`, en contexto `const`.
const fn purpose_at(id: u32, index: u32) -> Purpose {
    Purpose::new(id).nth(index)
}

/// `Draw::from_bits(bits).bits()`, en contexto `const`.
const fn draw_bits_roundtrip(bits: u64) -> u64 {
    Draw::from_bits(bits).bits()
}

/// `below`, en contexto `const`.
const fn draw_below(bits: u64, n: u32) -> u32 {
    Draw::from_bits(bits).below(n)
}

/// `chance`, en contexto `const`.
const fn draw_chance(bits: u64, numerator: u32, denominator: u32) -> bool {
    Draw::from_bits(bits).chance(numerator, denominator)
}

/// `split`, en contexto `const`.
const fn draw_split(bits: u64) -> (u32, u32) {
    Draw::from_bits(bits).split()
}

/// `below(n)` según la definición de la spec: `((bits as u128 * n as u128) >> 64) as u32`.
fn reference_below(bits: u64, n: u32) -> u32 {
    u32::try_from((u128::from(bits) * u128::from(n)) >> 64)
        .expect("el producto desplazado 64 bits cabe en u32")
}

/// `unit_f32` según la spec: los 24 bits altos divididos por 2^24.
#[allow(
    clippy::cast_precision_loss,
    reason = "los 24 bits altos caben exactos en la mantisa de f32, y 2^24 también"
)]
fn reference_unit_f32(bits: u64) -> f32 {
    (bits >> 40) as f32 / TWO_POW_24 as f32
}

/// `unit_f64` según la spec: los 53 bits altos divididos por 2^53.
#[allow(
    clippy::cast_precision_loss,
    reason = "los 53 bits altos caben exactos en la mantisa de f64, y 2^53 también"
)]
fn reference_unit_f64(bits: u64) -> f64 {
    (bits >> 11) as f64 / TWO_POW_53 as f64
}

/// CA-9, parte 1: el valor dorado de las cuatro entradas a cero, igual a la referencia.
fn assert_golden_zero() {
    assert_eq!(
        xxh3_64(&[0u8; 32]),
        XXH3_32_ZEROS,
        "la referencia coincide con el valor fijado en CA-9"
    );
    assert_eq!(
        random_u64(Seed(0), Tick::ZERO, None, Purpose(0)),
        XXH3_32_ZEROS,
        "random_u64 con todo a cero es el xxh3_64 de 32 bytes a cero"
    );
    assert_eq!(
        draw(Seed(0), Tick::ZERO, None, Purpose(0)),
        Draw::from_bits(XXH3_32_ZEROS),
        "draw envuelve random_u64 con todo a cero"
    );
}

/// CA-9, parte 2: 1.000 combinaciones de `proptest`, con `subject` en `Option<Handle>`,
/// igualan al `xxh3_64` de los 32 bytes little-endian; `draw` envuelve el mismo valor.
fn assert_matches_reference_on_combinations() {
    let strategy = (any::<u64>(), any::<u64>(), any_subject(), any::<u64>()).prop_map(
        |(seed, tick, subject, purpose)| (Seed(seed), Tick(tick), subject, Purpose(purpose)),
    );
    check(
        "random_u64 contra xxh3_64 de 32 bytes",
        COMBINATION_CASES,
        strategy,
        |(seed, tick, subject, purpose)| {
            let expected = reference_random_u64(seed, tick, subject, purpose);
            let actual = random_u64(seed, tick, subject, purpose);
            prop_assert_eq!(actual, expected, "random_u64 difiere de la referencia");
            prop_assert_eq!(
                random_u64(seed, tick, subject, purpose),
                actual,
                "random_u64 no es pura: dos llamadas iguales dan valores distintos"
            );
            prop_assert_eq!(
                draw(seed, tick, subject, purpose).bits(),
                expected,
                "draw no envuelve random_u64"
            );
            prop_assert_eq!(
                draw(seed, tick, subject, purpose),
                Draw::from_bits(expected),
                "draw != from_bits"
            );
            Ok(())
        },
    );
}

/// CA-9, parte 3: cambiar uno solo de los cuatro campos cambia el valor en al menos 999 de
/// 1.000 casos, contado por separado para cada campo. Los deltas son distintos de cero, y en
/// el sujeto el `raw_or_zero` del variante siempre difiere del original.
fn assert_each_input_changes_the_value() {
    let mut generator = XorShift64::new(GENERATOR_SEED);
    let mut changed_by_seed = 0u32;
    let mut changed_by_tick = 0u32;
    let mut changed_by_subject = 0u32;
    let mut changed_by_purpose = 0u32;
    let mut none_subjects = 0u32;

    for _ in 0..SENSITIVITY_CASES {
        let seed = Seed(generator.next_u64());
        let tick = Tick(generator.next_u64());
        let subject = generator.subject();
        let purpose = Purpose(generator.next_u64());
        let base = random_u64(seed, tick, subject, purpose);

        let other_seed = Seed(seed.0 ^ generator.nonzero_u64());
        let other_tick = Tick(tick.0 ^ generator.nonzero_u64());
        let other_subject = match subject {
            None => {
                none_subjects += 1;
                Some(generator.handle())
            }
            Some(handle) => Some(Handle::new(
                handle.slot() ^ generator.nonzero_u32(),
                handle.generation(),
            )),
        };
        let other_purpose = Purpose(purpose.0 ^ generator.nonzero_u64());

        assert_ne!(other_seed, seed);
        assert_ne!(other_tick, tick);
        assert_ne!(raw_or_zero(other_subject), raw_or_zero(subject));
        assert_ne!(other_purpose, purpose);

        changed_by_seed += u32::from(random_u64(other_seed, tick, subject, purpose) != base);
        changed_by_tick += u32::from(random_u64(seed, other_tick, subject, purpose) != base);
        changed_by_subject += u32::from(random_u64(seed, tick, other_subject, purpose) != base);
        changed_by_purpose += u32::from(random_u64(seed, tick, subject, other_purpose) != base);
    }

    assert!(
        none_subjects > 0,
        "los casos de sensibilidad incluyen sujetos `None`"
    );
    assert!(
        changed_by_seed >= SENSITIVITY_MIN_CHANGED,
        "cambiar la semilla cambió el valor en {changed_by_seed} de {SENSITIVITY_CASES} casos"
    );
    assert!(
        changed_by_tick >= SENSITIVITY_MIN_CHANGED,
        "cambiar el tick cambió el valor en {changed_by_tick} de {SENSITIVITY_CASES} casos"
    );
    assert!(
        changed_by_subject >= SENSITIVITY_MIN_CHANGED,
        "cambiar el sujeto cambió el valor en {changed_by_subject} de {SENSITIVITY_CASES} casos"
    );
    assert!(
        changed_by_purpose >= SENSITIVITY_MIN_CHANGED,
        "cambiar el propósito cambió el valor en {changed_by_purpose} de {SENSITIVITY_CASES} casos"
    );
}

/// CA-9, parte 4: `Purpose::new(7).nth(3)` tiene `id() == 7`, `index() == 3` y
/// `.0 == (7 << 32) | 3`. Y el layout de `new` y `nth` en general: `new(id).0 == id << 32`,
/// `nth` conserva el id y sustituye el índice, y `nth` sobre `nth` se queda con el último.
fn assert_purpose_layout() {
    let purpose = purpose_7_nth_3();
    assert_eq!(
        purpose_parts(purpose),
        (7, 3),
        "Purpose::new(7).nth(3) tiene id 7 e índice 3"
    );
    assert_eq!(purpose.id(), 7);
    assert_eq!(purpose.index(), 3);
    assert_eq!(
        purpose.0,
        (7_u64 << 32) | 3,
        "Purpose::new(7).nth(3).0 es (7 << 32) | 3"
    );

    assert_eq!(Purpose::new(7).0, 7_u64 << 32, "new deja el índice a 0");
    assert_eq!(Purpose::new(7).index(), 0);
    assert_eq!(
        Purpose::new(7).nth(0),
        Purpose::new(7),
        "nth(0) es el propio new"
    );
    assert_eq!(
        Purpose::new(0).nth(0),
        Purpose(0),
        "new(0) es el propósito 0 de CA-9"
    );

    let extremes = purpose_at(u32::MAX, u32::MAX);
    assert_eq!(
        extremes.0,
        u64::MAX,
        "id e índice máximos ocupan los 64 bits"
    );
    assert_eq!(purpose_parts(extremes), (u32::MAX, u32::MAX));
    assert_eq!(
        Purpose::new(u32::MAX).nth(0).0,
        u64::from(u32::MAX) << 32,
        "el id no invade el índice"
    );
    assert_eq!(
        Purpose::new(0).nth(u32::MAX).0,
        u64::from(u32::MAX),
        "el índice no invade el id"
    );
    assert_eq!(
        Purpose::new(7).nth(3).nth(9),
        Purpose::new(7).nth(9),
        "nth sobre nth sustituye el índice"
    );
    assert_eq!(
        Purpose(0x0000_0007_0000_0003),
        purpose,
        "el campo público coincide con el layout"
    );
}

/// CA-9, parte 5: los extremos de `Draw` fijados por la spec y los casos límite de
/// "Aleatoriedad": `from_bits(u64::MAX).unit_f32() == 1 - 2^-24`, `below(1) == 0` y
/// `below(u32::MAX)` nunca es `u32::MAX`.
fn assert_draw_extremes() {
    let top = Draw::from_bits(u64::MAX);
    assert_eq!(
        top.unit_f32().to_bits(),
        (1.0_f32 - 2_f32.powi(-24)).to_bits(),
        "unit_f32 de u64::MAX es exactamente 1 - 2^-24"
    );
    assert_eq!(
        top.unit_f64().to_bits(),
        (1.0_f64 - 2_f64.powi(-53)).to_bits(),
        "unit_f64 de u64::MAX es exactamente 1 - 2^-53"
    );
    assert!(top.unit_f32() < 1.0, "unit_f32 nunca llega a 1");
    assert!(top.unit_f64() < 1.0, "unit_f64 nunca llega a 1");
    assert_eq!(
        top.below(u32::MAX),
        u32::MAX - 1,
        "below(u32::MAX) del máximo es u32::MAX - 1"
    );
    assert_eq!(
        top.below(1),
        0,
        "below(1) == 0 incluso con todos los bits a 1"
    );
    assert_eq!(draw_below(u64::MAX, 10), 9);
    assert_eq!(draw_split(u64::MAX), (u32::MAX, u32::MAX));

    let bottom = Draw::from_bits(0);
    assert_eq!(
        bottom.unit_f32().to_bits(),
        0.0_f32.to_bits(),
        "unit_f32 de 0 es 0"
    );
    assert_eq!(
        bottom.unit_f64().to_bits(),
        0.0_f64.to_bits(),
        "unit_f64 de 0 es 0"
    );
    assert_eq!(bottom.below(u32::MAX), 0);
    assert_eq!(draw_below(0, 1), 0);
    assert!(!draw_chance(0, 0, 1), "chance(0, 1) es falso");
    assert!(draw_chance(0, 1, 1), "chance(1, 1) es cierto");
    assert_eq!(draw_split(0), (0, 0));

    let half = Draw::from_bits(1 << 63);
    assert_eq!(
        half.unit_f32().to_bits(),
        0.5_f32.to_bits(),
        "el bit alto solo vale 0.5"
    );
    assert_eq!(half.unit_f64().to_bits(), 0.5_f64.to_bits());
    assert_eq!(half.below(2), 1, "la mitad alta cae en el segundo cubo");
    assert_eq!(half.below(u32::MAX), (u32::MAX - 1) / 2);
    assert_eq!(draw_split(1 << 63), (1 << 31, 0));
    assert_eq!(
        draw_split(0x1234_5678_9ABC_DEF0),
        (0x1234_5678, 0x9ABC_DEF0),
        "split es (alto, bajo)"
    );
    assert_eq!(
        draw_bits_roundtrip(0x1234_5678_9ABC_DEF0),
        0x1234_5678_9ABC_DEF0,
        "bits devuelve from_bits"
    );
}

// CA-9: `random_u64(Seed(0), Tick::ZERO, None, Purpose(0)) == 0xA057_271C_9071_C99D`, el `xxh3_64` de 32
// bytes a cero; `random_u64(seed, tick, subject, purpose)` es igual a `xxh3_64` de los 32 bytes `seed.0`,
// `tick.0`, `raw_or_zero(subject)`, `purpose.0` en little-endian, para 1.000 combinaciones generadas con
// `proptest` con `subject` en `Option<Handle>` (incluido `None`); cambiar uno solo de los cuatro cambia el
// valor en al menos 999 de 1.000 casos; `Purpose` no implementa `Default` (por revisión, Q-019);
// `Purpose::new(7).nth(3)` tiene `id() == 7`, `index() == 3` y `.0 == (7 << 32) | 3`; [...]
// `Draw::from_bits(u64::MAX).unit_f32() == 1.0 - 2f32.powi(-24)`; [...]
#[test]
fn ca_09_rng_sin_estado_es_xxh3_de_32_bytes() {
    assert_golden_zero();
    assert_matches_reference_on_combinations();
    assert_each_input_changes_the_value();
    assert_purpose_layout();
    assert_draw_extremes();
}

// CA-9: para cualquier `Draw`: `unit_f32()` y `unit_f64()` están en `[0, 1)`, `below(n) < n` para `n` en
// `1..=u32::MAX`, `below(1) == 0`, `chance(0, d)` es falso y `chance(d, d)` cierto. Además, cada conversión
// coincide con la fórmula que fija la spec: 24 o 53 bits altos sobre 2^24 o 2^53, `below` como producto
// de 128 bits desplazado 64, `chance(k, d)` como `below(d) < k`, `split` como `(alto, bajo)` y `bits`
// como inversa de `from_bits`.
#[test]
fn ca_09_rng_rangos_propiedad() {
    // Bits con peso extra en los extremos, que son donde fallan los redondeos.
    let bits = prop_oneof![1 => Just(0_u64), 1 => Just(u64::MAX), 8 => any::<u64>()];
    let n = prop_oneof![1 => Just(1_u32), 1 => Just(u32::MAX), 8 => 1..=u32::MAX];
    let denominator = prop_oneof![1 => Just(1_u32), 1 => Just(u32::MAX), 8 => 1..=u32::MAX];
    let strategy = (bits, n, denominator, any::<u32>());

    check(
        "rangos de Draw",
        RANGE_CASES,
        strategy,
        |(bits, n, denominator, numerator_bits)| {
            let value = Draw::from_bits(bits);
            prop_assert_eq!(value.bits(), bits, "bits no es la inversa de from_bits");
            prop_assert_eq!(draw_bits_roundtrip(bits), bits);

            let unit_f32 = value.unit_f32();
            prop_assert!(
                (0.0..1.0).contains(&unit_f32),
                "unit_f32 fuera de [0, 1): {}",
                unit_f32
            );
            prop_assert_eq!(
                unit_f32.to_bits(),
                reference_unit_f32(bits).to_bits(),
                "unit_f32 no es bits >> 40 / 2^24"
            );

            let unit_f64 = value.unit_f64();
            prop_assert!(
                (0.0..1.0).contains(&unit_f64),
                "unit_f64 fuera de [0, 1): {}",
                unit_f64
            );
            prop_assert_eq!(
                unit_f64.to_bits(),
                reference_unit_f64(bits).to_bits(),
                "unit_f64 no es bits >> 11 / 2^53"
            );

            let below = value.below(n);
            prop_assert!(below < n, "below({}) devolvió {}", n, below);
            prop_assert_eq!(
                below,
                reference_below(bits, n),
                "below no sigue la fórmula de la spec"
            );
            prop_assert_eq!(draw_below(bits, n), below);
            prop_assert_eq!(value.below(1), 0, "below(1) != 0");
            prop_assert!(
                value.below(u32::MAX) < u32::MAX,
                "below(u32::MAX) devolvió u32::MAX"
            );

            prop_assert!(!value.chance(0, denominator), "chance(0, d) es cierto");
            prop_assert!(
                value.chance(denominator, denominator),
                "chance(d, d) es falso"
            );
            prop_assert!(!draw_chance(bits, 0, denominator));
            prop_assert!(draw_chance(bits, denominator, denominator));
            let numerator = numerator_bits % denominator.saturating_add(1);
            prop_assert_eq!(
                value.chance(numerator, denominator),
                value.below(denominator) < numerator,
                "chance(k, d) no es below(d) < k"
            );

            let (high, low) = value.split();
            prop_assert_eq!(u64::from(high), bits >> 32, "split: parte alta");
            prop_assert_eq!(u64::from(low), bits & 0xFFFF_FFFF, "split: parte baja");
            prop_assert_eq!(draw_split(bits), (high, low));
            Ok(())
        },
    );
}

// CA-9: `below(0)` hace pánico con `rango vacío`.
#[test]
#[should_panic(expected = "rango vacío")]
fn ca_09_below_cero_panica() {
    let _ = Draw::from_bits(0x1234_5678_9ABC_DEF0).below(0);
}

// CA-9: `chance(1, 0)` hace pánico con `rango vacío`.
#[test]
#[should_panic(expected = "rango vacío")]
fn ca_09_chance_denominador_cero_panica() {
    let _ = Draw::from_bits(0x1234_5678_9ABC_DEF0).chance(1, 0);
}

/// Los 16 sujetos de CA-10: `None` y 15 handles distintos entre sí.
fn table_subjects() -> [Option<Handle>; TABLE_SUBJECTS] {
    let mut subjects = [None; TABLE_SUBJECTS];
    for (i, subject) in subjects.iter_mut().enumerate().skip(1) {
        let i = u32::try_from(i).expect("16 sujetos");
        // Slot y generación distintos por sujeto, para que ningún `to_raw` se repita.
        *subject = Some(Handle::new(i * 1_000, nz(i * 7 + 1)));
    }
    subjects
}

/// Los 4 propósitos de CA-10: dos ids con dos índices cada uno.
fn table_purposes() -> [Purpose; TABLE_PURPOSES] {
    [
        Purpose::new(1),
        Purpose::new(1).nth(1),
        Purpose::new(2),
        Purpose::new(2).nth(5),
    ]
}

/// La tabla de `random_u64` para 10.000 ticks × 16 sujetos × 4 propósitos, en orden fijo
/// (tick, sujeto, propósito).
fn rng_table(seed: Seed) -> Vec<u64> {
    let subjects = table_subjects();
    let purposes = table_purposes();
    let mut table = Vec::with_capacity(
        usize::try_from(TABLE_TICKS).expect("cabe") * TABLE_SUBJECTS * TABLE_PURPOSES,
    );
    for tick in 0..TABLE_TICKS {
        for &subject in &subjects {
            for &purpose in &purposes {
                table.push(random_u64(seed, Tick(tick), subject, purpose));
            }
        }
    }
    table
}

// CA-10: Cuatro hilos de `std::thread` calculan cada uno la tabla de `random_u64` para 10.000 ticks, 16
// sujetos (`None` y 15 handles distintos) y 4 propósitos con la misma semilla, y las cuatro tablas son
// idénticas entre sí y a la calculada en el hilo principal.
#[test]
fn ca_10_rng_identico_en_varios_hilos() {
    let subjects = table_subjects();
    let raws: Vec<u64> = subjects
        .iter()
        .map(|&subject| raw_or_zero(subject))
        .collect();
    for (i, raw) in raws.iter().enumerate() {
        assert!(!raws[..i].contains(raw), "los 16 sujetos son distintos");
    }

    let main_table = rng_table(TABLE_SEED);
    let expected_len =
        usize::try_from(TABLE_TICKS).expect("cabe") * TABLE_SUBJECTS * TABLE_PURPOSES;
    assert_eq!(
        main_table.len(),
        expected_len,
        "la tabla tiene 10.000 × 16 × 4 valores"
    );

    let thread_tables: Vec<Vec<u64>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..TABLE_THREADS)
            .map(|_| scope.spawn(|| rng_table(TABLE_SEED)))
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("el hilo calculó la tabla sin pánico"))
            .collect()
    });

    assert_eq!(thread_tables.len(), TABLE_THREADS);
    for (i, table) in thread_tables.iter().enumerate() {
        assert_eq!(table.len(), expected_len, "hilo {i}: longitud de la tabla");
        assert!(
            *table == main_table,
            "hilo {i}: la tabla difiere de la del hilo principal"
        );
    }
    for (i, table) in thread_tables.iter().enumerate().skip(1) {
        assert!(
            *table == thread_tables[0],
            "hilo {i}: la tabla difiere de la del hilo 0"
        );
    }

    // Guarda contra un stub constante: la tabla de un hash no es un solo valor repetido.
    let first = main_table[0];
    assert!(
        main_table.iter().any(|&value| value != first),
        "la tabla no es constante"
    );
    // Y la primera entrada coincide con la referencia, igual que en CA-9.
    assert_eq!(
        first,
        reference_random_u64(TABLE_SEED, Tick::ZERO, None, Purpose::new(1))
    );
}

# Crate `engine_core`

- **Carpeta:** `engine/core`
- **Responsabilidad:** la base compartida del motor: tiempo en ticks enteros, el handle común a todas las tablas, hashers deterministas, aleatoriedad sin estado, el sobre binario de guardados y replays, la carga de RON con su hash de datos, el perfilado y los tipos matemáticos. No conoce entidades, tablas, gráficos ni juego.
- **Reglas de la constitución que le afectan:**
  - **1 y 16.** `engine_core` no depende de ningún crate del workspace. Es la raíz del grafo.
  - **2.** No depende de nada gráfico ni de ventana. Como `engine_sim` depende de él, cualquier dependencia que entre aquí entra en la simulación.
  - **3.** `#![forbid(unsafe_code)]` en `src/lib.rs`. Lo comprueba `check-arch`.
  - **4 y 17.** Aquí viven el tipo `Tick`, sobre `u64`, la duración `Ticks` como diferencia de dos `Tick`, y la frecuencia de 20 Hz, definida una sola vez (ADR-0006). La API de tiempo no expone flotantes.
  - **6.** Los tipos que forman parte del estado canónico (`Tick`, `Ticks`, `Handle`, `Seed`, `DataHash`, `StateHash`, `Version`) implementan `Serialize` y `Deserialize` con una codificación `postcard` estable y con vectores de bytes fijados en los criterios.
  - **7.** Sin estado global mutable: ni `static mut`, ni contadores atómicos, ni cachés. El único estado de proceso es el cliente de Tracy, que vive dentro de su librería, arranca con el proceso y no influye en la simulación.
  - **11 y 18.** Da las primitivas para cargar RON y para calcular el hash de datos que llevan guardados y replays, y rechaza un sobre cuyo hash de datos no coincide. Qué archivos existen y qué contienen lo deciden las specs de features.
  - **13.** Define los hashers fijos (ADR-0009), el hash de estado por bytes en orden fijo y la aleatoriedad sin estado `hash(semilla, tick, handle, propósito)`. Nada de aquí depende del número de hilos ni del reloj, y toda la aritmética de ticks hace lo mismo en cualquier perfil de compilación.
  - **19.** Ninguna primitiva de este crate asigna memoria, salvo las que construyen, codifican o decodifican buffers o leen archivos, que lo dicen en su firma: `fx_map_with_capacity`, `fx_set_with_capacity`, `encode`, `encode_to_vec`, `decode`, `decode_body`, `parse_ron`, `load_ron`, `list_data_files` y `hash_data_tree`.
  - **12.** El hash de estado tiene una estimación de coste heredada del ADR-0009, que el hito 2 convierte o no en presupuesto.
- **ADR relacionados:** 0001 (Rust y `forbid(unsafe_code)`), 0004 (handle común sin ECS), 0006 (tick a 20 Hz definido aquí), 0007 (`postcard` con cabecera propia; RON solo para autoría), 0009 (hashers deterministas, `bytemuck` y envoltorios en este crate; respuestas Q-006 a Q-011).
- **Estado:** aprobada

## Qué hace

Un módulo por responsabilidad, cada uno en su archivo bajo `src/`:

| Módulo | Archivo | Contenido |
| --- | --- | --- |
| `time` | `src/time.rs` | `Tick`, `Ticks`, `TICK_RATE_HZ`, `TICKS_PER_SECOND`, `TICK_DURATION` |
| `handle` | `src/handle.rs` | `Handle` (slot y generación) y su forma cruda de 64 bits |
| `collections` | `src/collections.rs` | `FxHashMap`, `FxHashSet` y `FxBuildHasher` (ADR-0009) |
| `hash` | `src/hash.rs` | `StateHasher`, `StateHash`, el trait `HashState` para el hash de estado por tick y por sistema, y el reexport de `bytemuck` |
| `rng` | `src/rng.rs` | `Seed`, `Purpose`, `Draw` y `random_u64`: aleatoriedad sin estado |
| `save` | `src/save.rs` | el sobre binario de guardados y replays: `MAGIC`, `Header`, `encode`, `decode` |
| `data` | `src/data.rs` | `DataHash`, listado ordenado de archivos de datos, `parse_ron` y `load_ron` |
| `version` | `src/version.rs` | `Version` y `GAME_VERSION` |
| `profile` | `src/profile.rs` | `start`, `frame_mark`, `tick_mark`, `span!`, `plot!` sobre Tracy, detrás de la feature `tracy` |
| `math` | `src/math.rs` | reexporta `glam` |

`src/lib.rs` declara los módulos, lleva `#![forbid(unsafe_code)]` y `#![deny(missing_docs)]`, y no define nada más. `src/hash.rs` lleva `const _: () = assert!(cfg!(target_endian = "little"));`, porque la vía en bloque del hash de estado solo vale en little-endian y la plataforma de la fase A es `x86_64-pc-windows-msvc` (ADR-0001). Toda la documentación está en español; los identificadores, en inglés.

## Qué no hace nunca

- No define tablas, entidades, fases, eventos, órdenes, snapshots ni el cuerpo de un guardado o un replay. Eso es de `engine_sim` y `game_rules`.
- No mide tiempo real: ni `Instant`, ni `SystemTime`, ni acumuladores de frame. Solo expone la duración entera de un tick para quien sí toca el reloj (`game_app`).
- No crea hilos, no usa `rayon` y no tiene nada `static` mutable.
- No expone en sus firmas tipos de `rustc-hash`, `xxhash-rust`, `bytemuck`, `postcard` ni `ron`, salvo los alias de `collections` y el reexport de `bytemuck` en `hash`. Cambiar de algoritmo o de formato es tocar solo este crate (ADR-0009, Q-008).
- No usa `std::collections::HashMap` ni `HashSet` con `RandomState`, ni en su API ni dentro; lo impide `clippy.toml` (CA-22).
- No lee ni escribe archivos salvo en `data` (carga de RON y hash del árbol de datos) y nunca en un camino caliente. El sobre de guardado trabaja sobre bytes; abrir y escribir el archivo es del hilo de entrada y salida de quien lo use.
- No vigila archivos ni recarga nada: el hot-reload de desarrollo (regla 18) es de la app, detrás de su feature.
- No asigna en `time`, `handle`, `hash`, `rng`, `profile` sin la feature, ni en las consultas de `collections`.
- No contiene número ni regla de juego alguna. Ni siquiera los ids de `Purpose`: los definen quienes extraen aleatoriedad.

## API pública

Es el contrato. `autor-tests` escribe contra estas firmas y los implementadores no las cambian salvo en una tarea de tipo `contrato`. Todo lo que no esté aquí es privado, o lleva `#[doc(hidden)]` y no es contrato. Las derivaciones listadas forman parte del contrato; añadir una derivación es un cambio de contrato, porque cambia qué pueden hacer los demás crates con el tipo.

Los pánicos que se citan son comprobaciones explícitas (`checked_*` y `assert!`), nunca el desbordamiento de `u64`, para que el comportamiento sea el mismo en dev y en release (el perfil de release no activa `overflow-checks`). Cada uno lleva un mensaje fijo, que los tests usan en `#[should_panic(expected = ...)]`.

### `engine_core::time`

```rust
use core::ops::{Add, AddAssign, Mul, Sub};
use core::time::Duration;
use serde::{Deserialize, Serialize};

/// Ticks de simulación por segundo. Es el único lugar del código donde aparece el 20 (ADR-0006).
pub const TICK_RATE_HZ: u32 = 20;
/// Ticks que caben en un segundo de juego, para expresar duraciones largas en tests y datos.
pub const TICKS_PER_SECOND: Ticks = Ticks(TICK_RATE_HZ as u64);
/// Duración de un tick en tiempo de pared, exacta: 50 ms. Solo la usan los crates que tocan el reloj real.
pub const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE_HZ as u64);

/// Instante de la simulación: número de ticks completados desde el inicio de la partida.
/// Garantiza orden total y codificación `postcard` como `u64` varint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Tick(pub u64);

/// Duración en ticks: la diferencia entre dos `Tick` (regla 17). Nunca negativa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Ticks(pub u64);

impl Tick {
    /// Estado inicial, antes del primer tick.
    pub const ZERO: Tick = Tick(0);
    /// El tick siguiente. Pánico `desbordamiento de ticks` si `self.0 == u64::MAX`.
    #[must_use]
    pub const fn next(self) -> Tick;
    /// Ticks transcurridos desde `earlier`. Exige `earlier <= self`; si no, pánico `retroceso de ticks`.
    #[must_use]
    pub const fn since(self, earlier: Tick) -> Ticks;
    /// Como `since`, pero devuelve `None` en vez de pánico cuando `earlier > self`.
    #[must_use]
    pub const fn checked_since(self, earlier: Tick) -> Option<Ticks>;
    /// Cierto si `self` es múltiplo de `period` (el tick 0 lo es de cualquier periodo). Exige `period > 0`; si no, pánico `periodo cero`.
    #[must_use]
    pub const fn is_multiple_of(self, period: Ticks) -> bool;
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Tick { type Output = Tick; }
impl AddAssign<Ticks> for Tick {}
/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Tick { type Output = Tick; }
/// Igual que `since`.
impl Sub<Tick> for Tick { type Output = Ticks; }

impl Ticks {
    pub const ZERO: Ticks = Ticks(0);
    pub const ONE: Ticks = Ticks(1);
    /// `self` menos `rhs`, o `None` si `rhs > self`.
    #[must_use]
    pub const fn checked_sub(self, rhs: Ticks) -> Option<Ticks>;
    /// Duración en tiempo de pared: `self * TICK_DURATION`. Para la app y las herramientas; nunca para la simulación.
    /// No desborda: `u64::MAX` ticks son menos de 2^60 segundos y `Duration` llega a 2^64 segundos.
    #[must_use]
    pub const fn to_wall_duration(self) -> Duration;
}

/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Add<Ticks> for Ticks { type Output = Ticks; }
impl AddAssign<Ticks> for Ticks {}
/// Exige `rhs <= self`; si no, pánico `retroceso de ticks`.
impl Sub<Ticks> for Ticks { type Output = Ticks; }
/// Pánico `desbordamiento de ticks` si el resultado no cabe en `u64`.
impl Mul<u64> for Ticks { type Output = Ticks; }
impl Mul<Ticks> for u64 { type Output = Ticks; }
```

### `engine_core::handle`

```rust
use core::num::NonZeroU32;
use serde::{Deserialize, Serialize};

/// Identidad de cualquier entidad de cualquier tabla (ADR-0004): slot más generación.
/// Garantiza: ocupa 8 bytes, `Option<Handle>` también ocupa 8 bytes, el orden es por (slot, generación)
/// y coincide con el orden numérico de `to_raw()`. La generación nunca es 0, así que `to_raw()` nunca es 0.
/// La tabla que emite el handle es la única que puede validarlo: este tipo no sabe de qué tabla viene.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(C)]
pub struct Handle {
    slot: u32,
    generation: NonZeroU32,
}

impl Handle {
    /// Generación con la que nace un slot: 1.
    pub const FIRST_GENERATION: NonZeroU32 = NonZeroU32::MIN;
    #[must_use]
    pub const fn new(slot: u32, generation: NonZeroU32) -> Handle;
    #[must_use]
    pub const fn slot(self) -> u32;
    /// `slot` como `usize`. Qué indexa lo fija la spec de `engine_sim`.
    #[must_use]
    pub const fn index(self) -> usize;
    #[must_use]
    pub const fn generation(self) -> NonZeroU32;
    /// `(slot << 32) | generation`. Es la forma que entra en el hash de estado y en la aleatoriedad sin estado.
    #[must_use]
    pub const fn to_raw(self) -> u64;
    /// Inversa de `to_raw`. `None` si los 32 bits bajos son 0.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Option<Handle>;
    /// La generación que sigue a `generation` cuando un slot se libera: suma 1 y de `u32::MAX` vuelve a 1, nunca a 0.
    #[must_use]
    pub const fn next_generation(generation: NonZeroU32) -> NonZeroU32;
}

/// `handle.to_raw()`, o 0 si no hay handle. Es lo que `rng::random_u64` hashea como sujeto, reciba `Some` o `None`.
#[must_use]
pub const fn raw_or_zero(handle: Option<Handle>) -> u64;
```

La codificación `serde` de `Handle` es la de sus dos campos en orden: `slot` y `generation`, los dos como `u32`. Deserializar una generación 0 es un error del deserializador, que `save::decode` convierte en `SaveError::Corrupt`.

### `engine_core::collections`

```rust
/// Mapa con hasher fijo (ADR-0009). Se construye con `FxHashMap::default()` o con `fx_map_with_capacity`.
/// Garantiza el mismo resultado para la misma secuencia de operaciones. No garantiza ningún orden de iteración:
/// un resultado que dependa de iterar un mapa viola la regla 13.
pub type FxHashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type FxHashSet<T> = rustc_hash::FxHashSet<T>;
pub use rustc_hash::{FxBuildHasher, FxHasher};

/// Reserva `capacity` entradas. Asigna: no es para el camino caliente.
#[must_use]
pub fn fx_map_with_capacity<K, V>(capacity: usize) -> FxHashMap<K, V>;
#[must_use]
pub fn fx_set_with_capacity<T>(capacity: usize) -> FxHashSet<T>;
```

### `engine_core::hash`

```rust
use core::fmt;
use serde::{Deserialize, Serialize};
use crate::data::DataHash;
use crate::handle::Handle;
use crate::rng::Seed;
use crate::time::{Tick, Ticks};
use crate::version::Version;

/// Reexport de `bytemuck`, para que `engine_sim` y `game_rules` pasen columnas a bytes con `cast_slice` sin
/// declararlo (Q-008, ADR-0009). Solo la API segura: aquí no hay `unsafe` que la acompañe.
pub use bytemuck;

/// Hash de 64 bits del estado, o de una parte de él. `Display` lo escribe en hexadecimal de 16 dígitos en minúsculas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StateHash(pub u64);
impl fmt::Display for StateHash {}

/// xxh3 de 64 bits en streaming con semilla 0 (ADR-0009). Vive en la pila (576 bytes) y no asigna.
/// El hash de una secuencia de bytes no depende de cómo se trocee entre llamadas a `write*`.
/// Un clon a mitad de secuencia continúa exactamente igual que el original (fijado en P01-T05).
#[derive(Clone, Default)]
pub struct StateHasher { /* privado */ }

impl StateHasher {
    #[must_use]
    pub fn new() -> StateHasher;
    pub fn write(&mut self, bytes: &[u8]);
    /// Los enteros entran en little-endian con su anchura exacta.
    pub fn write_u8(&mut self, v: u8);
    pub fn write_u16(&mut self, v: u16);
    pub fn write_u32(&mut self, v: u32);
    pub fn write_u64(&mut self, v: u64);
    pub fn write_i8(&mut self, v: i8);
    pub fn write_i16(&mut self, v: i16);
    pub fn write_i32(&mut self, v: i32);
    pub fn write_i64(&mut self, v: i64);
    /// Entra `v.to_bits()` en little-endian, sin normalizar: `0.0` y `-0.0` dan hashes distintos y cada NaN conserva su carga.
    pub fn write_f32(&mut self, v: f32);
    pub fn write_f64(&mut self, v: f64);
    /// Un byte: 0 o 1.
    pub fn write_bool(&mut self, v: bool);
    /// Los `to_raw()` de `handles` en orden, como `u64` little-endian y sin longitud delante. Mismos bytes que
    /// `write_u64(h.to_raw())` por elemento, pero troceados en bloques desde un buffer de pila de tamaño fijo (64 handles,
    /// 512 bytes, fijado en P01-T05): es la vía en bloque de las columnas de handles, que no pueden ir por `cast_slice`
    /// porque su memoria es (slot, generación). Con un slice vacío no escribe nada.
    pub fn write_handles(&mut self, handles: &[Handle]);
    pub fn write_value<T: HashState + ?Sized>(&mut self, value: &T);
    /// Hash de todo lo escrito hasta ahora. No consume ni reinicia: sirve para el hash por sistema del buscador de desincronizaciones.
    #[must_use]
    pub fn finish(&self) -> StateHash;
    /// Vuelve al estado de `new()`.
    pub fn reset(&mut self);
}

/// Cómo entra un valor en el hash de estado. Cada implementación define una secuencia de bytes fija.
pub trait HashState {
    fn hash_state(&self, hasher: &mut StateHasher);

    /// Cómo entra un slice de este tipo, sin la longitud delante. Por defecto, `hash_state` de cada elemento en orden;
    /// las implementaciones con vía en bloque lo sobrescriben con una sola escritura que produce exactamente los mismos
    /// bytes. `[T]`, `Vec<T>` y `[T; N]` lo llaman. Es el gancho que sustituye a la especialización, que Rust estable no tiene.
    fn hash_state_slice(items: &[Self], hasher: &mut StateHasher)
    where
        Self: Sized,
    {
        for item in items {
            item.hash_state(hasher);
        }
    }
}

/// `StateHasher::new()`, `write_value(value)`, `finish()`.
#[must_use]
pub fn hash_of<T: HashState + ?Sized>(value: &T) -> StateHash;
```

Implementaciones de `HashState` que da este crate, con la secuencia de bytes que aporta cada una y la vía en bloque de `hash_state_slice` cuando la hay:

| Tipo | Bytes de `hash_state` | `hash_state_slice` |
| --- | --- | --- |
| `u8` `u16` `u32` `u64` `i8` `i16` `i32` `i64` | el entero en little-endian con su anchura | `write(bytemuck::cast_slice(items))` |
| `bool` | un byte, 0 o 1 | `write(bytemuck::cast_slice(items))` |
| `f32` `f64` | `to_bits()` en little-endian | `write(bytemuck::cast_slice(items))` |
| `Tick` `Ticks` `StateHash` `DataHash` `Seed` | su `u64` | por defecto |
| `Handle` | `to_raw()` como `u64` | `write_handles(items)` |
| `Version` | `major`, `minor`, `patch` como `u16` | por defecto |
| `Option<T>` | un byte 0 si es `None`; un byte 1 y los bytes de `T` si es `Some` | por defecto |
| `[T]` y `Vec<T>` | la longitud como `u64` y después `T::hash_state_slice` | no aplica |
| `[T; N]` | `T::hash_state_slice`, sin longitud | no aplica |
| `(A, B)` y `(A, B, C)` | los componentes en orden | por defecto |
| `Vec2` `Vec3` `Vec4` `IVec2` `IVec3` `UVec2` `UVec3` de `glam` | los componentes en orden `x, y, z, w` | `write(bytemuck::cast_slice(items))` |

No se implementa para `usize` ni `isize`, porque su anchura depende de la plataforma, ni para `Vec3A`, porque lleva relleno. La vía en bloque da el mismo hash que la secuencia elemento a elemento porque en little-endian los bytes en memoria ya son los de la tabla, y es la forma de cumplir la estimación de coste.

### `engine_core::rng`

```rust
use serde::{Deserialize, Serialize};
use crate::handle::Handle;
use crate::time::Tick;

/// Semilla de la partida. Es canónica: va en el guardado y en el replay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Seed(pub u64);

/// Para qué se extrae aleatoriedad: un id de 32 bits que fija quien la usa, y un índice para la n-ésima extracción
/// del mismo propósito en el mismo tick y sujeto (Q-009). La aleatoriedad no tiene estado canónico. Sin `Default`:
/// el propósito se declara siempre con `new`, para que el id 0 no se obtenga por omisión (Q-019).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Purpose(pub u64);

impl Purpose {
    /// `(id << 32)`, índice 0.
    #[must_use]
    pub const fn new(id: u32) -> Purpose;
    /// Mismo id, índice `index` en los 32 bits bajos. `index` es la posición de la extracción dentro del bucle del
    /// tick (la n-ésima del mismo propósito y sujeto), nunca un contador guardado en la entidad, que sería estado no
    /// canónico (regla 6, Q-019). Se cuenta en el orden canónico de la fase, nunca por la posición densa de una
    /// entidad en su tabla, que cambia tras un swap-remove, ni por el orden de llegada de los hilos; una extracción
    /// por entidad lleva su handle como sujeto, no `None` con el índice de la entidad (Q-022).
    #[must_use]
    pub const fn nth(self, index: u32) -> Purpose;
    #[must_use]
    pub const fn id(self) -> u32;
    #[must_use]
    pub const fn index(self) -> u32;
}

/// `xxh3_64` con semilla 0 sobre 32 bytes: `seed.0`, `tick.0`, `handle::raw_or_zero(subject)` y `purpose.0`, cada uno
/// como `u64` little-endian y en ese orden (ADR-0009). Pura: mismas entradas, mismo valor, en cualquier hilo. No asigna.
/// `subject` es el handle de la entidad, o `None` para una extracción global. El tipo obliga a que el sujeto sea un
/// handle emitido por su tabla, no un índice denso de slot, que cambia tras un swap-remove y haría que la misma
/// entidad extrajera valores distintos según la historia de borrados (Q-018, Q-022).
#[must_use]
pub fn random_u64(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> u64;

/// Los 64 bits de una extracción, con las conversiones habituales. Todas son deterministas y no asignan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Draw(u64);

impl Draw {
    #[must_use]
    pub const fn from_bits(bits: u64) -> Draw;
    #[must_use]
    pub const fn bits(self) -> u64;
    /// En `[0, 1)`: los 24 bits altos divididos por 2^24. Exacto en `f32`.
    #[must_use]
    pub fn unit_f32(self) -> f32;
    /// En `[0, 1)`: los 53 bits altos divididos por 2^53. Exacto en `f64`.
    #[must_use]
    pub fn unit_f64(self) -> f64;
    /// En `[0, n)`: `((bits as u128 * n as u128) >> 64) as u32`. Exige `n > 0`; si no, pánico `rango vacío`.
    #[must_use]
    pub const fn below(self, n: u32) -> u32;
    /// `below(denominator) < numerator`. Exige `denominator > 0`; si no, pánico `rango vacío`.
    #[must_use]
    pub const fn chance(self, numerator: u32, denominator: u32) -> bool;
    /// `(bits >> 32, bits & 0xFFFF_FFFF)`, para quien necesite dos valores de 32 bits de una extracción.
    #[must_use]
    pub const fn split(self) -> (u32, u32);
}

/// `Draw::from_bits(random_u64(seed, tick, subject, purpose))`.
#[must_use]
pub fn draw(seed: Seed, tick: Tick, subject: Option<Handle>, purpose: Purpose) -> Draw;
```

### `engine_core::save`

```rust
use serde::{Serialize, de::DeserializeOwned, Deserialize};
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
    Save,
    Replay,
}

/// Cabecera del ADR-0007: tipo, versión del formato del cuerpo, versión del juego y hash de los datos con los que se jugó.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub kind: Kind,
    /// La fija quien define el cuerpo (`engine_sim` o `game_rules`) y la sube con cada cambio de struct canónica.
    pub format_version: u32,
    /// Informativa: no decide por sí sola si el archivo se puede cargar.
    pub game_version: Version,
    pub data_hash: DataHash,
}

impl Header {
    /// Comprueba, en este orden, `kind`, `format_version` y `data_hash`, y devuelve el primer error.
    pub fn check(&self, expected: &Expected) -> Result<(), SaveError>;
}

/// Lo que el llamante exige a una cabecera para cargarla sin migración.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Expected {
    pub kind: Kind,
    pub format_version: u32,
    pub data_hash: DataHash,
}

/// Mensajes en español, fijados en P01-T04. `PartialEq` para que los tests comparen errores enteros.
/// `UnexpectedKind` escribe los nombres de variante de `Kind` (`Save`, `Replay`) con `Debug` (Q-020, aceptado);
/// `DataHashMismatch` usa el `Display` hexadecimal de `DataHash`.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    /// Menos de 8 bytes, o los 8 primeros no son `MAGIC`.
    #[error("el archivo no es un guardado ni un replay: faltan los 8 bytes iniciales `PLUSULTR`")]
    BadMagic,
    /// El byte de la posición 8 no es `ENVELOPE_VERSION`.
    #[error("versión del sobre no admitida: se encontró la {found} y esta build lee la {}", ENVELOPE_VERSION)]
    UnsupportedEnvelope { found: u8 },
    #[error("tipo de archivo inesperado: se esperaba {expected:?} y se encontró {found:?}")]
    UnexpectedKind { expected: Kind, found: Kind },
    #[error("versión del formato del cuerpo no admitida: se esperaba la {expected} y se encontró la {found}")]
    UnsupportedFormat { expected: u32, found: u32 },
    /// Regla 18: los datos con los que se jugó no son los cargados.
    #[error("los datos con los que se jugó ({found}) no son los cargados ({expected})")]
    DataHashMismatch { expected: DataHash, found: DataHash },
    /// Sobran `count` bytes después del cuerpo.
    #[error("después del cuerpo sobran {count} bytes")]
    TrailingBytes { count: usize },
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
pub fn encode<T: Serialize + ?Sized>(header: &Header, body: &T, out: &mut Vec<u8>) -> Result<(), SaveError>;
/// `encode` sobre un `Vec` nuevo.
pub fn encode_to_vec<T: Serialize + ?Sized>(header: &Header, body: &T) -> Result<Vec<u8>, SaveError>;
/// Valida `MAGIC` y `ENVELOPE_VERSION`, decodifica la cabecera y devuelve el resto, que es el cuerpo sin tocar.
/// Es el primer paso de una migración: se mira `format_version` y se elige con qué struct leer el cuerpo.
pub fn decode_header(bytes: &[u8]) -> Result<(Header, &[u8]), SaveError>;
/// Decodifica un cuerpo entero. Exige consumirlo todo; si sobran bytes, `TrailingBytes`.
pub fn decode_body<T: DeserializeOwned>(body: &[u8]) -> Result<T, SaveError>;
/// `decode_header`, `Header::check(expected)` y `decode_body`, en ese orden. El camino normal de carga.
pub fn decode<T: DeserializeOwned>(bytes: &[u8], expected: &Expected) -> Result<(Header, T), SaveError>;
```

### `engine_core::data`

```rust
use core::fmt;
use std::path::{Path, PathBuf};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Hash de los datos con los que se juega (regla 18): xxh3 de 64 bits (Q-006). `Display` en hexadecimal de 16 dígitos.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct DataHash(pub u64);
impl fmt::Display for DataHash {}

impl DataHash {
    /// `xxh3_64` con semilla 0 de `bytes`.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> DataHash;
}

/// Extensión de los archivos de datos. Solo estos entran en el listado y en el hash (Q-010).
pub const DATA_EXTENSION: &str = "ron";

/// Un archivo de datos: su ruta relativa a la raíz, con `/` como separador, y su ruta completa.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataFile {
    pub relative: String,
    pub path: PathBuf,
}

/// Todos los archivos regulares con extensión `ron` (sin distinguir mayúsculas) bajo `root`, recursivamente,
/// ordenados por los bytes UTF-8 de `relative`. No sigue enlaces simbólicos. El orden no depende del sistema de archivos.
/// `root` inexistente da `DataError::Io`; un nombre que no es UTF-8, `DataError::InvalidPath`.
pub fn list_data_files(root: &Path) -> Result<Vec<DataFile>, DataError>;

/// Hash del árbol de datos. Para cada archivo de `list_data_files`, en su orden, entran: los bytes UTF-8 de
/// `relative`, un byte 0, la longitud del contenido como `u64` little-endian y el contenido tal cual.
/// Un árbol sin archivos da el hash de la secuencia vacía.
pub fn hash_data_tree(root: &Path) -> Result<DataHash, DataError>;

/// Deserializa RON con las opciones por defecto de `ron`. Quita una marca de orden de bytes UTF-8 inicial si la hay.
/// Las extensiones de RON se activan dentro del propio archivo (`#![enable(implicit_some)]`), no aquí.
pub fn parse_ron<T: DeserializeOwned>(text: &str) -> Result<T, DataError>;
/// Lee el archivo y llama a `parse_ron`. Los errores llevan la ruta. Un contenido que no es UTF-8 da `Io` con
/// `source.kind() == InvalidData`.
pub fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T, DataError>;

/// Mensajes en español, fijados en P01-T03. El de `Io` incluye el texto del sistema, que Windows localiza:
/// los tests comparan variantes y `source.kind()`, nunca `to_string()`.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("error de entrada/salida en `{}`: {source}", .path.display())]
    Io { path: PathBuf, #[source] source: std::io::Error },
    /// Un nombre de archivo o de carpeta que no es UTF-8.
    #[error("ruta de datos que no es UTF-8: `{}`", .path.display())]
    InvalidPath { path: PathBuf },
    /// `line` y `column` empiezan en 1. `path` es `None` cuando viene de `parse_ron`; con ruta, el mensaje
    /// intercala `` en `ruta` `` tras "RON".
    #[error("error de sintaxis RON{} en la línea {line}, columna {column}: {message}", /* ` en `ruta`` o nada */)]
    Parse { path: Option<PathBuf>, line: u32, column: u32, message: String },
}
```

### `engine_core::version`

```rust
use core::fmt;
use serde::{Deserialize, Serialize};

/// Versión semántica reducida. El orden es el lexicográfico de `(major, minor, patch)`. `Display` escribe `major.minor.patch` en decimal y sin relleno, por ejemplo `0.1.0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Version;
}
impl fmt::Display for Version {}

/// La versión de `workspace.package.version`, leída de `CARGO_PKG_VERSION_MAJOR`, `_MINOR` y `_PATCH` al compilar.
/// La versión del workspace no lleva sufijo de prerrelease (`-alpha`, `-rc.1`): lo comprueba al compilar
/// `const _: () = assert!(env!("CARGO_PKG_VERSION_PRE").is_empty());` en este módulo (Q-015).
pub const GAME_VERSION: Version;
```

### `engine_core::profile`

```rust
/// Con la feature `tracy`, llama a `Client::start()` de `tracy-client`; es idempotente y se puede llamar desde cualquier
/// hilo. Sin la feature no hace nada. El cliente de Tracy arranca con el proceso (sin `manual-lifetime`), así que las zonas
/// se registran desde la primera instrucción de `main`, se haya llamado o no a `start()`.
pub fn start();
/// Cierto con la feature activa, desde el inicio del proceso; falso sin ella.
#[must_use]
pub fn is_active() -> bool;
/// Fin de un frame del hilo principal.
pub fn frame_mark();
/// Fin de un tick de simulación: frame secundario llamado `tick`.
pub fn tick_mark();
/// Nombre del hilo actual en el perfilador. Con la feature asigna una vez por llamada (`CString` dentro de
/// `tracy-client`); se llama una vez por hilo, nunca en el camino caliente.
pub fn set_thread_name(name: &'static str);

/// Zona de perfilado. Se cierra al soltarla. No es `Send` en ninguna configuración: se abre y se cierra en el mismo hilo.
/// Sin la feature es un tipo de tamaño cero.
#[must_use = "una zona que no se guarda en una variable se cierra en la misma línea"]
pub struct Span { /* privado */ }

/// `let _zona = engine_core::span!("fase_mover");`. El nombre es un literal: sin la feature no asigna nunca; con
/// ella, `span_location!` de `tracy-client` crea un `static` por punto de uso que asigna una sola vez, en la primera
/// llamada (decisión de P01-T04).
#[macro_export]
macro_rules! span { ($name:literal) => { /* ... */ }; }
/// `engine_core::plot!("unidades_vivas", valor_f64)`. El nombre es un literal: sin la feature no asigna nunca; con
/// ella, un `static` por punto de uso guarda el `PlotName` creado con `PlotName::new_leak` la primera vez, una sola
/// asignación por punto de uso y ningún `unsafe` expandido en el crate que llama (Q-021; `plot_name!` de
/// `tracy-client` no se usa).
#[macro_export]
macro_rules! plot { ($name:literal, $value:expr) => { /* ... */ }; }
```

Las macros se expanden en el crate que las usa, así que un `cfg(feature = "tracy")` dentro de `macro_rules!` no serviría. Cada macro tiene dos definiciones en `engine_core`, una bajo `#[cfg(feature = "tracy")]` y otra bajo `#[cfg(not(feature = "tracy"))]`, y las dos se apoyan en ítems `#[doc(hidden)] pub` de `profile` (constructores de `Span`, el reexport de `tracy_client` con la feature) que no forman parte del contrato y pueden cambiar.

### `engine_core::math`

```rust
/// Tipos matemáticos del motor. `engine_sim` y `game_rules` los toman de aquí.
pub use glam::*;
```

El determinismo que se exige es en la misma máquina (regla 13), así que la ruta SSE2 de `glam` vale. Lo que cambiaría los resultados en flotante entre dos builds es el FMA: `glam` lo usa cuando el compilador lleva `target_feature = "fma"`, que activan `-C target-cpu` o `-C target-feature`, y entonces los hashes dorados de una máquina no valen en otra. Por eso `-C target-cpu` y `-C target-feature` están prohibidos en `.cargo/config.toml` de todo el workspace hasta que el hito 2 mida, y lo vigila `check-arch` (Q-011, respondida el 26 de octubre de 2026; CA-23). La feature `core-simd` de `glam` exige nightly y el toolchain es estable, y `fast-math` ya no existe en `glam` 0.34: ninguna de las dos necesita vigilancia.

### Features del crate

| Feature | Por defecto | Efecto |
| --- | --- | --- |
| `tracy` | no | activa `tracy-client` con `enable`; `profile` emite zonas, plots y marcas de frame. Se usa en builds de medición del autor. La puerta adicional de este crate la compila y la prueba para que no se pudra |

## Contratos compartidos que toca

| Contrato | Esta spec | Quién lo consume |
| --- | --- | --- |
| Handle | lo crea: `handle::Handle` | `engine_sim` (tablas, grid, eventos), `game_rules`, `engine_render` (casar unidades entre snapshots) |
| `Tick` | lo crea: `time::Tick`, `time::Ticks`, 20 Hz | todos |
| Hash de estado | lo crea: `hash::StateHasher`, `HashState` | `engine_sim` (hash por tick y por sistema), herramientas de replay |
| Aleatoriedad sin estado | la crea: `rng` | `engine_sim`, `game_rules` |
| Sobre de guardado y replay | crea la cabecera y la codificación; **no** el cuerpo | `engine_sim` define el cuerpo del replay (semilla, órdenes selladas por tick) y el del guardado; `game_rules` sus structs canónicas |
| Hash de datos | lo crea: `data::DataHash`, `hash_data_tree` | quien carga los datos (`game_rules`, herramientas) y quien guarda |
| `[workspace.dependencies]` | añade las versiones de la sección siguiente | el `Cargo.toml` raíz, en la tarea de contrato |
| `clippy.toml` con `disallowed-types` (ADR-0009) | lo crea la tarea de contrato de este crate: un archivo igual en `engine/core/`, `engine/sim/` y `game/rules/`, porque clippy busca la configuración desde la carpeta del manifiesto de cada crate hacia arriba y un archivo en la raíz afectaría a `xtask`, que usa `HashMap` de forma legítima | `engine_sim` y `game_rules` lo heredan sin hacer nada |
| Reexports de `bytemuck` y `glam` | los crea | `engine_sim` y `game_rules` no declaran `rustc-hash`, `xxhash-rust` ni `bytemuck` (Q-008); si necesitan `glam` o las derivaciones de `bytemuck` directamente, lo dice su spec |
| Extensión de `cargo xtask check-arch` (Q-008 y Q-011; ADR-0009 "comprobación por añadir") | la pide CA-23; va en una tarea propia del plan de esta spec que toca `xtask/`, y el encargo lo dice expresamente | la puerta de todos |
| Puerta adicional de este crate | la declara la sección "Criterios de aceptación" | `/planificar` la copia en la sección "Verificación" del plan y en cada tarea de este crate; `/verificar` la ejecuta |
| Fases y eventos, API de órdenes, layout del snapshot | **no los toca** | specs de `engine_sim`, `game_rules` y `engine_render` |

## Dependencias permitidas

Crates del workspace: **ninguno**.

Externos, con la línea exacta que va en `[workspace.dependencies]` del `Cargo.toml` raíz. En `engine/core/Cargo.toml` cada uno se declara con `nombre.workspace = true`; `tracy-client` además con `optional = true`. Versiones, features y licencias comprobadas en crates.io el 26 de octubre de 2026 con `cargo info` (`glam` 0.34.1, `serde` 1.0.229, `postcard` 1.1.3, `ron` 0.12.2, `rustc-hash` 2.1.3, `xxhash-rust` 0.8.19, `bytemuck` 1.25.2, `thiserror` 2.0.21, `tracy-client` 0.19.0, `proptest` 1.12.0, `criterion` 0.8.2, `allocation-counter` 0.8.1); la tarea de contrato las vuelve a confirmar con `cargo check --workspace --all-targets` y `cargo deny check`, y termina `bloqueado` si alguna no resuelve.

| Crate | Línea en `[workspace.dependencies]` | Para qué | Licencia |
| --- | --- | --- | --- |
| `glam` | `glam = { version = "0.34", features = ["serde", "bytemuck"] }` | `math`; `serde` para las structs canónicas con posiciones; `bytemuck` para hashear columnas de vectores de un golpe | MIT o Apache-2.0 |
| `serde` | `serde = { version = "1.0", features = ["derive"] }` | derivaciones de los tipos canónicos | MIT o Apache-2.0 |
| `postcard` | `postcard = { version = "1.1", default-features = false, features = ["use-std"] }` | el sobre (ADR-0007); sin `heapless` | MIT o Apache-2.0 |
| `ron` | `ron = "0.12"` | `parse_ron` (ADR-0007) | MIT o Apache-2.0 |
| `rustc-hash` | `rustc-hash = "2.1"` | `collections` (ADR-0009) | MIT o Apache-2.0 |
| `xxhash-rust` | `xxhash-rust = { version = "0.8", features = ["xxh3"] }` | `hash`, `rng`, `data` (ADR-0009) | BSL-1.0, permitida en `deny.toml` |
| `bytemuck` | `bytemuck = "1.25"` | `cast_slice` de columnas a bytes en `hash`, sin `unsafe` propio (Q-007) | Zlib, Apache-2.0 o MIT |
| `thiserror` | `thiserror = "2.0"` | `SaveError` y `DataError`; su `syn` ya lo trae `serde_derive` | MIT o Apache-2.0 |
| `tracy-client` | `tracy-client = { version = "0.19", default-features = false, features = ["enable"] }` | `profile`, solo con la feature `tracy` del crate; sin `sampling` ni `system-tracing`, que en Windows piden privilegios; sin `manual-lifetime`, así que el cliente arranca con el proceso | MIT o Apache-2.0; su `sys` incluye Tracy, BSD-3-Clause, permitida |

De desarrollo (solo `[dev-dependencies]` de `engine/core/Cargo.toml`, con `nombre.workspace = true`). `cargo deny` también las revisa:

| Crate | Línea en `[workspace.dependencies]` | Para qué | Licencia |
| --- | --- | --- | --- |
| `proptest` | `proptest = "1.12"` | tests de propiedades de `handle`, `time`, `rng` y el sobre | MIT o Apache-2.0 |
| `criterion` | `criterion = { version = "0.8", default-features = false, features = ["cargo_bench_support"] }` | benchmarks de `hash`, `rng` y el sobre | Apache-2.0 o MIT |
| `allocation-counter` | `allocation-counter = "0.8"` | CA-14, CA-17 y CA-18, regla 19. Verificado en su fuente (0.8.1): sin dependencias, sin `unsafe` en el código que lo usa, sustituye el asignador global de los binarios de test al enlazarse, y su contador es **por hilo** (`measure` cuenta solo las asignaciones del hilo que lo llama). Era el candidato de Q-003 y aquí queda fijado porque este crate lo necesita antes que la simulación sintética (decisión 5) | MIT o Apache-2.0 |

Los tests pueden declarar además `xxhash-rust.workspace = true` y `postcard.workspace = true` como dependencias de desarrollo para comparar con la implementación de referencia (CA-4, CA-8, CA-9); siguen dentro de `engine/core/Cargo.toml`, así que `engine_core` sigue siendo el único crate que declara `rustc-hash`, `xxhash-rust` y `bytemuck` (Q-008). Los tests no usan `tempfile` ni `rand`: las carpetas temporales van bajo `env!("CARGO_TARGET_TMPDIR")`, en una subcarpeta con el nombre del test que se borra y se vuelve a crear al empezar, y los bytes pseudoaleatorios salen de un `xorshift64` de semilla fija escrito en el propio test. Cualquier otro crate es una violación de las reglas 1 y 2 y una pregunta para el autor.

## Datos y formatos

**Sobre de guardado y replay** (`save`), de principio a fin del archivo, con posiciones desde 0:

| Posición | Contenido |
| --- | --- |
| 0 a 7 | `MAGIC` = `PLUSULTR` = `50 4C 55 53 55 4C 54 52` |
| 8 | `ENVELOPE_VERSION` = `01` |
| 9 en adelante | `Header` en `postcard`: `kind` (índice de variante, varint), `format_version` (varint `u32`), `game_version` (tres varint `u16`), `data_hash` (varint `u64`) |
| después | el cuerpo en `postcard`, hasta el final del archivo |

La cabecera completa, de la posición 0 al inicio del cuerpo, ocupa como mucho 34 bytes (8 + 1 + 1 + 5 + 9 + 10). Ejemplo fijado: `Header { kind: Replay, format_version: 300, game_version: 0.1.0, data_hash: DataHash(1) }` con cuerpo `()` es exactamente `50 4C 55 53 55 4C 54 52 01 01 AC 02 00 01 00 01`, 16 bytes. El formato de `postcard` es el documentado en su especificación: enteros varint, `f32` y `f64` en little-endian de anchura fija, `bool` en un byte, variantes de enum por índice. Quien define el cuerpo versiona sus structs con `format_version` y migra con `decode_header` más `decode_body` sobre la struct antigua (ADR-0007). Este crate no migra nada.

**Árbol de datos** (`data`): archivos `*.ron` bajo una raíz que fija quien lo usa (`data/` en el repositorio, Q-004). Orden y bytes que entran en `DataHash` según `hash_data_tree`: es el "bytes crudos de cada archivo en orden lexicográfico de rutas relativas" del ADR-0009, más la ruta y la longitud delante de cada archivo para que un cambio de nombre o un desplazamiento de bytes entre dos archivos también cambien el hash. El contenido entra tal cual, con marca de orden de bytes si la tuviera: cualquier cambio de bytes invalida los replays, como pide `data/README.md`. Solo cuentan los `*.ron`; los demás archivos de `data/`, como su `README.md`, no entran (Q-010, respondida el 26 de octubre de 2026).

**RON** (`data::parse_ron`): opciones por defecto de `ron` 0.12; extensiones activadas dentro de cada archivo. Qué structs y qué campos tiene cada archivo lo dicen las specs de features y de `game_rules`.

## Presupuestos

Ninguna fila de la tabla de presupuestos de la constitución es de este crate. Lo que le toca:

| Métrica | Valor | De dónde sale | Cómo se mide |
| --- | --- | --- | --- |
| Hash de estado de 3 MB de columnas SoA | unos 0,3 ms en la máquina de referencia: **estimación sin umbral**; el hito 2 decide si entra en la tabla de presupuestos | ADR-0009 y revisión técnica | `benches/hash_estado.rs` |
| Cabecera del sobre | ≤ 34 bytes, para no restar nada a los 20 MB de archivo de la fila "Guardar partida" | esta spec | CA-11 |
| Asignaciones de las primitivas de `time`, `handle`, `hash`, `rng`, `collections` (consulta) y `profile` sin feature | 0 | regla 19 | CA-18 |
| Codificar y decodificar un cuerpo de 3 MB | sin umbral; se reporta | fila "Guardar partida": el parón lo decide la copia en el hilo de simulación, no esta codificación | `benches/sobre.rs` |
| `random_u64` | sin umbral; se reporta en extracciones por segundo y por hilo | regla 13 | `benches/rng.rs` |

## Criterios de aceptación

Los tests van en `engine/core/tests/aceptacion_core_tNN.rs`, uno por tarea, y usan solo la API pública; los de CA-23 van en `xtask/tests/`. Los vectores de bytes y los valores de hash citados se han calculado con `postcard` 1.1.3 y `xxhash-rust` 0.8.19; el de la entrada vacía coincide con el valor publicado de XXH3. Un test que no coincida con ellos señala un error de implementación, no del test.

**Puerta adicional de este crate.** Además de las cinco comprobaciones de `CLAUDE.md`, toda tarea de `engine_core` pasa `cargo test -p engine_core --release` (los pánicos de CA-1 y CA-9 no dependen del perfil), `cargo clippy -p engine_core --features tracy --all-targets -- -D warnings`, `cargo test -p engine_core --features tracy`, `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_core --no-deps` y `cargo tree -p engine_core -e normal` (CA-21). `/planificar` la copia en la sección "Verificación" del plan y en cada tarea; `/verificar` la ejecuta junto a la puerta común. Las pruebas negativas que exigen editar (CA-22 y CA-23) las hace el orquestador en el worktree de la tarea, nunca un revisor, que es de solo lectura, y las anota en el informe de `/verificar`.

- **CA-1.** `Tick::ZERO.next() == Tick(1)`; `Tick::ZERO.since(Tick::ZERO) == Ticks::ZERO`; `Tick(10).since(Tick(3)) == Ticks(7)`; `Tick(3) + Ticks(4) == Tick(7)`; `Tick(7) - Tick(3) == Ticks(4)`; `Tick(7) - Ticks(7) == Tick::ZERO`; `Ticks(2) + Ticks(3) == Ticks(5)`; `Ticks(9) - Ticks(4) == Ticks(5)`; `Ticks(3) * 4 == Ticks(12)` y `4 * Ticks(3) == Ticks(12)`; `+=` sobre `Tick` y `Ticks`; `Tick(3).checked_since(Tick(4)) == None`; `Ticks(3).checked_sub(Ticks(4)) == None`; `Tick(0).is_multiple_of(Ticks(5))` y `Tick(10).is_multiple_of(Ticks(5))` ciertos, `Tick(7).is_multiple_of(Ticks(5))` falso. Y siete tests con `#[should_panic(expected = ...)]`: `Tick(3).since(Tick(4))` y `Tick(3) - Ticks(4)` con `retroceso de ticks`; `Ticks(3) - Ticks(4)` con `retroceso de ticks`; `Tick(1).is_multiple_of(Ticks(0))` con `periodo cero`; `Tick(u64::MAX).next()`, `Tick(u64::MAX) + Ticks::ONE` y `Ticks(u64::MAX) * 2` con `desbordamiento de ticks`. Lo demuestran: tests `ca_01_tick_aritmetica_entera`, `ca_01_since_retroceso_panica`, `ca_01_resta_de_ticks_panica`, `ca_01_resta_de_duraciones_panica`, `ca_01_periodo_cero_panica`, `ca_01_next_desbordamiento_panica`, `ca_01_suma_desbordamiento_panica` y `ca_01_producto_desbordamiento_panica`, en dev y con `cargo test -p engine_core --release`.
- **CA-2.** `TICK_RATE_HZ == 20`; `TICK_DURATION == Duration::from_millis(50)`; `TICKS_PER_SECOND.to_wall_duration() == Duration::from_secs(1)`; `Ticks(u64::MAX).to_wall_duration()` no hace pánico y vale exactamente `Duration::new(u64::MAX / 20, 750_000_000)`; `(TICKS_PER_SECOND * 3_600 * 4) == Ticks(288_000)`; y el texto de `engine/core/src/time.rs` y `engine/core/src/handle.rs` no contiene `f32` ni `f64` (regla 17). Lo demuestra: test `ca_02_frecuencia_y_duracion_del_tick_sin_flotantes`, que lee los dos archivos con `include_str!("../src/time.rs")` e `include_str!("../src/handle.rs")`.
- **CA-3.** `size_of::<Handle>() == 8` y `size_of::<Option<Handle>>() == 8`; `new(5, 7)` devuelve `slot() == 5`, `index() == 5`, `generation() == 7`, `to_raw() == (5 << 32) | 7`; `from_raw(to_raw()) == Some(h)` para cualquier handle; `from_raw(5 << 32) == None`; `next_generation(1) == 2` y `next_generation(u32::MAX) == 1`; para dos handles cualesquiera `a < b` equivale a `a.to_raw() < b.to_raw()`; `raw_or_zero(None) == 0` y `raw_or_zero(Some(h)) == h.to_raw()`. Lo demuestran: tests `ca_03_handle_ocho_bytes_y_orden_igual_al_raw` y, con `proptest`, `ca_03_handle_raw_ida_y_vuelta_propiedad`.
- **CA-4.** `postcard::to_allocvec` de `Tick(300)` y de `Ticks(300)` da `[0xAC, 0x02]`; el de `Handle::new(5, 7)` da `[0x05, 0x07]`; el de `Version::new(0, 1, 0)` da `[0x00, 0x01, 0x00]`; y todos vuelven idénticos con `postcard::from_bytes`. Deserializar `[0x05, 0x00]` como `Handle` falla. Lo demuestra: test `ca_04_tick_handle_y_version_postcard_con_bytes_conocidos`.
- **CA-5.** `size_of::<FxBuildHasher>() == 0`; dos `FxHashMap` construidos con `default()` en dos hilos distintos, con las mismas 10.000 inserciones en el mismo orden, producen la misma secuencia al iterar; `fx_map_with_capacity::<u64, u64>(100).capacity() >= 100` y lo mismo para `fx_set_with_capacity`. Lo demuestra: test `ca_05_colecciones_fx_sin_estado_aleatorio`.
- **CA-6.** Sobre un bloque de 1 MB de bytes pseudoaleatorios: `write` de todo de una vez, `write` en trozos de 1, 7 y 4.096 bytes y `write_u8` byte a byte dan el mismo `finish()`; `finish()` tras más escrituras cambia; `reset()` devuelve el hash de `new()`. Y para cada fila de la tabla de `HashState`, con al menos un valor por tipo, `hash_of(&valor)` es igual a `write` de la secuencia de bytes escrita a mano en el test según la tabla. Entre otros: `7u8` → `07`; `0x1234u16` → `34 12`; `-2i16` → `FE FF`; `true` → `01`; `1.5f32` → `00 00 C0 3F`; `1.5f64` → `00 00 00 00 00 00 F8 3F`; `Tick(300)` → `2C 01 00 00 00 00 00 00`; `Ticks(5)`, `StateHash(5)`, `DataHash(5)` y `Seed(5)` → `05` y siete `00`; `Handle::new(5, 7)` → `07 00 00 00 05 00 00 00`; `Version::new(0, 1, 0)` → `00 00 01 00 00 00`; `Some(7u8)` → `01 07` y `None::<u8>` → `00`; `&[1u8, 2, 3][..]` y `vec![1u8, 2, 3]` → `03` y siete `00` y `01 02 03`; `&[][..]` como `&[u32]` → `00` por ocho; `[1u8, 2, 3]` → `01 02 03`; `(1u8, 2u16)` → `01 02 00`; `(1u8, 2u16, 3u32)` → `01 02 00 03 00 00 00`; `Vec2::new(1.0, 2.0)` → `00 00 80 3F 00 00 00 40`; `IVec2::new(-1, 2)` → `FF FF FF FF 02 00 00 00`; `UVec3::new(1, 2, 3)` → `01 00 00 00 02 00 00 00 03 00 00 00`; y los `Vec3`, `Vec4`, `IVec3` y `UVec2` análogos. Lo demuestran: tests `ca_06_hash_estado_independiente_del_troceo` y `ca_06_hash_estado_tabla_fila_a_fila`.
- **CA-7.** `hash_of(&0.0f32) != hash_of(&-0.0f32)`; dos NaN con distinta carga dan hashes distintos; `hash_of(&[1.5f32, -2.25][..])` es igual a `write_u64(2)`, `write_u32(1.5f32.to_bits())`, `write_u32((-2.25f32).to_bits())`. Vía en bloque, para cada fila de la tabla con `hash_state_slice` propio (`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `bool`, `f32`, `f64`, `Vec2`, `Vec3`, `Vec4`, `IVec2`, `IVec3`, `UVec2`, `UVec3`): para un `Vec<T>` `v` de 15.000 elementos generados con `proptest` (en los flotantes, con ceros negativos y NaN incluidos), `hash_of(&v[..])`, `write_u64(15_000)` más `write(bytemuck::cast_slice(&v))`, y `write_u64(15_000)` seguido de `hash_state` elemento a elemento dan el mismo `finish()` (Q-017); para un `Vec<Handle>` `h` de 15.000 elementos, `hash_of(&h[..])`, `write_u64(15_000)` más `write_handles(&h)`, y `write_u64(15_000)` seguido de `write_u64(x.to_raw())` por elemento dan el mismo `finish()`; `write_handles(&[])` deja el hash como estaba; y para un `Vec<Option<Handle>>` de 15.000 elementos, `hash_of` es igual a la secuencia de la regla de `Option` escrita a mano. Lo demuestra: test `ca_07_hash_estado_flotantes_bit_a_bit_y_via_en_bloque`.
- **CA-8.** `StateHasher::new().finish() == StateHash(0x2D06_8005_38D3_94C2)` y `DataHash::of_bytes(&[]) == DataHash(0x2D06_8005_38D3_94C2)`, el valor publicado de XXH3 de 64 bits para la entrada vacía; `DataHash::of_bytes(b"a") == DataHash(0xE6C6_32B6_1E96_4E1F)`; para 100 bloques pseudoaleatorios de longitud entre 0 y 10.000 bytes, `StateHasher` con `write` y `DataHash::of_bytes` coinciden con `xxhash_rust::xxh3::xxh3_64` sobre los mismos bytes; `Display` de ambos es el hexadecimal de 16 dígitos en minúsculas del `u64` (`2d06800538d394c2` para el vacío); y `postcard::to_allocvec` de `Seed(1)` da `[0x01]`, el de `DataHash(u64::MAX)` y el de `StateHash(u64::MAX)` dan nueve `0xFF` seguidos de `0x01`, y los tres vuelven idénticos con `postcard::from_bytes`. `DataHash::of_bytes` se implementa en la tarea de `hash`, porque es la misma función. Lo demuestra: test `ca_08_hash_estado_y_datos_coinciden_con_xxh3_de_referencia`.
- **CA-9.** `random_u64(Seed(0), Tick::ZERO, None, Purpose(0)) == 0xA057_271C_9071_C99D`, el `xxh3_64` de 32 bytes a cero; `random_u64(seed, tick, subject, purpose)` es igual a `xxh3_64` de los 32 bytes `seed.0`, `tick.0`, `raw_or_zero(subject)`, `purpose.0` en little-endian, para 1.000 combinaciones generadas con `proptest` con `subject` en `Option<Handle>` (incluido `None`); cambiar uno solo de los cuatro cambia el valor en al menos 999 de 1.000 casos; `Purpose` no implementa `Default` (por revisión, Q-019); `Purpose::new(7).nth(3)` tiene `id() == 7`, `index() == 3` y `.0 == (7 << 32) | 3`; para cualquier `Draw`: `unit_f32()` y `unit_f64()` están en `[0, 1)`, `below(n) < n` para `n` en `1..=u32::MAX`, `below(1) == 0`, `chance(0, d)` es falso y `chance(d, d)` cierto; `Draw::from_bits(u64::MAX).unit_f32() == 1.0 - 2f32.powi(-24)`; y `below(0)` y `chance(1, 0)` hacen pánico con `rango vacío`. Lo demuestran: tests `ca_09_rng_sin_estado_es_xxh3_de_32_bytes`, `ca_09_rng_rangos_propiedad`, `ca_09_below_cero_panica` y `ca_09_chance_denominador_cero_panica`, en dev y con `cargo test -p engine_core --release`.
- **CA-10.** Cuatro hilos de `std::thread` calculan cada uno la tabla de `random_u64` para 10.000 ticks, 16 sujetos (`None` y 15 handles distintos) y 4 propósitos con la misma semilla, y las cuatro tablas son idénticas entre sí y a la calculada en el hilo principal. Lo demuestra: test `ca_10_rng_identico_en_varios_hilos`.
- **CA-11.** El ejemplo fijado de "Datos y formatos" (`Kind::Replay`, `format_version: 300`, `game_version: Version::new(0, 1, 0)`, `DataHash(1)`, cuerpo `()`) codifica exactamente a los 16 bytes `50 4C 55 53 55 4C 54 52 01 01 AC 02 00 01 00 01`, y `decode::<()>` de esos 16 bytes con la `Expected` correspondiente devuelve esa cabecera y `()`. Con una struct de prueba de unos 3 MB (varias columnas `Vec<f32>`, `Vec<u32>` y `Vec<Handle>`): `encode` seguido de `decode` con la `Expected` correspondiente devuelve la misma cabecera y el mismo cuerpo; `decode_header` devuelve la cabecera y un resto cuya longitud es la del cuerpo; la cabecera completa ocupa ≤ 34 bytes para cualquier `Header`, comprobado con `format_version: u32::MAX`, `Version::new(u16::MAX, u16::MAX, u16::MAX)` y `DataHash(u64::MAX)`; y el archivo entero pesa menos del doble del tamaño en memoria de las columnas. Lo demuestra: test `ca_11_sobre_ida_y_vuelta_y_cabecera_fijada`.
- **CA-12.** Partiendo de un sobre válido: cambiar un byte de `MAGIC` da `BadMagic`; un archivo de 7 bytes da `BadMagic`; un archivo de exactamente 8 bytes iguales a `MAGIC` da `Corrupt`; poner el byte de la posición 8 a `2` da `UnsupportedEnvelope { found: 2 }`; un `kind` con índice 7 en la cabecera da `Corrupt`; codificar con `Kind::Save` y exigir `Kind::Replay` da `UnexpectedKind`; `format_version` distinta da `UnsupportedFormat`; `data_hash` distinto da `DataHashMismatch` con los dos valores (regla 18); una `game_version` distinta de `GAME_VERSION` no es error; añadir un byte al final da `TrailingBytes { count: 1 }`; truncar el cuerpo a la mitad da `Corrupt`; truncar dentro de la cabecera da `Corrupt`; un cuerpo `Vec<Handle>` cuyos bytes llevan un handle con generación 0 da `Corrupt`; un cuerpo cuyo `Serialize` devuelve error da `Encode`; y con una cabecera que falla en todo, `decode` devuelve `UnexpectedKind` antes que `UnsupportedFormat` y este antes que `DataHashMismatch`. Lo demuestra: test `ca_12_sobre_rechaza_magia_tipo_version_y_hash_de_datos`.
- **CA-13.** Un cuerpo codificado como `V1 { a: u32 }` con `format_version: 1` se carga con `decode_header`, se comprueba la versión, se lee con `decode_body::<V1>` y se convierte a `V2 { a: u32, b: u16 }`, mientras `decode::<V2>` con `format_version: 2` exigida devuelve `UnsupportedFormat { expected: 2, found: 1 }`; y en ese mismo camino de migración, `Header::check` con una `Expected` de versión 1 y `data_hash` distinto devuelve `DataHashMismatch` (la migración no esquiva la regla 18). Lo demuestra: test `ca_13_migracion_explicita_por_version_de_formato`.
- **CA-14.** Tras un primer `encode` en un `Vec` vacío, un segundo `encode` del mismo cuerpo en el mismo `Vec` deja `capacity()` igual y `allocation_counter::measure` cuenta 0 asignaciones durante la segunda llamada. Lo demuestra: test `ca_14_encode_reutiliza_el_buffer`.
- **CA-15.** En una carpeta bajo `CARGO_TARGET_TMPDIR` con `b.ron`, `a/c.ron`, `a/B.RON`, `leeme.md` y `z.txt`: `list_data_files` devuelve exactamente tres entradas en el orden `a/B.RON`, `a/c.ron`, `b.ron`, con `relative` usando `/`; `hash_data_tree` da lo mismo si los archivos se crean en otro orden; cambia si cambia un byte de `b.ron`, si `b.ron` pasa a llamarse `bb.ron` o si se añade `d.ron`; no cambia si cambia `leeme.md`; una carpeta vacía da `DataHash::of_bytes(&[])`; una carpeta inexistente da `DataError::Io` con esa ruta; en Windows (test bajo `#[cfg(windows)]`), un archivo cuyo nombre se construye con `OsStringExt::from_wide(&[0x61, 0xD800])` más `.ron` hace que `list_data_files` devuelva `InvalidPath`; y el valor es igual al calculado a mano con un `StateHasher` siguiendo la definición de la API. Lo demuestran: tests `ca_15_hash_de_datos_ordenado_por_ruta_y_solo_ron` y `ca_15_nombre_no_utf8_es_invalid_path`.
- **CA-16.** `parse_ron::<Prueba>("(a: 1, b: )")` devuelve `DataError::Parse` con `line == 1`, `column > 0` y `path == None`; `parse_ron::<Prueba>("")` devuelve `Parse`; `load_ron` del texto malo en un archivo bajo `CARGO_TARGET_TMPDIR` devuelve `Parse` con `path == Some(ruta)`; `load_ron` de una ruta inexistente devuelve `Io` con esa ruta; `load_ron` de un archivo con bytes que no son UTF-8 devuelve `Io` con `source.kind() == InvalidData`; y un archivo que empieza por la marca de orden de bytes UTF-8 seguida de `(a: 1, b: 2)` se carga bien. Lo demuestra: test `ca_16_errores_ron_con_linea_columna_y_ruta`.
- **CA-17.** Sin la feature `tracy` (test bajo `#[cfg(not(feature = "tracy"))]`): `size_of::<Span>() == 0`, `is_active() == false` antes y después de `start()`, `start()` dos veces seguidas no cambia nada, dos zonas anidadas se abren y se cierran en orden, y `span!`, `plot!`, `frame_mark`, `tick_mark` y `set_thread_name` se pueden llamar 100.000 veces sin asignar (medido con `allocation_counter::measure`). Con la feature (test bajo `#[cfg(feature = "tracy")]`, con `cargo test -p engine_core --features tracy`): `is_active()` es cierto sin haber llamado a `start()`, sigue cierto tras `start()` dos veces, y `span!("prueba")`, dos zonas anidadas, una zona abierta en un hilo de `std::thread`, `plot!`, `frame_mark` y `tick_mark` terminan sin pánico. Lo demuestran: tests `ca_17_perfilado_sin_feature_es_nulo` y `ca_17_perfilado_con_feature_no_hace_panico`.
- **CA-18.** `allocation_counter::measure` cuenta 0 asignaciones en: 10.000 llamadas a `random_u64` y a los métodos de `Draw`; un `StateHasher` que escribe un slice de 1 MB, 10.000 `write_u32`, un `write_handles` de 15.000 handles, un `hash_of` de una columna de 15.000 `Vec2` y hace `finish()` cada 1.000 escrituras; 10.000 operaciones de `Tick` y `Handle`; 10.000 consultas `get` a un `FxHashMap<u64, u64>` ya construido con 1.000 entradas; y 10.000 `Handle::from_raw`. Lo demuestra: test `ca_18_primitivas_sin_asignaciones`.
- **CA-19.** Existen y corren con `cargo bench -p engine_core`: `benches/hash_estado.rs` (3 MB repartidos en columnas de `f32`, `u32`, `Vec2`, `Handle` y `Option<Handle>`, cada una por `hash_of`, resultado en milisegundos y por columna, para que se vea cuánto cuesta la columna de `Option<Handle>`, que no tiene vía en bloque), `benches/sobre.rs` (`encode` y `decode` del cuerpo de CA-11) y `benches/rng.rs` (`random_u64`, en extracciones por segundo). Lo demuestra: métrica informativa; el hito 2 decide si el hash de estado recibe umbral y si `Option<Handle>` necesita su propia vía en bloque.
- **CA-20.** `GAME_VERSION` coincide con `env!("CARGO_PKG_VERSION")` del test parseado en tiempo de ejecución, y `GAME_VERSION.to_string()` coincide con esa misma cadena; `env!("CARGO_PKG_VERSION_PRE")` está vacío y el texto de `engine/core/src/version.rs` contiene `CARGO_PKG_VERSION_PRE` dentro de un `const _: () = assert!(...)` (Q-015). Lo demuestran: tests `ca_20_version_del_juego_igual_a_cargo` (P01-T02) y `ca_20_sin_prerrelease` (P01-T09).
- **CA-21.** El texto de `engine/core/src/lib.rs` contiene `#![forbid(unsafe_code)]` y `#![deny(missing_docs)]` (test con `include_str!("../src/lib.rs")`); `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_core --no-deps` termina sin avisos; `cargo tree -p engine_core -e normal --depth 1` lista solo los crates de "Dependencias permitidas"; y `cargo tree -p engine_core -e normal` no contiene `heapless`, `wgpu`, `winit`, `egui`, `rayon`, `rand`, `getrandom`, `hashbrown`, `ahash`, `foldhash`, `indexmap` ni ningún `game_*` (los seis de aleatoriedad y colecciones, por Q-014). Lo demuestran: test `ca_21_lib_rs_lleva_los_atributos` y la puerta adicional, cuyos dos `cargo tree` compara el orquestador con la lista en `/verificar`.
- **CA-22.** Existen `engine/core/clippy.toml`, `engine/sim/clippy.toml` y `game/rules/clippy.toml`, idénticos, con `disallowed-types` para `std::collections::HashMap`, `std::collections::HashSet` y `std::hash::RandomState` con un motivo que cite la regla 13 y el ADR-0009, y para `glam::Vec3A`, `glam::Mat3A` y `glam::Affine3A` con un motivo que cite la regla 6 (su carril de relleno no es canónico y `Pod` permite hashearlo en bloque; se usan `Vec3`, `Mat3` y `Mat4`; Q-016); un test lee los tres (el propio con `include_str!("../clippy.toml")`, los otros dos desde `CARGO_MANIFEST_DIR`) y comprueba que son idénticos y contienen las seis rutas; `cargo clippy --workspace --all-targets -- -D warnings` pasa; y, al añadir dentro de una función cualquiera de `engine_core` la línea `let _m: std::collections::HashMap<u8, u8> = std::collections::HashMap::new();`, o la línea `let _v = glam::Vec3A::ZERO;`, `cargo clippy -p engine_core -- -D warnings` falla con `disallowed_types`. Si `collections.rs` necesita nombrar `HashMap` para definir los alias, lleva `#[allow(clippy::disallowed_types)]` a nivel de módulo con su comentario. Lo demuestran: test `ca_22_tres_clippy_toml_identicos` y la prueba negativa de la línea añadida, que hace el orquestador en el worktree de la tarea y retira antes de integrar.
- **CA-23.** `xtask` pasa a tener `src/lib.rs` (con `#![forbid(unsafe_code)]`) con las comprobaciones de `check-arch` como funciones puras sobre datos ya extraídos: para la nueva, `direct_dependency_violations(members)` recibe, por cada miembro del workspace, su nombre y sus dependencias directas con su tipo (normal, de compilación o de desarrollo), y devuelve un mensaje que cite el ADR-0009 por cada miembro distinto de `engine_core` que declare `rustc-hash`, `xxhash-rust` o `bytemuck`, y un mensaje que cite Q-014 por cada vez que `engine_sim` o `game_rules` declare `hashbrown`, `ahash`, `foldhash`, `indexmap` o un crate cuyo nombre empiece por `rand`, en cualquiera de los tres tipos de dependencia; `main.rs` la alimenta con `cargo_metadata` y falla si devuelve algo. `cargo_config_violations(text)`, sobre el texto de `.cargo/config.toml`, devuelve un mensaje que cite Q-011 si contiene `target-cpu` o `target-feature`; `main.rs` la llama con el archivo real si existe. Y `allow_disallowed_types_violations(files)`, sobre pares (ruta relativa, texto) de los `.rs` bajo `engine/core`, `engine/sim` y `game/rules`, devuelve un mensaje por cada archivo distinto de `engine/core/src/collections.rs` cuyo texto contenga `allow(clippy::disallowed_types)` (Q-014); `main.rs` la alimenta recorriendo esas tres carpetas. Con el workspace tal cual, `cargo xtask check-arch` sigue pasando. Lo demuestran: tests en `xtask/tests/aceptacion_check_arch_t10.rs` con entradas construidas a mano: un miembro ajeno con cada uno de los tres crates del ADR-0009 en cada uno de los tres tipos de dependencia da error, `engine_core` con los tres no da error, `engine_sim` con `hashbrown` o con `rand_xoshiro` da error y `game_app` con ellos no, un workspace sin ellos no da error, un texto de configuración con `target-cpu` o con `target-feature` da error y el texto del `.cargo/config.toml` actual no, un archivo `engine/sim/src/x.rs` con `#[allow(clippy::disallowed_types)]` da error y `engine/core/src/collections.rs` con la misma línea no; y la puerta, donde `check-arch` corre en cada integración. Es una tarea propia del plan, que toca `xtask/` y lo dice en su encargo.
- **CA-24.** `deny.toml` lleva en `[bans] features` una entrada con `exact = true` para `glam` y otra para `rustc-hash`, cada una con la lista exacta de features que el workspace activa hoy (la tarea la confirma con `cargo tree -p engine_core -e features` y la deja escrita en el propio `deny.toml` con un comentario que cite Q-014); `cargo deny check` pasa con el workspace tal cual; y, al añadir `glam = { workspace = true, features = ["scalar-math"] }` a `engine/sim/Cargo.toml`, o `rustc-hash = { workspace = true, features = ["rand"] }`, `cargo deny check` falla en `bans`. Una feature nueva de esos dos crates entra solo por una tarea de contrato que cambie la lista y la spec de quien la pida. Lo demuestran: la puerta y las dos pruebas negativas, que hace y retira el orquestador en el worktree de P01-T10.

**Reparto previsto en tareas**, para `/planificar`. Todas van en serie, porque una tarea que toca `engine_core` nunca va en paralelo con otra, y los contratos van antes que las implementaciones:

1. Contrato A: `[workspace.dependencies]`, `engine/core/Cargo.toml` (feature `tracy`, dev-dependencies y los tres `[[bench]]` con `harness = false`), los tres `clippy.toml` (CA-22), los tres archivos de `benches/` con solo `fn main() {}` para que la puerta compile desde el principio, y las firmas con `todo!()` de `time`, `handle`, `version`, `collections` y `math`.
2. Implementación de `time`, `handle`, `version`, `collections` y `math` (CA-1 a CA-5, CA-20, CA-22).
3. Contrato B1: firmas de `hash`, `rng` y `data` (`hash` necesita `DataHash` y `Seed`).
4. Contrato B2: firmas de `save` y `profile`.
5. `hash`, más `DataHash::of_bytes` (CA-6 a CA-8).
6. `rng` (CA-9, CA-10).
7. `data` salvo `of_bytes` (CA-15, CA-16).
8. `save` (CA-11 a CA-14).
9. Cierre: `profile`, los tres benchmarks, la aserción de prerrelease de `version` y CA-17 a CA-21; CA-18 va aquí porque cruza módulos.
10. Extensión de `check-arch` en `xtask` y features fijadas en `deny.toml` (CA-23, CA-24).
11. Añadida el 31 de octubre de 2026 (Q-016), ligera, entre la 4 y la 5: `Vec3A`, `Mat3A` y `Affine3A` en los tres `clippy.toml` (CA-22).
12. Añadida el 31 de octubre de 2026 (Q-018, Q-019), contrato, entre la 5 y la 6: `rng` con sujeto `Option<Handle>` y `Purpose` sin `Default`.

Cada contrato ronda las 300 líneas con su documentación en español; ninguna implementación pasa de un módulo.

## Casos límite

- **Tiempo.** `Tick::ZERO.since(Tick::ZERO) == Ticks::ZERO` (CA-1); `Tick(u64::MAX).next()`, `Tick(u64::MAX) + Ticks::ONE` y `Ticks(u64::MAX) * 2` hacen pánico `desbordamiento de ticks` en cualquier perfil (CA-1); restar más de lo transcurrido, en `Tick` o en `Ticks`, hace pánico `retroceso de ticks` (CA-1); `is_multiple_of` con periodo 0 hace pánico `periodo cero` (CA-1); `Ticks(u64::MAX).to_wall_duration()` no desborda (CA-2). Sin test, por revisión: `Ticks(19).to_wall_duration()` son 950 ms y `Ticks(21)` es 1 s y 50 ms, el resto máximo y el primer salto de segundo.
- **Handle.** Generación `u32::MAX` seguida de `next_generation` vuelve a 1 (CA-3); `from_raw` con generación 0 da `None` (CA-3); dos handles con el mismo slot y distinta generación son distintos y se ordenan por generación (CA-3); un handle de una tabla es válido en otra si coinciden slot y generación: la tabla receptora no puede detectarlo y es responsabilidad de quien pasa handles entre tablas (decisión 1). Sin test, por revisión: `Handle::new(0, 1).to_raw() == 1` es el raw no nulo más pequeño y lo distingue de "sin handle" en `raw_or_zero`.
- **Hash de estado.** Slice vacío: entra solo la longitud 0 (CA-6); `finish()` sin escribir da el xxh3 de la secuencia vacía (CA-8); escrituras de un byte repetidas un millón de veces dan lo mismo que una sola de un millón (CA-6); `-0.0`, `0.0` y los NaN se distinguen (CA-7); `usize` no entra; `write_handles` con slice vacío no escribe nada (CA-7); `Option<Handle>` no tiene vía en bloque y el benchmark lo mide (CA-19).
- **Aleatoriedad.** Semilla, tick, sujeto y propósito todos a 0 dan `0xA057_271C_9071_C99D`, no un caso especial (CA-9); `below(1) == 0` (CA-9); `below(u32::MAX)` nunca devuelve `u32::MAX` (CA-9); `unit_f32` de `u64::MAX` es exactamente `1 - 2^-24` (CA-9).
- **Sobre.** Cuerpo vacío (`()`) codifica y decodifica (CA-11); menos de 8 bytes o `MAGIC` distinto: `BadMagic`; exactamente `MAGIC` sin byte de versión: `Corrupt`; byte de la posición 8 distinto de 1: `UnsupportedEnvelope`; cabecera truncada: `Corrupt`; `Kind` con un índice desconocido: `Corrupt`; handle con generación 0 en el cuerpo: `Corrupt`; `Serialize` que falla: `Encode`; un cuerpo que termina justo donde acaba el archivo no da `TrailingBytes`; una `game_version` distinta de `GAME_VERSION` no es error (todo en CA-12).
- **Datos.** Carpeta vacía (CA-15); carpeta inexistente (`Io`, CA-15); extensión en mayúsculas (CA-15); subcarpetas anidadas (CA-15); nombre no UTF-8 (`InvalidPath`, CA-15, solo en Windows); archivo RON con marca de orden de bytes (CA-16); archivo vacío (`Parse`, CA-16); contenido no UTF-8 (`Io` con `InvalidData`, CA-16). Sin test, por revisión: enlaces simbólicos ignorados, porque crearlos en Windows exige privilegios.
- **Perfilado.** `start()` dos veces, zonas anidadas, zona abierta en un hilo de trabajo, y zonas sin haber llamado a `start()` con la feature activa (CA-17).
- **Hilos.** Nada de este crate cambia con el número de hilos: CA-5 y CA-10 lo comprueban para lo único que podría, hashers y aleatoriedad.

## Fuera de alcance

- Tablas SoA, grid, scheduler de fases, pipeline de eventos, borrado diferido y conjunto activo: `engine_sim`.
- El cuerpo del guardado y del replay, el versionado de sus structs y las migraciones: `engine_sim` y `game_rules`. El formato de replay como tal (semilla, hash de datos y órdenes selladas por tick) se cierra en la spec de `engine_sim`, usando el sobre de aquí con `Kind::Replay`.
- Qué archivos RON existen, sus structs y su validación (regla 11): `specs/engine/simulacion-sintetica.md` y la spec de `game_rules`.
- Hot-reload y vigilancia de archivos: `game_app`, detrás de una feature de desarrollo.
- Acumulador de tiempo real, interpolación y desfase simulación-render: `game_app` y `engine_render`.
- Contadores deterministas de vecinos (regla 9), hash de estado "cada N ticks en release" y buscador de desincronizaciones: `engine_sim`, con `StateHasher` de aquí.
- Las derivaciones `Pod` y `Zeroable` de `bytemuck` para las structs del snapshot: si `engine_render` o `engine_sim` las necesitan, su spec decide entre `#[bytemuck(crate = "engine_core::hash::bytemuck")]` sobre el reexport, si la versión de `bytemuck_derive` lo admite, o ampliar CA-23 con el autor para declarar `bytemuck` directamente.
- Vía en bloque para `Option<Handle>`: se decide con la medición de CA-19 en el hito 2.
- Fachada de logging: no entra en el hito 0; la decide la spec de `game_app`.
- Ids concretos de `Purpose` y semillas de partida: quien los use.
- Hashes dorados de simulación: los fijan las specs de features cuando exista el código que los produce. Los de aquí (CA-8, CA-9, CA-11) son del algoritmo y del formato, no de ninguna partida.
- El asignador contador fuera de los tests y cualquier medición de memoria en producción.
- Optimización más allá de lo que piden las estimaciones: ni SIMD propio ni hash de 128 bits.

## Decisiones de redacción

Las tomó el orquestador al escribir esta spec porque ningún documento las fijaba. El autor las confirma o las cambia al aprobar; cada una dice qué criterios cambian si no se confirma.

1. **Un solo `Handle` sin identificador de tabla.** ADR-0004 dice "el mismo tipo para todo" y no reserva bits para la tabla. Añadirlos costaría 8 bits de slot y abriría la puerta a dos modelos de identidad. Las tablas validan por generación; el uso cruzado entre tablas es un error de programación que se caza con tests de propiedades. Cambia CA-3.
2. **Generación `NonZeroU32`.** Así `Option<Handle>` ocupa 8 bytes y las columnas "objetivo" de las tablas no pagan un byte de etiqueta ni necesitan un valor centinela que alguien olvide comprobar. El coste: la generación arranca en 1 y `next_generation` salta el 0. Cambia CA-3, CA-4 y CA-6.
3. **Pánico explícito, igual en dev y en release, en toda la aritmética de ticks.** Un retroceso o un desbordamiento son errores de lógica; un pánico los enseña en el primer replay y una saturación o una envoltura los esconden durante horas, y un comportamiento distinto por perfil haría que un replay difiriera entre dev y release. `checked_since` y `checked_sub` cubren los casos legítimos. Cambia CA-1.
4. **`Ticks` como tipo propio de duración.** La regla 17 habla de "un único tipo `Tick`" y de "las duraciones como diferencias de ticks"; `Ticks` es esa diferencia con nombre, para que el compilador no deje sumar dos instantes. Cambia CA-1 y CA-2.
5. **`allocation-counter` 0.8 fijado aquí**, adelantando lo que Q-003 dejaba a la spec de la simulación sintética, porque CA-14, CA-17 y CA-18 lo necesitan y ya está verificado (ver "Dependencias permitidas"). Su contador es por hilo: la spec de la simulación sintética tiene que medir dentro de los hilos de trabajo de `rayon` o medir con el pool a un solo hilo y el tick ejecutado en el hilo de test, y lo decide ella. Si el autor prefiere no adelantarlo, CA-14, CA-17 y CA-18 pasan a esa spec.
6. **Sobre con dos versiones.** `ENVELOPE_VERSION` para la cabecera y `format_version` para el cuerpo, de forma que `engine_sim` y `game_rules` versionen sus structs sin tocar este crate. ADR-0007 habla de una sola "versión del formato"; esta es una extensión, no un cambio. Cambia CA-11 a CA-13.
7. **`glam` reexportado entero** en `math` en vez de una lista cerrada de tipos, para que ningún crate tenga que pedir un cambio de contrato por un `IVec4`.
8. **Tracy mínimo y con el ciclo de vida automático.** `tracy-client` sin features por defecto y solo con `enable`: las de muestreo piden privilegios en Windows y compilan más C++. Sin `manual-lifetime`, el cliente arranca con el proceso y `start()` es una cortesía idempotente; la alternativa, `manual-lifetime`, obligaría a decidir quién apaga el perfilador. Si una medición del hito 2 necesita más, se añade en la spec que lo pida. Cambia CA-17.
9. **`#![deny(missing_docs)]`** en este crate, porque es el contrato de todos los demás y lo leerán agentes. Cambia CA-21.
10. **Ruta y longitud delante de cada archivo en el hash de datos.** ADR-0009 dice "bytes crudos de cada archivo en orden lexicográfico"; esta spec añade la ruta relativa, un byte 0 y la longitud antes del contenido. Sin eso, mover bytes de un archivo al siguiente o renombrar un archivo no cambiaría el hash. Cambia CA-15.
11. **Tres `clippy.toml` iguales** en vez de uno en la raíz, por cómo busca clippy su configuración (ver "Contratos compartidos"). Cambia CA-22.
12. **`write_handles` como vía en bloque de las columnas de handles**, porque la memoria de `Handle` es (slot, generación) y no coincide con el `u64` de `to_raw()` en little-endian, y una columna de 15.000 handles elemento a elemento no cabría en la estimación de coste. Cambia CA-7, CA-18 y CA-19.
13. **`hash_state_slice` como gancho de slice del trait `HashState`**, con implementación por defecto, porque Rust estable no tiene especialización y sin el gancho `hash_of` de una columna iría elemento a elemento. Cambia CA-6 y CA-7.
14. **La puerta adicional del crate se declara en la spec y la copia el plan**, porque `/verificar` solo corre las cinco comprobaciones comunes. Si el autor prefiere que `/verificar` ejecute siempre la puerta adicional que declare el `CLAUDE.md` del crate, es una línea en `.claude/skills/verificar/SKILL.md` y la decide él. Cambia CA-21.

## Preguntas abiertas

Ninguna. Las respondidas están incorporadas arriba; el detalle de cada una está en `specs/preguntas-pendientes.md`. De la verificación de P01-T01 a P01-T04 salieron Q-012 a Q-021, respondidas el 31 de octubre de 2026:

- **Q-012.** La feature `tracy` no compila con el Windows SDK 10.0.19041: el autor actualiza el SDK; no cambia nada aquí.
- **Q-013.** El compromiso de determinismo es "misma máquina, mismo binario". Los dorados de la CI no son comparables con los locales mientras `glam` use las trascendentes de la UCRT (nota en el ADR-0009). Sin `libm` por ahora.
- **Q-014.** Features de `glam` y `rustc-hash` fijadas en `deny.toml` (CA-24); listas de CA-21 y CA-23 ampliadas con crates de aleatoriedad y colecciones; `check-arch` busca `allow(clippy::disallowed_types)` fuera de `collections.rs`.
- **Q-015.** Sin sufijo de prerrelease en la versión del workspace; aserción en compilación en `version` (CA-20).
- **Q-016.** `Vec3A`, `Mat3A` y `Affine3A` vetados en los tres `clippy.toml` (CA-22).
- **Q-017.** CA-7 comprueba la vía en bloque en todas las filas de la tabla.
- **Q-018.** El sujeto de `random_u64` y `draw` es `Option<Handle>` (CA-9, CA-10).
- **Q-019.** `Purpose` sin `Default`; la doc de `nth` fija de dónde sale el índice.
- **Q-022.** La doc de `nth` cierra el índice denso y el orden de los hilos; la de `random_u64` promete lo que el tipo garantiza. La spec de la simulación sintética ejercita un borrado antes del punto de guardado.
- **Q-023.** El campo de `Purpose` sigue público; `new` y `nth` son la vía recomendada. Se reabre si aparece un uso a mano en `engine_sim` o `game_rules`.
- **Q-020.** `UnexpectedKind` usa el `Debug` de `Kind`.
- **Q-021.** `plot!` con la feature cachea un `PlotName::new_leak` por punto de uso; ningún `unsafe` expandido en el llamante.

Las dos de la revisión de la spec las respondió el autor el 26 de octubre de 2026, las dos con la opción (a):

- **Q-010.** El hash de datos cubre solo los `*.ron` bajo `data/` (`DATA_EXTENSION`, "Datos y formatos", CA-15). `data/README.md` queda aclarado en el mismo commit; la nota en el ADR-0009 va con el siguiente commit que toque ese archivo.
- **Q-011.** `-C target-cpu` y `-C target-feature` quedan prohibidos en `.cargo/config.toml` de todo el workspace hasta que el hito 2 mida, y lo vigila `check-arch` (`math`, CA-23). La constitución no tiene regla para esta comprobación: cuando el hito 2 decida si la prohibición se queda, se le pone regla o se retira la comprobación.

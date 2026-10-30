# Plan P01: `engine_core`

- **Spec:** `specs/architecture/engine_core.md` (aprobada el 26 de octubre de 2026)
- **Hito:** 0
- **Rama de integración:** main
- **Estado:** aprobado

## Enfoque

**Punto de partida.** `engine/core` es un esqueleto: `src/lib.rs` con la documentación y `#![forbid(unsafe_code)]`, un `Cargo.toml` sin dependencias, ningún test y ningún `clippy.toml` en el repositorio. `[workspace.dependencies]` solo declara `anyhow` y `cargo_metadata`, y `xtask` es un único `main.rs` con las comprobaciones de las reglas 1, 2, 3 y 16. Todo lo que pide la spec se crea desde cero.

**Contratos primero, en tres tandas.** Las firmas de la spec entran con cuerpos `todo!()` y su documentación en español antes de cualquier implementación, para que `autor-tests` escriba contra ellas y compile con `cargo test --no-run`. Van en tres tareas de contrato en serie, porque juntas pasan de las 400 líneas: la A con `time`, `handle`, `version`, `collections` y `math`, más todo el `Cargo.toml`; la B1 con `hash`, `rng` y `data`, porque `hash` necesita `DataHash` y `Seed`; y la B2 con `save` y `profile`. Cada contrato deja la puerta completa en verde: `cargo check --workspace --all-targets`, `check-arch`, `clippy` y `cargo deny`. `#![deny(missing_docs)]` entra con el contrato A, así que la documentación es parte del contrato y no una tarea aparte.

**El `Cargo.toml` entero en el contrato A.** Las nueve dependencias normales y las tres de desarrollo entran en `[workspace.dependencies]` con las líneas exactas de la spec, y en `engine/core/Cargo.toml` con `nombre.workspace = true` (`tracy-client` además `optional = true`, detrás de la feature `tracy`). Los tres `[[bench]]` con `harness = false` llevan desde el principio su archivo en `benches/` con `fn main() {}`, porque un `[[bench]]` sin archivo rompe `cargo metadata` y con él toda la puerta. Los tres `clippy.toml` idénticos (`engine/core/`, `engine/sim/`, `game/rules/`) con `disallowed-types` nacen aquí, porque clippy busca la configuración desde la carpeta del manifiesto de cada crate hacia arriba. `xxhash-rust` y `postcard` entran también como dependencias de desarrollo del crate, porque `autor-tests` solo puede declararlas con `workspace = true` y los tests de CA-4, CA-8 y CA-9 las comparan con la implementación de referencia.

**`time`, `handle`, `version`, `collections` y `math`.** Aritmética de ticks con `checked_add`, `checked_sub`, `checked_mul` y `assert!`, con los mensajes fijos de la spec, para que dev y release hagan lo mismo. `Handle` como `#[repr(C)]` de `u32` más `NonZeroU32`, con `to_raw` como `(slot << 32) | generation` y `next_generation` que salta el 0; el tamaño de `Option<Handle>` sale solo del nicho de `NonZeroU32`. `GAME_VERSION` se construye en una `const fn` que parsea `CARGO_PKG_VERSION_MAJOR`, `_MINOR` y `_PATCH`. `collections` son alias y reexports de `rustc-hash`, y `math` un `pub use glam::*`. Sin flotantes en `time.rs` ni en `handle.rs`, ni en comentarios, porque CA-2 lee el texto de los archivos.

**`hash`.** `StateHasher` envuelve un `Xxh3` en streaming con semilla 0; cada `write_*` escribe los bytes little-endian del valor y `finish` llama a `digest` sin consumir. `HashState` lleva el gancho `hash_state_slice` con implementación por defecto, que los primitivos, `bool`, los flotantes y los vectores de `glam` sobrescriben con un solo `write(bytemuck::cast_slice(items))`, y `Handle` con `write_handles`, que copia los `to_raw()` en bloques a un buffer de pila de tamaño fijo. `const _: () = assert!(cfg!(target_endian = "little"))` protege la vía en bloque. `DataHash::of_bytes` se implementa en esta misma tarea, porque es `xxh3_64` y vive en la misma dependencia.

**`rng`.** `random_u64` rellena un `[u8; 32]` con los cuatro `u64` en little-endian y llama a `xxh3_64`; `Draw` hace las conversiones con aritmética entera (`u128` para `below`) y divisiones por potencias de dos para `unit_f32` y `unit_f64`. Sin estado y sin asignaciones por construcción.

**`data`.** `list_data_files` recorre el árbol con `read_dir` sin seguir enlaces simbólicos, construye la ruta relativa con `/`, falla con `InvalidPath` ante un nombre que no es UTF-8, filtra por extensión `ron` sin distinguir mayúsculas y ordena por los bytes UTF-8 de la ruta relativa. `hash_data_tree` alimenta un `StateHasher` con ruta, byte 0, longitud y contenido de cada archivo en ese orden. `parse_ron` quita la marca de orden de bytes y traduce el error de `ron` a `Parse` con línea y columna desde 1; `load_ron` lee con `read_to_string`, de modo que un contenido que no es UTF-8 llega como `Io` con `InvalidData`.

**`save`.** `encode` vacía el `Vec`, escribe `MAGIC`, `ENVELOPE_VERSION`, la cabecera y el cuerpo con `postcard::to_io` sobre `&mut Vec<u8>`, que reutiliza la capacidad; `decode_header` valida los 9 primeros bytes y lee la cabecera con `take_from_bytes`, devolviendo el resto; `decode_body` exige consumirlo todo; `decode` encadena los tres con `Header::check` en medio, en el orden de errores que fija CA-12. Los errores de `postcard` se convierten a `Corrupt(String)` o `Encode(String)`.

**`profile`.** Dos definiciones de cada macro (`span!`, `plot!`), una bajo `#[cfg(feature = "tracy")]` y otra bajo `#[cfg(not(feature = "tracy"))]`, apoyadas en constructores `#[doc(hidden)]` de `Span` y en un reexport oculto de `tracy_client`. Sin la feature, `Span` es un tipo de tamaño cero con un `PhantomData<*const ()>` para que tampoco sea `Send`. Con la feature, el cliente de Tracy arranca con el proceso (sin `manual-lifetime`), `start` solo llama a `Client::start()`, `is_active` devuelve cierto, y las zonas usan `span_location!` más `Client::span`.

**Cierre.** Los tres benchmarks con `criterion` (hash de 3 MB por columnas, sobre de 3 MB, extracciones por segundo), el test de cero asignaciones que cruza módulos (CA-18) y los tests de documentación y atributos (CA-21). La puerta adicional del crate corre aquí por primera vez entera, incluida la feature `tracy`.

**Extensión de `check-arch`.** `xtask` gana un `src/lib.rs` con `#![forbid(unsafe_code)]` y dos funciones puras: `direct_dependency_violations`, sobre los nombres de los miembros y sus dependencias directas con su tipo, y `cargo_config_violations`, sobre el texto de `.cargo/config.toml`. `main.rs` las alimenta con `cargo_metadata` y con el archivo real si existe. Los tests viven en `xtask/tests/` con entradas construidas a mano, que es lo que el hook permite escribir a `autor-tests`.

## Contratos

| Contrato | Crate | Quién lo usa | ¿Compartido? |
| --- | --- | --- | --- |
| `time::{Tick, Ticks, TICK_RATE_HZ, TICKS_PER_SECOND, TICK_DURATION}` | `engine_core` | todos los crates; `engine_sim` para el bucle y las órdenes selladas | sí (`Tick`) |
| `handle::{Handle, raw_or_zero}` | `engine_core` | `engine_sim` (tablas, eventos), `game_rules`, `engine_render` (snapshot, hito 1) | sí (handle) |
| `collections::{FxHashMap, FxHashSet, FxBuildHasher, FxHasher, fx_map_with_capacity, fx_set_with_capacity}` | `engine_core` | `engine_sim`, `game_rules` | sí (ADR-0009: único lugar que nombra `rustc-hash`) |
| `hash::{StateHash, StateHasher, HashState, hash_of, bytemuck}` | `engine_core` | `engine_sim` (hash por tick y por fase, tablas), `game_rules` (structs canónicas) | sí (hash de estado) |
| `rng::{Seed, Purpose, Draw, random_u64, draw}` | `engine_core` | `engine_sim`, `game_rules` | sí (aleatoriedad sin estado) |
| `save::{MAGIC, ENVELOPE_VERSION, Kind, Header, Expected, SaveError, encode, encode_to_vec, decode_header, decode_body, decode}` | `engine_core` | `engine_sim` (replay y guardado), herramientas | sí (formatos de replay y guardado: la cabecera) |
| `data::{DataHash, DATA_EXTENSION, DataFile, DataError, list_data_files, hash_data_tree, parse_ron, load_ron}` | `engine_core` | quien carga datos (`game_rules`, herramientas, simulación sintética), quien guarda | sí (hash de datos, regla 18) |
| `version::{Version, GAME_VERSION}` | `engine_core` | `save`, `engine_sim` | sí (va en la cabecera del sobre) |
| `profile::{start, is_active, frame_mark, tick_mark, set_thread_name, Span}` y las macros `span!`, `plot!` | `engine_core` | `engine_sim` (zona `tick`), `engine_render`, `game_app` | no es formato, pero toda la medición pasa por aquí |
| `math` (`pub use glam::*`) | `engine_core` | `engine_sim` (`Vec2` del grid), `game_rules` | sí (ADR-0009, Q-008: los demás no declaran `glam`) |
| Feature `tracy` | `engine_core` | `engine_sim` la reexpone como `tracy = ["engine_core/tracy"]` | sí |
| `[workspace.dependencies]`: `glam`, `serde`, `postcard`, `ron`, `rustc-hash`, `xxhash-rust`, `bytemuck`, `thiserror`, `tracy-client`, `proptest`, `criterion`, `allocation-counter` | `Cargo.toml` raíz | todos; `autor-tests` solo puede declarar lo que ya está aquí | sí (regla 1 del orquestador) |
| `engine/core/clippy.toml`, `engine/sim/clippy.toml`, `game/rules/clippy.toml` con `disallowed-types` | tres carpetas | `clippy` de los tres crates | sí (ADR-0009) |
| `xtask` como biblioteca: `direct_dependency_violations`, `cargo_config_violations` | `xtask` | la puerta de todos (`cargo xtask check-arch`) | sí (comprobación de la constitución, Q-008 y Q-011) |

Todo lo anterior es contrato compartido en el sentido de `CLAUDE.md`: se congela en las tres tareas de contrato, que van solas y se integran antes de cualquier implementación. Ninguna tarea de este plan abre paralelismo (ver "Paralelismo").

## Riesgos

| Riesgo | Cómo se retira |
| --- | --- |
| Alguna versión o feature no resuelve en la máquina del autor (`glam` 0.34 con `serde` y `bytemuck`, `postcard` sin `heapless`, `tracy-client` con solo `enable`, `allocation-counter` 0.8) | Las versiones se comprobaron en crates.io el 26 de octubre de 2026. El contrato A ejecuta `cargo check --workspace --all-targets` y `cargo deny check` y termina `bloqueado` si algo no resuelve, sin inventar sustitutos. |
| `tracy-client` compila C++ y necesita la toolchain de MSVC; podría no compilar o tardar mucho | La feature está apagada por defecto y nada de la puerta común la usa. La prueba con `--features tracy` entra en el cierre; si no compila, se anota en el informe y lo decide el autor, sin bloquear el resto. |
| `disallowed-types` marca los alias `FxHashMap` o algún uso legítimo | CA-22 ya prevé `#[allow(clippy::disallowed_types)]` a nivel de módulo en `collections.rs` con su comentario. Si marcara algo en otro crate, se revisa el mecanismo en la spec de la simulación sintética. |
| `clippy::pedantic` con `-D warnings` produce avisos en masa (`cast_possible_truncation`, `cast_precision_loss` en `Draw`, `missing_const_for_fn`) | `allow` puntuales con comentario que justifique cada uno, nunca a nivel de crate; `revisor-constitucion` lo revisa en cada tarea. |
| `size_of::<Option<Handle>>() == 8` depende de la optimización de nicho del compilador | Comprobado con el toolchain actual y exigido por CA-3. Si una versión futura lo rompiera, sería un cambio de contrato con ADR. |
| Las constantes doradas (xxh3 de la entrada vacía y de 32 ceros, vectores `postcard`) no coinciden | Se calcularon con `xxhash-rust` 0.8.19 y `postcard` 1.1.3, las versiones que fija el contrato. Si una subida de versión las cambiara, CA-4, CA-8 y CA-9 fallan y se aplica ADR-0009: se corrige, no se regenera. |
| Los tests con `include_str!` se acoplan al nombre de los archivos de `src/` | El reparto de módulos en archivos es parte de la spec ("Qué hace") y no cambia sin tarea de contrato. |
| La tarea de `hash` pasa de 400 líneas (17 métodos, unas 30 implementaciones de `HashState`, `write_handles`) | Si al escribirla se ve que pasa, se parte en dos: `StateHasher` con primitivos por un lado, y las implementaciones compuestas, los vectores de `glam` y `write_handles` por otro. |
| La vía en bloque (`cast_slice`, `write_handles`) produce bytes distintos de la secuencia elemento a elemento | CA-6 y CA-7 comparan las dos vías y la tabla de `HashState` fila a fila con vectores escritos a mano. |
| El implementador de la extensión de `check-arch` no puede tocar `xtask/` sin permiso expreso | El encargo de esa tarea lo dice expresamente. El nuevo `xtask/src/lib.rs` lleva `#![forbid(unsafe_code)]`, porque `check-arch` se comprueba también a sí mismo. |
| Las pruebas negativas de CA-22 y CA-23 y los `cargo tree` de CA-21 son manuales | Los hace el orquestador en el worktree de la tarea y los anota en el informe de `/verificar`; los revisores son de solo lectura y no pueden hacerlas. |
| `RUSTDOCFLAGS="-D warnings"` no vale en PowerShell | La puerta adicional lleva el equivalente: `$env:RUSTDOCFLAGS = "-D warnings"; cargo doc -p engine_core --no-deps; Remove-Item Env:RUSTDOCFLAGS`. |
| `allocation-counter` cuenta por hilo | En este crate no hay hilos en el camino medido: CA-18 mide en el hilo del test. El problema es de la spec de la simulación sintética y ya está anotado allí. |
| `/verificar` no ejecuta hoy la puerta adicional del crate | Hasta que el autor decida si cambia el skill, cada tarea de este plan lleva la puerta adicional en su texto y el orquestador la ejecuta a mano en `/verificar`. |

## Verificación

Tests de aceptación en `engine/core/tests/aceptacion_core_tNN.rs`, uno por tarea de implementación, y `xtask/tests/aceptacion_check_arch_t10.rs` para la extensión de `check-arch`. Las carpetas temporales van bajo `CARGO_TARGET_TMPDIR` y los bytes pseudoaleatorios salen de un `xorshift64` de semilla fija. La columna "Tarea" sigue el reparto previsto en la spec y se confirma en `/tareas`.

| Criterio | Cómo se demuestra | Tarea |
| --- | --- | --- |
| CA-1 | tests `ca_01_tick_aritmetica_entera`, `ca_01_since_retroceso_panica`, `ca_01_resta_de_ticks_panica`, `ca_01_resta_de_duraciones_panica`, `ca_01_periodo_cero_panica`, `ca_01_next_desbordamiento_panica`, `ca_01_suma_desbordamiento_panica`, `ca_01_producto_desbordamiento_panica`, en dev y con `--release` | P01-T02 |
| CA-2 | test `ca_02_frecuencia_y_duracion_del_tick_sin_flotantes` (`include_str!` de `time.rs` y `handle.rs`) | P01-T02 |
| CA-3 | tests `ca_03_handle_ocho_bytes_y_orden_igual_al_raw` y `ca_03_handle_raw_ida_y_vuelta_propiedad` (`proptest`) | P01-T02 |
| CA-4 | test `ca_04_tick_handle_y_version_postcard_con_bytes_conocidos` | P01-T02 |
| CA-5 | test `ca_05_colecciones_fx_sin_estado_aleatorio` | P01-T02 |
| CA-6 | tests `ca_06_hash_estado_independiente_del_troceo` y `ca_06_hash_estado_tabla_fila_a_fila` | P01-T05 |
| CA-7 | test `ca_07_hash_estado_flotantes_bit_a_bit_y_via_en_bloque` | P01-T05 |
| CA-8 | test `ca_08_hash_estado_y_datos_coinciden_con_xxh3_de_referencia` (con `xxhash-rust` y `postcard` como dependencias de desarrollo) | P01-T05 |
| CA-9 | tests `ca_09_rng_sin_estado_es_xxh3_de_32_bytes`, `ca_09_rng_rangos_propiedad`, `ca_09_below_cero_panica`, `ca_09_chance_denominador_cero_panica`, en dev y con `--release` | P01-T06 |
| CA-10 | test `ca_10_rng_identico_en_varios_hilos` | P01-T06 |
| CA-11 | test `ca_11_sobre_ida_y_vuelta_y_cabecera_fijada` | P01-T08 |
| CA-12 | test `ca_12_sobre_rechaza_magia_tipo_version_y_hash_de_datos` | P01-T08 |
| CA-13 | test `ca_13_migracion_explicita_por_version_de_formato` | P01-T08 |
| CA-14 | test `ca_14_encode_reutiliza_el_buffer` (`allocation-counter`) | P01-T08 |
| CA-15 | tests `ca_15_hash_de_datos_ordenado_por_ruta_y_solo_ron` y `ca_15_nombre_no_utf8_es_invalid_path` (este último `#[cfg(windows)]`) | P01-T07 |
| CA-16 | test `ca_16_errores_ron_con_linea_columna_y_ruta` | P01-T07 |
| CA-17 | tests `ca_17_perfilado_sin_feature_es_nulo` (`#[cfg(not(feature = "tracy"))]`) y `ca_17_perfilado_con_feature_no_hace_panico` (`cargo test -p engine_core --features tracy`) | P01-T09 |
| CA-18 | test `ca_18_primitivas_sin_asignaciones` (`allocation-counter`) | P01-T09 |
| CA-19 | métrica: `benches/hash_estado.rs`, `benches/sobre.rs`, `benches/rng.rs` corren con `cargo bench -p engine_core`; sin umbral hasta el hito 2 | P01-T09 |
| CA-20 | test `ca_20_version_del_juego_igual_a_cargo` | P01-T02 |
| CA-21 | test `ca_21_lib_rs_lleva_los_atributos` más la puerta adicional: `cargo doc` sin avisos y los dos `cargo tree` comparados con la lista de dependencias permitidas por el orquestador | P01-T09 |
| CA-22 | test `ca_22_tres_clippy_toml_identicos` más la prueba negativa (línea con `std::collections::HashMap` dentro de una función, `cargo clippy` falla con `disallowed_types`), que hace y retira el orquestador en el worktree | P01-T02 |
| CA-23 | tests en `xtask/tests/aceptacion_check_arch_t10.rs` con entradas construidas a mano, y la puerta (`cargo xtask check-arch` en cada integración); la prueba negativa (`rustc-hash.workspace = true` en `engine/sim/Cargo.toml`) la hace y retira el orquestador | P01-T10 |

Las tres tareas de contrato (P01-T01, P01-T03, P01-T04) no cubren criterios: se verifican con la puerta completa en verde, `cargo test -p engine_core --no-run` y la revisión de `revisor-constitucion` de que las firmas son las de la spec.

**Puerta adicional de `engine_core`**, que `/verificar` ejecuta en toda tarea de este plan además de las cinco comprobaciones de `CLAUDE.md`:

```bash
cargo test -p engine_core --release
cargo clippy -p engine_core --features tracy --all-targets -- -D warnings
cargo test -p engine_core --features tracy
RUSTDOCFLAGS="-D warnings" cargo doc -p engine_core --no-deps
cargo tree -p engine_core -e normal --depth 1
cargo tree -p engine_core -e normal
```

En PowerShell, la línea de `cargo doc` es `$env:RUSTDOCFLAGS = "-D warnings"; cargo doc -p engine_core --no-deps; Remove-Item Env:RUSTDOCFLAGS`. Los dos `cargo tree` se comparan con "Dependencias permitidas" de la spec: el primero debe listar exactamente `glam`, `serde`, `postcard`, `ron`, `rustc-hash`, `xxhash-rust`, `bytemuck` y `thiserror`; el segundo no debe contener `heapless`, `wgpu`, `winit`, `egui`, `rayon` ni ningún `game_*`.

**Revisores por tarea:** `revisor-constitucion` siempre; `cazador-determinismo` en todas, porque todas tocan `engine_core`; `revisor-spec` en ninguna, porque no hay mecánicas ni datos de juego.

## Paralelismo

Ninguno. `CLAUDE.md` fija el hito 0 como secuencial y la regla de `/tareas` dice que una tarea que toca `engine_core` o un contrato compartido nunca va en paralelo con otra: aquí todas tocan `engine_core`, y las tres de contrato además el `Cargo.toml` raíz. El orden es el del reparto previsto en la spec: contrato A, su implementación, contratos B1 y B2, y después `hash`, `rng`, `data`, `save` y el cierre, porque `hash` da `DataHash::of_bytes` a `data` y el sobre a `save`, y el cierre cruza módulos.

La única tarea que no toca `engine_core` es la extensión de `check-arch` en `xtask`. Podría correr en paralelo con cualquiera de las anteriores sin pisarla, dentro del tope de dos implementadores, pero no gana nada: es corta, y el hito 0 es secuencial. Se deja la última para que compruebe el workspace tal como queda.

El modo ligero no aplica a ninguna tarea: todas crean o cambian contratos o tocan `engine_core`.

Lo que espera a este plan: el plan de `engine_sim` no empieza hasta que estén integradas las tareas 1 a 8 de aquí, porque su hash de tablas necesita `hash`, su mundo de prueba `rng` y sus formatos `save`. La spec de la simulación sintética se puede escribir mientras tanto. Mientras compila una tarea no se lanzan benchmarks.

## Tareas

Diez tareas en serie, según el "Reparto previsto en tareas" de la spec. Ninguna es `ligera`: todas crean o cambian contratos o tocan `engine_core`, salvo la última, que cambia la comprobación de la constitución y pasa de las 100 líneas. Worktree `.worktrees/p01-tNN`, rama `tarea/p01-tNN-<slug>`. La puerta adicional de `engine_core` (sección "Verificación") corre en todas las tareas que tocan `engine/core`.

### P01-T01 · Contrato A: dependencias, `clippy.toml` y firmas de `time`, `handle`, `version`, `collections` y `math`

- **Tipo:** contrato
- **Crate:** `engine_core`; además el `Cargo.toml` raíz (`[workspace.dependencies]`), `engine/sim/clippy.toml` y `game/rules/clippy.toml`, porque son contrato compartido y ninguna otra tarea debe tocarlos
- **Depende de:** ninguna
- **Contrato que usa:** ninguno; define `time`, `handle`, `version`, `collections`, `math`, la feature `tracy`, las doce entradas de `[workspace.dependencies]` y los tres `clippy.toml`
- **Criterios que cubre:** ninguno directamente. Deja preparados CA-21 (`#![forbid(unsafe_code)]` y `#![deny(missing_docs)]` en `lib.rs`) y CA-22 (los tres `clippy.toml` idénticos), que verifica P01-T02. Se verifica con la puerta completa en verde más `cargo check --workspace --all-targets`, y con `revisor-constitucion` comprobando que las firmas son las de la spec
- **Tests de aceptación:** ninguno, porque es contrato: los cuerpos son `todo!()` y los tests llegan con P01-T02
- **Paralelizable:** no, porque toca `engine_core` y el `Cargo.toml` raíz
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cualquier cuerpo que no sea `todo!()`; los módulos `hash`, `rng`, `data`, `save` y `profile` (contratos B1 y B2); los tests; el contenido de los benchmarks más allá de `fn main() {}`
- **Estado:** pendiente

### P01-T02 · `time`, `handle`, `version`, `collections` y `math`

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T01
- **Contrato que usa:** `time`, `handle`, `version`, `collections`, `math` de P01-T01
- **Criterios que cubre:** CA-1, CA-2, CA-3, CA-4, CA-5, CA-20, CA-22
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t02.rs`. CA-1 y CA-2 se corren también con `--release`. La prueba negativa de CA-22 (línea con `std::collections::HashMap` dentro de una función) la hace y retira el orquestador en el worktree durante `/verificar`
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cambiar una firma del contrato A; `hash`, `rng`, `data`, `save`, `profile`; los benchmarks
- **Estado:** pendiente

### P01-T03 · Contrato B1: firmas de `hash`, `rng` y `data`

- **Tipo:** contrato
- **Crate:** `engine_core`
- **Depende de:** P01-T02
- **Contrato que usa:** `Handle`, `Tick`, `Version` y `math` de P01-T01; define `hash` (incluido el reexport de `bytemuck`), `rng` y `data`. Van juntos porque `hash` necesita `DataHash` de `data` y `Seed` de `rng`
- **Criterios que cubre:** ninguno directamente. Se verifica con la puerta completa en verde, `cargo test -p engine_core --no-run` y `revisor-constitucion` sobre las firmas
- **Tests de aceptación:** ninguno, porque es contrato
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cualquier cuerpo que no sea `todo!()`, salvo las constantes (`DATA_EXTENSION`) y las derivaciones; `save` y `profile`
- **Estado:** pendiente

### P01-T04 · Contrato B2: firmas de `save` y `profile`

- **Tipo:** contrato
- **Crate:** `engine_core`
- **Depende de:** P01-T03
- **Contrato que usa:** `Version` de P01-T01, `DataHash` y `StateHash` de P01-T03; define `save`, `profile`, las macros `span!` y `plot!` y las dos variantes por feature `tracy`
- **Criterios que cubre:** ninguno directamente. Se verifica con la puerta completa en verde, `cargo test -p engine_core --no-run`, `cargo check -p engine_core --features tracy --all-targets` y `revisor-constitucion` sobre las firmas
- **Tests de aceptación:** ninguno, porque es contrato
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cualquier cuerpo que no sea `todo!()`, salvo `MAGIC`, `ENVELOPE_VERSION` y las derivaciones; si `tracy-client` no compila en la máquina del autor, la tarea termina `bloqueada` y lo decide el autor
- **Estado:** pendiente

### P01-T05 · `hash` y `DataHash::of_bytes`

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T04
- **Contrato que usa:** `hash` y `DataHash` de P01-T03; `Handle` y `math` de P01-T01
- **Criterios que cubre:** CA-6, CA-7, CA-8
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t05.rs`, con `xxhash-rust` como dependencia de desarrollo para la referencia de CA-8
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el resto de `data` (`list_data_files`, `hash_data_tree`, `parse_ron`, `load_ron`); `rng`; el benchmark `hash_estado.rs`. Si el diff pasa de 400 líneas, se parte según el riesgo anotado: `StateHasher` con primitivos primero, implementaciones compuestas, vectores de `glam` y `write_handles` después, y la segunda mitad pasa a ser P01-T05b con el mismo archivo de tests
- **Estado:** pendiente

### P01-T06 · `rng`

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T05
- **Contrato que usa:** `rng` de P01-T03; `Tick` y `Handle` de P01-T01
- **Criterios que cubre:** CA-9, CA-10
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t06.rs`. CA-9 se corre también con `--release`
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el benchmark `rng.rs`; cualquier aleatoriedad con estado
- **Estado:** pendiente

### P01-T07 · `data` salvo `of_bytes`

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T06
- **Contrato que usa:** `data` de P01-T03; `StateHasher` y `DataHash::of_bytes` de P01-T05
- **Criterios que cubre:** CA-15, CA-16
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t07.rs`, con carpetas bajo `CARGO_TARGET_TMPDIR`; `ca_15_nombre_no_utf8_es_invalid_path` bajo `#[cfg(windows)]`
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** qué archivos RON existen y sus structs (spec de la simulación sintética); seguir enlaces simbólicos; hot-reload
- **Estado:** pendiente

### P01-T08 · `save`

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T07
- **Contrato que usa:** `save` de P01-T04; `Version` de P01-T01; `DataHash` de P01-T05
- **Criterios que cubre:** CA-11, CA-12, CA-13, CA-14
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t08.rs`, con `postcard` como dependencia de desarrollo para los vectores de CA-11 y `allocation-counter` para CA-14
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el cuerpo del guardado y del replay y sus migraciones (`engine_sim`, `game_rules`); el benchmark `sobre.rs`
- **Estado:** pendiente

### P01-T09 · Cierre: `profile`, benchmarks y criterios transversales

- **Tipo:** implementación
- **Crate:** `engine_core`
- **Depende de:** P01-T08
- **Contrato que usa:** `profile` de P01-T04 y todos los módulos anteriores
- **Criterios que cubre:** CA-17, CA-18, CA-19, CA-21
- **Tests de aceptación:** `engine/core/tests/aceptacion_core_t09.rs`; `ca_17_perfilado_con_feature_no_hace_panico` se corre con `cargo test -p engine_core --features tracy`. CA-19 es métrica: los tres benchmarks corren con `cargo bench -p engine_core`, sin umbral. Los dos `cargo tree` de CA-21 los compara el orquestador con "Dependencias permitidas" en `/verificar`
- **Paralelizable:** no, porque toca `engine_core`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** umbrales de rendimiento (hito 2); una vía en bloque para `Option<Handle>`
- **Estado:** pendiente

### P01-T10 · Extensión de `check-arch` en `xtask`

- **Tipo:** implementación
- **Crate:** `xtask`. El encargo autoriza expresamente tocar `xtask/`
- **Depende de:** P01-T01, porque vigila las dependencias que ese contrato declara. Va la última para comprobar el workspace tal como queda
- **Contrato que usa:** ninguno de `engine_core`; define `xtask` como biblioteca con `direct_dependency_violations` y `cargo_config_violations`
- **Criterios que cubre:** CA-23
- **Tests de aceptación:** `xtask/tests/aceptacion_check_arch_t10.rs`, con entradas construidas a mano. La prueba negativa (`rustc-hash.workspace = true` en `engine/sim/Cargo.toml`) la hace y retira el orquestador en el worktree
- **Paralelizable:** sí, en principio, porque no toca `engine_core`; no se aprovecha, porque el hito 0 es secuencial y la tarea es corta
- **Revisores:** revisor-constitucion
- **Fuera de alcance:** cualquier comprobación nueva que no esté en CA-23; cambiar las comprobaciones existentes de las reglas 1, 2, 3 y 16 más allá de moverlas a funciones puras
- **Estado:** pendiente

## Oleadas

| Oleada | Tareas | Nota |
| --- | --- | --- |
| 1 | P01-T01 | contrato A, va sola; abre las dependencias del workspace |
| 2 | P01-T02 | primera implementación; valida el contrato A antes de seguir |
| 3 | P01-T03 | contrato B1, va solo |
| 4 | P01-T04 | contrato B2, va solo; prueba que `tracy-client` compila |
| 5 | P01-T05 | `hash`; puede partirse en dos si pasa de 400 líneas |
| 6 | P01-T06 | `rng` |
| 7 | P01-T07 | `data` |
| 8 | P01-T08 | `save`; al integrarse, puede empezar `/planificar` de `engine_sim` |
| 9 | P01-T09 | cierre; primera vez que corre entera la puerta adicional, con `--features tracy` y `cargo bench` |
| 10 | P01-T10 | `check-arch`; comprueba el workspace final |

**Camino crítico:** las diez tareas, en serie: P01-T01 → P01-T02 → P01-T03 → P01-T04 → P01-T05 → P01-T06 → P01-T07 → P01-T08 → P01-T09 → P01-T10. P01-T10 podría salir de la cadena, pero no acorta nada mientras solo haya una implementación a la vez. El plan de `engine_sim` se desbloquea al integrarse P01-T08.

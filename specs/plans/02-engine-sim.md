# Plan P02: `engine_sim`

- **Spec:** `specs/architecture/engine_sim.md` (aprobada el 26 de octubre de 2026)
- **Hito:** 0
- **Rama de integración:** main
- **Estado:** en curso

## Enfoque

**Punto de partida.** `engine/sim` es un esqueleto: `src/lib.rs` con la documentación y `#![forbid(unsafe_code)]`, un `Cargo.toml` sin dependencias y el `clippy.toml` que creó el contrato A de `engine_core`. De `engine_core` están integradas las once tareas que dan `Handle`, `Tick`, `hash`, `rng`, `data` y `save`; solo `profile` sigue con cuerpos `todo!("P01-T09")`. `[workspace.dependencies]` no declara `rayon` ni `engine_core`. Todo lo que pide la spec se crea desde cero, y es el primer crate que depende de otro del workspace.

**Contratos primero, en dos tandas.** Las firmas de la spec entran con cuerpos `todo!()` y su documentación en español antes de cualquier implementación, para que `autor-tests` escriba contra ellas y compile con `cargo test -p engine_sim --no-run`. Van en dos tareas de contrato en serie, porque juntas pasan de las 400 líneas y porque la segunda depende de tipos que la primera define: el contrato A con `buffer`, `table`, `parallel`, `events`, `orders` y `grid`, más todo el `Cargo.toml`; el contrato B con `world`, `sim`, `replay`, `save` y `desync`, que usan `OrderQueue`, `Sealed`, `Pool` y `PhaseHash`. Cada contrato deja la puerta completa en verde. `#![deny(missing_docs)]` entra con el contrato A, así que la documentación es parte del contrato. Los `todo!` en `const fn` van como `panic!("P02-TNN: pendiente de implementar")` y los parámetros sin usar con `#![expect(unused_variables, reason = ...)]` de módulo, que la tarea que implementa retira, como en `engine_core`.

**El `Cargo.toml` entero en el contrato A.** `[workspace.dependencies]` gana `rayon = "1.12"` y `engine_core = { path = "engine/core", version = "0.1.0" }`, con versión para que `wildcards = "deny"` de `deny.toml` no lo tome por comodín. `engine/sim/Cargo.toml` declara `engine_core`, `rayon`, `serde` y `thiserror` con `workspace = true`, la feature `tracy = ["engine_core/tracy"]`, las cuatro dependencias de desarrollo (`proptest`, `criterion`, `allocation-counter`, `postcard`) y los cuatro `[[bench]]` con `harness = false` y su archivo en `benches/` con `fn main() {}`, porque un `[[bench]]` sin archivo rompe `cargo metadata` y con él toda la puerta. `check-arch` ya vigila que `engine_sim` no declare `rustc-hash`, `xxhash-rust`, `bytemuck`, `hashbrown`, `ahash`, `foldhash`, `indexmap` ni `rand*` (CA-23 de `engine_core`), y `deny.toml` fija las features de `glam` y `rustc-hash` (CA-24): este crate no puede activar ninguna por unificación.

**`buffer` y `table`.** `FixedVec<T>` es un `Vec<T>` con la capacidad congelada: `with_capacity` y `reserve` son las únicas asignaciones, `push` hace pánico `columna llena` al llegar a la capacidad, y `Clone` es propio para que el clon reserve la capacidad y no la longitud. `serde` lo codifica como secuencia y al deserializar la capacidad queda igual a la longitud. `Column<T>` lo envuelve con índice por `Row`. `Table<C>` lleva el directorio por slot (fila y generación), la pila de slots libres, la columna densa de handles, los borrados pendientes con su bandera por slot y un scratch de permutación; `insert` saca de la pila, `mark_removed` es de tiempo constante, `apply_removals` recorre las filas pendientes de mayor a menor con los dos intercambios que fija la spec, `set_active` y `permute_active` mueven filas por intercambio manteniendo el directorio, y toda operación estructural comprueba `C::len() == len()` o hace pánico `columnas desalineadas`. La deserialización de `Table` es una implementación propia que valida todos los invariantes antes de indexar, para que unos bytes corruptos sean `Corrupt` y nunca un pánico, y después vuelve a reservar la capacidad. Las implementaciones de `HashState` siguen la secuencia de bytes de la spec; la capacidad entra por la longitud del directorio y de la pila.

**`parallel` y `events`.** `Pool` envuelve un `Arc<rayon::ThreadPool>` y guarda el número de hilos, que es lo único que el crate sabe de `rayon`. `par_map` y `par_scan` trocean entrada y salida en chunks de `CHUNK` filas con `chunks` y `chunks_mut` de tamaño fijo, los emparejan con el scratch de cada chunk y los recorren con `rayon` dentro de `Pool::install`, que entra en línea si ya se está dentro del pool; el reparto no depende de los hilos y cada chunk solo escribe en su trozo y en su scratch. `reduce_in_order` recorre el scratch en orden de chunk, secuencial. `SortedEvents::collect` vacía, concatena en orden de chunk, numera la secuencia de emisión y ordena con `sort_unstable_by_key` sobre `(clave, secuencia)`, que no asigna y da el mismo resultado que un orden estable.

**`orders` y `grid`.** `OrderQueue` es un `FixedVec<Sealed<O>>` ordenado por tick de entrada, que comprueba el orden antes que el sitio y cuyo `drain_for` mueve a `out` exactamente las del tick pedido, con pánico `orden del pasado` si queda algo anterior. `Grid` guarda la configuración validada, un contador por celda, los desplazamientos de inicio de celda y la columna de filas ordenada por celda; `rebuild` es un counting sort estable en dos pasadas, `cell_of` recorta al borde y manda los NaN a la celda 0 de su eje, `for_each_in_radius` recorre el rectángulo de celdas en orden `(y, x)` y filas ascendentes, cuenta candidatos y visita los que están dentro del radio al cuadrado, y `cell_order` vuelca la permutación `(celda, fila)` para `permute_active`.

**`world`, `sim` y `desync`.** `TickContext` guarda tick, semilla, pool, las órdenes del tick en un `FixedVec<O>` vaciado por `drain_for`, el buffer de órdenes que las fases envían con `submit`, el contador y el nombre de la fase abierta y el `FixedVec<PhaseHash>` del buscador. `Simulation::step` encola las órdenes del replay si lo hay, vacía la cola para el tick siguiente, entra una sola vez en el pool y llama a `World::step`, encola lo enviado desde dentro sellado para el tick siguiente, avanza el tick y calcula el punto de control cuando `hash_every` divide al tick o hay un hash esperado. `state_hash` escribe tick, semilla, cola pendiente y mundo en ese orden con un `StateHasher`. `first_divergence` es una comparación de dos slices. Las zonas de perfilado (`tick` en `step`, una por fase en el mundo de prueba) usan `engine_core::span!` y `tick_mark`, cuyos cuerpos llegan con P01-T09: por eso estas tareas esperan a esa integración (ver "Riesgos").

**`save` y `replay`.** `SaveBody<W>` y `Replay<S, O>` son structs con `serde` derivado que entran en el sobre de `engine_core::save` con `Kind::Save` o `Kind::Replay`, `format_version(engine, game)` como `(engine << 16) | game` y `GAME_VERSION`; `decode_save` y `decode_replay` pasan por `decode` con la `Expected` correspondiente, así que un hash de datos distinto o una versión distinta de motor o de juego se rechazan con el error del sobre. `save_body` clona el canónico; `load` reconstruye la cola con la capacidad de `SimConfig`, llama a `rebuild_derived`, recalcula el hash y lo compara. El registro de replay son dos `FixedVec` de capacidad `replay_orders` y `replay_checkpoints` que, al llenarse, dejan de grabar y lo avisan una sola vez con `replay_truncated`; `prepare_replay` copia las órdenes y fija los puntos de control como esperados; `expect_checkpoints` exige ticks crecientes y futuros.

**El mundo de prueba.** Vive en `engine/sim/tests/common/mod.rs`, lo escribe `autor-tests` en la primera tarea que lo necesita, la de `world` y `sim`, y los benchmarks lo incluyen con `#[path = "../tests/common/mod.rs"]`. Es la tabla con posición, velocidad, vida y objetivo, el grid y las cinco fases con nombre que fija la spec, más `MundoV2` y `MundoVacio`. Las tareas anteriores usan columnas y mundos sueltos definidos en su propio archivo de tests.

**Cierre.** Los cuatro benchmarks con `criterion` sobre el mundo de prueba, el test de cero asignaciones por tick medido dentro de `Pool::install` con un hilo (CA-18), el test que lee el texto de los doce archivos de `src/` (CA-19) y los de documentación y atributos (CA-21). La puerta adicional del crate corre aquí por primera vez entera, incluida la feature `tracy`.

## Contratos

| Contrato | Crate | Quién lo usa | ¿Compartido? |
| --- | --- | --- | --- |
| `buffer::FixedVec` | `engine_sim` | `table`, `orders`, `events`, `grid`, `game_rules` (buffers de fase), `engine_render` (filas del snapshot, hito 1) | sí (tablas SoA) |
| `table::{Row, Column, Columns, Table, TableFull}` | `engine_sim` | `game_rules` (tablas concretas, implementa `Columns`), `engine_render` (snapshot, hito 1) | sí (tablas SoA con el handle común, ADR-0004) |
| `parallel::{CHUNK, chunk_count, ChunkId, Pool, PoolError, ChunkScratch, par_map, par_scan, Accumulate, reduce_in_order}` | `engine_sim` | `game_rules` (fases), `game_app` (crea el `Pool`) | sí (fases: es el scheduler determinista) |
| `events::{EventKey, SortedEvents}` | `engine_sim` | `game_rules` | sí (eventos) |
| `orders::{Sealed, QueueError, OrderQueue}` | `engine_sim` | `sim`, `game_rules` (tipo `Order` propio), `game_app` (jugador), IA | sí (API de órdenes: el mecanismo, no el tipo) |
| `world::{World, TickContext, PhaseHash}` | `engine_sim` | `game_rules` (implementa `World`), `game_app` | sí (contrato principal con el juego) |
| `sim::{SimConfig, Divergence, TickReport, LoadError, Simulation}` | `engine_sim` | `game_app` (hilo de simulación), herramientas, simulación sintética | sí |
| `replay::{REPLAY_FORMAT_VERSION, Replay, ReplayError, encode_replay, decode_replay}` | `engine_sim` | visor de replays (hito 1), partidas IA contra IA, CI | sí (formato de replay) |
| `save::{SAVE_FORMAT_VERSION, format_version, SaveBody, encode_save, decode_save}` | `engine_sim` | `game_rules` (versiona con `World::FORMAT_VERSION`), `game_app` | sí (formato de guardado: contenedor y composición de versiones) |
| `grid::{MAX_CELLS, GridConfig, CellId, Grid}` | `engine_sim` | `game_rules` (vecinos, combate, separación) | sí (regla 9) |
| `desync::first_divergence` | `engine_sim` | herramienta de comparación (hito 1), tests | no es formato; es el núcleo del buscador |
| Feature `tracy = ["engine_core/tracy"]` | `engine_sim` | `game_app`, mediciones del autor | sí |
| `[workspace.dependencies]`: `rayon`, `engine_core` | `Cargo.toml` raíz | `engine_sim`; después `game_rules` y `game_app` declaran `engine_core` igual | sí (regla 1 del orquestador) |
| Módulos reservados `snapshot`, `thread`, `nav`, `visibility` | `engine_sim` | **no se tocan**: sus specs llegan en los hitos 1, 2 y 3 | sí, pero fuera de este plan |

Todo lo anterior es contrato compartido en el sentido de `CLAUDE.md`: se congela en las dos tareas de contrato, que van solas y se integran antes de cualquier implementación. Los contratos de `engine_core` que este crate consume no cambian; si una tarea necesitara cambiarlos, para y vuelve como pregunta.

## Riesgos

| Riesgo | Cómo se retira |
| --- | --- |
| `engine_core::span!` y `tick_mark` siguen en `todo!("P01-T09")`: cualquier `World::step` que abra una zona hace pánico hasta que P01-T09 se integre, también sin la feature | Las tareas de `buffer`, `table`, `parallel`, `events`, `orders` y `grid` y el contrato B no usan `profile` y pueden ir antes. La tarea de `world` y `sim`, con el mundo de prueba, y las siguientes dependen de P01-T09 integrada; `/tareas` lo escribe en su "Depende de". Si P01-T09 se retrasa por el SDK (Q-012), el mundo de prueba no abre zonas hasta que llegue, y la tarea que las añada es una de doc y una línea por fase. |
| La feature `tracy` no compila en la máquina del autor (Q-012, Windows SDK 10.0.19041) | Igual que en `engine_core`: la puerta común no la usa; las dos líneas de la puerta adicional con `--features tracy` se ejecutan cuando el autor actualice el SDK, y hasta entonces `/verificar` lo anota. |
| `rayon` asigna por dentro con más de un hilo y `allocation-counter` cuenta por hilo | CA-18 mide con un `Pool` de 1 hilo y dentro de `pool.install`, como fija la spec; lo que asigne `rayon` con más hilos se mide en el hito 2. El test deja claro en su nombre que es con un hilo. |
| El reparto de `rayon` en chunks dependiera del número de hilos | `par_map` trocea con `chunks`/`chunks_mut` de tamaño `CHUNK` fijo antes de entrar en `rayon`, así que el chunk `i` siempre recibe las mismas filas; CA-7 lo comprueba con pools de 1, 3, 6 y 12 hilos, y CA-12 compara el hash por tick entre los cuatro. |
| `Pool::install` desde dentro del pool (una fase que llama a otra) bloqueara o se anidara | La doc fija que `install` entra en línea si ya se está dentro, y `rayon` lo garantiza con `ThreadPool::install` sobre el pool actual; CA-7 y CA-11 ejercitan `par_map` dentro de `World::step`, que ya corre dentro de `install`. |
| `sort_unstable` sobre eventos diera un orden distinto según hilos | La secuencia de emisión entra en la clave como último criterio, así que no hay empates y el resultado es el de un orden estable; CA-8 lo compara con 1, 3, 6 y 12 hilos y con todos los eventos de la misma clave. |
| La deserialización propia de `Table` deja pasar unos bytes inconsistentes y el pánico llega después, al indexar | CA-6 prueba siete corrupciones distintas y exige error sin pánico; `revisor-constitucion` revisa que todas las validaciones de "Datos y formatos" están antes de cualquier índice; `cazador-determinismo` prueba otras corrupciones. |
| Las tareas de `table` pasan de 400 líneas (directorio, pila, borrado diferido, conjunto activo, permutación, `serde` propio, `HashState`) | El reparto de la spec ya las parte en dos: hasta `apply_removals` con `clear` y `Clone`, y después `set_active`, `permute_active`, `HashState` y `serde`. Si la primera pasa igualmente, se parte en `buffer` más `Column` y `Columns` por un lado y `Table` por otro, con el mismo archivo de tests. |
| Los vectores de bytes fijados (tabla mínima de 20 bytes, replay de 5 y 9 bytes, `format_version` 65.537) no coinciden | Se calcularon con `postcard` 1.1.3, la versión fijada por `engine_core`. Si no coinciden, se corrige el código o se vuelve a la spec con la discrepancia explicada; nunca se regenera el vector para que pase (ADR-0009). |
| `proptest` con semilla aleatoria hace irreproducibles CA-3 y los tests de propiedades | Semilla fija con `RngSeed::Fixed` y `failure_persistence: None`, como quedó fijado en P01-T05 de `engine_core`; el encargo de `autor-tests` lo exige. |
| Los tests con pools de 12 hilos en una máquina con menos núcleos son lentos, y CA-12 repite cinco veces 300 ticks con 4.000 filas | `rayon` admite más hilos que núcleos; la puerta adicional corre los tests también en `--release`, y los tests pesados (CA-12, CA-13, CA-14, CA-17, CA-18) se diseñan para pasar en menos de un minuto en dev. Si no, `autor-tests` lo reporta y se reduce la escala en la spec. |
| `clippy::pedantic` con `-D warnings` sobre código genérico (`needless_pass_by_value`, `cast_possible_truncation` en `Row(u32)` y `CellId`, `missing_panics_doc` en métodos con pánico fijado) | `expect` puntual con `reason`, nunca `allow` ni a nivel de crate; los pánicos fijados por la spec se documentan con `# Panics` en la doc, que es lo que `missing_panics_doc` pide. `revisor-constitucion` lo revisa en cada tarea. |
| El mundo de prueba de `tests/common/mod.rs` lo escribe `autor-tests` pero lo necesitan los benchmarks del cierre, que escribe el implementador | El hook permite al implementador escribir `benches/`; incluye el módulo con `#[path]` sin tocarlo. Si el mundo de prueba necesita un cambio para los benchmarks, lo hace `autor-tests` en una vuelta de corrección, nunca el implementador. |
| El hook `guarda-ediciones.ps1` bloquea `deny.toml` y `.claude/` a los subagentes | Ninguna tarea de este plan los toca: las features de `glam` y `rustc-hash` ya están fijadas y `engine_sim` no añade features a crates externos. |
| `Simulation<W>` con `W: 'static + Serialize + DeserializeOwned` obliga a `W::Order` y `W::Scenario` a lo mismo y complica el mundo de prueba | Los tres tipos del mundo de prueba derivan `Serialize`, `Deserialize`, `Clone` y `HashState` a mano; `Option<Handle>` ya tiene todo en `engine_core`. Es el coste previsto por la decisión 5 de la spec. |
| El `Grid` con `MAX_CELLS` celdas (2^20) y 15.000 filas en `benches/grid.rs` tarda o asigna | `rebuild` es lineal en filas más celdas por construcción; es informativo hasta el hito 2 (CA-20). Si el tamaño de celda de un mapa concreto no cabe, lo decide la spec del mapa con el benchmark, como fija la decisión 12. |
| `/verificar` no ejecuta hoy la puerta adicional del crate | Cada tarea lleva la puerta adicional en su texto y el orquestador la ejecuta a mano en `/verificar`, como en el plan P01. |
| Un `World::step` de los tests que no sea determinista por error (orden de `HashMap`, tiempo real) se cuela porque CA-12 solo compara hashes de un mundo correcto | `clippy.toml` veta las colecciones con hasher aleatorio también en `tests/`, CA-19 lee el texto de `src/`, y `cazador-determinismo` revisa el mundo de prueba en la tarea que lo crea. |

## Verificación

Tests de aceptación en `engine/sim/tests/aceptacion_sim_tNN.rs`, uno por tarea de implementación; el mundo de prueba en `engine/sim/tests/common/mod.rs` desde la tarea de `world` y `sim`. Los bytes pseudoaleatorios salen de un `xorshift64` de semilla fija y `proptest` va con semilla fija. Todo en memoria: este crate no lee ni escribe archivos. La columna "Tarea" sigue el reparto previsto en la spec y se confirma en `/tareas`.

| Criterio | Cómo se demuestra | Tarea |
| --- | --- | --- |
| CA-1 | tests `ca_01_tabla_insercion_llena_clon_y_clear`, `ca_01_tabla_capacidad_excesiva_panica` y `ca_01_handle_at_fuera_panica` (`allocation-counter` para los ocho `insert`) | P02-T02 |
| CA-2 | test `ca_02_borrado_diferido_descendente_y_generaciones` | P02-T02 |
| CA-22 | tests `ca_22_fixedvec_capacidad_fija` y `ca_22_fixedvec_lleno_y_fuera_de_rango_panican` (con `postcard` para `02 07 08`) | P02-T02 |
| CA-3 | test `ca_03_tabla_invariantes_propiedad` (`proptest` con semilla fija, 2.000 operaciones, capacidades 256 y 0, modelo de referencia) | P02-T03 |
| CA-4 | test `ca_04_conjunto_activo_como_prefijo` | P02-T03 |
| CA-5 | tests `ca_05_permutacion_de_activas`, `ca_05_permutacion_invalida_panica`, `ca_05_swap_fuera_de_rango_panica` y `ca_05_columnas_desalineadas_panica` (`allocation-counter` en la permutación de 4.096 filas) | P02-T03 |
| CA-6 | tests `ca_06_tabla_bytes_fijados`, `ca_06_tabla_serde_ida_y_vuelta_con_capacidad`, `ca_06_tabla_corrupta_no_deserializa` y `ca_06_tabla_hash_segun_la_secuencia` (con `postcard`) | P02-T03 |
| CA-7 | tests `ca_07_chunks_fijos_con_cualquier_pool` (pools de 1, 3, 6 y 12) y `ca_07_par_map_longitudes_y_scratch_panican` | P02-T04 |
| CA-8 | tests `ca_08_eventos_en_orden_de_chunk_y_clave` (pools de 1, 3, 6 y 12; `allocation-counter` tras calentamiento) y `ca_08_eventos_capacidad_panica` | P02-T04 |
| CA-9 | test `ca_09_reduccion_en_orden_de_chunk` | P02-T04 |
| CA-10 | tests `ca_10_cola_de_ordenes_sellada_por_tick` y `ca_10_cola_orden_del_pasado_panica` | P02-T05 |
| CA-16 | tests `ca_16_grid_orden_estable_y_vecinos`, `ca_16_grid_casos_limite` y `ca_16_grid_configuracion_invalida_panica` | P02-T05 |
| CA-17 | test `ca_17_contador_de_vecinos_lineal_con_densidad_constante` (`par_scan` y `reduce_in_order`, pools de 1 y 6) | P02-T05 |
| CA-11 | tests `ca_11_bucle_de_ticks_ordenes_y_puntos_de_control`, `ca_11_demasiadas_fases_panica`, `ca_11_periodo_cero_panica`, `ca_11_ctx_submit_cola_llena_panica` y `ca_11_fase_mal_anidada_panica`, con el mundo de prueba y `MundoVacio` | P02-T07 |
| CA-12 | test `ca_12_mismo_hash_con_1_3_6_12_hilos` (4.000 filas, 300 ticks, cinco repeticiones con 6 hilos; 15.000 filas y 20 ticks con 1 y 12) | P02-T07 |
| CA-15 | test `ca_15_buscador_senala_la_fase_que_diverge` (mundo adrede no determinista) | P02-T07 |
| CA-13 | tests `ca_13_guardar_cargar_y_continuar_con_otro_pool` y `ca_13_load_periodo_cero_panica` (pools de 3, 6 y 1; `MundoV2` para `UnsupportedFormat { expected: 65_538, found: 65_537 }`) | P02-T08 |
| CA-14 | tests `ca_14_replay_reproduce_y_detecta_divergencias`, `ca_14_replay_bytes_fijados_y_limites` (`Replay<(), u8>` contra los vectores de "Datos y formatos") y `ca_14_replay_cola_llena_panica` | P02-T09 |
| CA-18 | test `ca_18_tick_sin_asignaciones` (`Pool` de 1 hilo, `pool.install(|| allocation_counter::measure(|| sim.step()))`, 20 ticks sin órdenes y 20 con una por tick, tras cinco de calentamiento) | P02-T10 |
| CA-19 | test `ca_19_sin_tiempo_real_ni_globales_en_el_codigo` (`include_str!` de los doce archivos de `src/`) | P02-T10 |
| CA-20 | métrica: `benches/tick_sintetico.rs`, `benches/tabla.rs`, `benches/grid.rs` y `benches/guardado.rs` corren con `cargo bench -p engine_sim`; sin umbral hasta el hito 2 | P02-T10 |
| CA-21 | test `ca_21_lib_rs_lleva_los_atributos` más la puerta adicional: `cargo doc` sin avisos, los dos `cargo tree` comparados con la lista de dependencias permitidas por el orquestador, y `cargo xtask check-arch` | P02-T10 |

Las dos tareas de contrato (P02-T01, P02-T06) no cubren criterios: se verifican con la puerta completa en verde, `cargo test -p engine_sim --no-run` y la revisión de `revisor-constitucion` de que las firmas son las de la spec. Como en `engine_core`, los `cargo tree` y las pruebas negativas las hace el orquestador en el worktree y las anota en `/verificar`.

**Puerta adicional de `engine_sim`**, que `/verificar` ejecuta en toda tarea de este plan además de las cinco comprobaciones de `CLAUDE.md`:

```bash
cargo test -p engine_sim --release
cargo clippy -p engine_sim --features tracy --all-targets -- -D warnings
cargo test -p engine_sim --features tracy
RUSTDOCFLAGS="-D warnings" cargo doc -p engine_sim --no-deps
cargo tree -p engine_sim -e normal --depth 1
cargo tree -p engine_sim -e normal
```

En PowerShell, la línea de `cargo doc` es `$env:RUSTDOCFLAGS = "-D warnings"; cargo doc -p engine_sim --no-deps; Remove-Item Env:RUSTDOCFLAGS`. El primer `cargo tree` debe listar exactamente `engine_core`, `rayon`, `serde` y `thiserror`; el segundo no debe contener `wgpu`, `winit`, `egui`, `rand`, `getrandom`, `hashbrown`, `ahash`, `foldhash`, `indexmap` ni ningún `game_*`. Las dos líneas con `--features tracy` dependen de Q-012 (Windows SDK) y de P01-T09 integrada.

**Revisores por tarea:** `revisor-constitucion` siempre; `cazador-determinismo` en todas, porque todas tocan `engine_sim`; `revisor-spec` en ninguna, porque el mundo de prueba no es mecánica de juego y no hay datos RON.

## Paralelismo

Dentro del plan, ninguno: toda tarea toca `engine_sim`, y la regla de `/tareas` dice que una tarea que toca un contrato compartido nunca va en paralelo con otra del mismo crate. El orden es el del reparto previsto en la spec: contrato A, `buffer` y `table` en dos tareas, `parallel` y `events`, `orders` y `grid`, contrato B, `world` y `sim` con el mundo de prueba, `save`, `replay` y el cierre.

Con el plan P01, hay una ventana real. `engine_sim` y `engine_core` son crates distintos y ninguna tarea de este plan toca `engine/core`, así que las tareas de `engine_sim` que no usan `profile` (contrato A, `buffer` y `table`, `parallel` y `events`, `orders` y `grid`, contrato B) pueden ir antes de P01-T09, o a la vez que ella dentro del tope de dos implementadores. El contrato A toca el `Cargo.toml` raíz, así que va solo, como todo contrato. El hito 0 es secuencial por defecto; la ventana se usa si P01-T09 queda a la espera del SDK (Q-012), no por acortar la cadena.

Lo que espera a este plan: `world` y `sim` en adelante necesitan P01-T09 integrada, porque el mundo de prueba abre zonas con `span!` y `Simulation::step` llama a `tick_mark`. La spec de la simulación sintética (`specs/engine/simulacion-sintetica.md`) se puede escribir mientras tanto y es la que lleva la tabla de estado de la constitución a "hecha" (decisión 18). Las specs de `engine_render`, `game_rules` y `game_app` van antes del hito 1. Mientras compila una tarea no se lanzan benchmarks.

El modo ligero no aplica a ninguna tarea: todas crean o cambian contratos o tocan código de simulación con determinismo en juego.

## Tareas

Diez tareas en serie, según el "Reparto previsto en tareas" de la spec. Ninguna es `ligera`: todas crean o cambian contratos o tocan código de simulación con determinismo en juego. Worktree `.worktrees/p02-tNN`, rama `tarea/p02-tNN-<slug>`. La puerta adicional de `engine_sim` (sección "Verificación") corre en todas; sus dos líneas con `--features tracy` esperan a Q-012 y a P01-T09. Las convenciones de contrato son las de P01: cuerpos `todo!("P02-TNN")`, `panic!("P02-TNN: pendiente de implementar")` en `const fn`, `#![expect(unused_variables, reason = ...)]` de módulo que la tarea que implementa retira, y nunca `allow`.

### P02-T01 · Contrato A: dependencias, `Cargo.toml` y firmas de `buffer`, `table`, `parallel`, `events`, `orders` y `grid`

- **Tipo:** contrato
- **Crate:** `engine_sim`, más `[workspace.dependencies]` del `Cargo.toml` raíz
- **Depende de:** P01-T08 (integrada); no necesita P01-T09
- **Contrato que define:** en el `Cargo.toml` raíz, `rayon = "1.12"` y `engine_core = { path = "engine/core", version = "0.1.0" }`; en `engine/sim/Cargo.toml`, las cuatro dependencias normales con `workspace = true`, la feature `tracy = ["engine_core/tracy"]`, las cuatro dependencias de desarrollo y los cuatro `[[bench]]` con `harness = false` (`tick_sintetico`, `tabla`, `grid`, `guardado`) con sus archivos de `fn main() {}`; `src/lib.rs` con `#![deny(missing_docs)]` y los once módulos declarados (los cinco del contrato B como archivos con solo su doc de módulo); y las firmas, derivaciones y docs literales de la spec para `buffer`, `table`, `parallel`, `events`, `orders` y `grid`, con `CHUNK`, `MAX_CELLS` y los `#[error]` con su valor real
- **Criterios que cubre:** ninguno directamente. Deja preparado CA-21 (atributos de `lib.rs`, dependencias directas exactas). Se verifica con la puerta completa en verde, `cargo test -p engine_sim --no-run`, `cargo tree -p engine_sim -e normal --depth 1` igual a `engine_core`, `rayon`, `serde`, `thiserror`, y `revisor-constitucion` sobre las firmas
- **Tests de aceptación:** ninguno, porque es contrato
- **Paralelizable:** no: es contrato y toca el `Cargo.toml` raíz
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cualquier cuerpo que no sea `todo!()`, salvo constantes y derivaciones; las firmas de `world`, `sim`, `replay`, `save` y `desync` (contrato B); contenido de los benchmarks; `specs/`
- **Estado:** pendiente

### P02-T02 · `buffer`, `Column`, `Columns` y `Table` hasta `apply_removals`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T01
- **Contrato que usa:** `buffer` y `table` de P02-T01; `Handle` y `next_generation` de `engine_core`
- **Criterios que cubre:** CA-1, CA-2, CA-22
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t02.rs`, con columnas de prueba definidas en el propio archivo y `postcard` para los bytes de `FixedVec`
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** `set_active`, `permute_active`, `HashState` y `serde` de `Table`, que siguen en `todo!("P02-T03")`; el resto de módulos. Si el diff pasa de 400 líneas, se parte: `buffer`, `Column` y `Columns` por un lado y `Table` por otro, con el mismo archivo de tests, y la segunda mitad pasa a ser P02-T02b
- **Estado:** pendiente

### P02-T03 · `set_active`, `permute_active`, `HashState` y `serde` de `table`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T02
- **Contrato que usa:** `table` de P02-T01 y lo implementado en P02-T02; `StateHasher` y `HashState` de `engine_core`
- **Criterios que cubre:** CA-3, CA-4, CA-5, CA-6
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t03.rs`, con `proptest` de semilla fija para CA-3 y `postcard` para los 20 bytes fijados y las siete corrupciones de CA-6
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el resto de módulos; cualquier cambio de firma. La deserialización propia valida todos los invariantes de "Datos y formatos" antes de indexar
- **Estado:** pendiente

### P02-T04 · `parallel` y `events`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T03
- **Contrato que usa:** `parallel` y `events` de P02-T01; `FixedVec` de P02-T02
- **Criterios que cubre:** CA-7, CA-8, CA-9
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t04.rs`, con pools de 1, 3, 6 y 12 hilos y `allocation-counter` para el `collect` tras calentamiento
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el resto de módulos; cualquier uso de `rayon` fuera de `parallel.rs`; reducciones paralelas
- **Estado:** pendiente

### P02-T05 · `orders` y `grid`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T04
- **Contrato que usa:** `orders` y `grid` de P02-T01; `FixedVec` de P02-T02; `par_scan` y `reduce_in_order` de P02-T04 para CA-17; `Tick` y `Vec2` de `engine_core`
- **Criterios que cubre:** CA-10, CA-16, CA-17
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t05.rs`, con posiciones de `xorshift64` y pools de 1 y 6 hilos para CA-17
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** el resto de módulos; el grid de navegación (hito 2)
- **Estado:** pendiente

### P02-T06 · Contrato B: firmas de `world`, `sim`, `replay`, `save` y `desync`

- **Tipo:** contrato
- **Crate:** `engine_sim`
- **Depende de:** P02-T05; no necesita P01-T09
- **Contrato que define:** las firmas, derivaciones, docs y `#[error]` literales de la spec para `world`, `sim`, `replay`, `save` y `desync`, incluidos `REPLAY_FORMAT_VERSION`, `SAVE_FORMAT_VERSION`, `format_version` (con cuerpo real, porque es una `const fn` de una línea fijada por la spec) y `SimConfig::default()` con los valores de la API; cuerpos `todo!("P02-T07")` en `world`, `sim` y `desync`, `todo!("P02-T08")` en `save`, `save_body` y `load`, y `todo!("P02-T09")` en `replay`, `replay`, `prepare_replay` y `expect_checkpoints`
- **Criterios que cubre:** ninguno directamente. Se verifica con la puerta completa en verde, `cargo test -p engine_sim --no-run` y `revisor-constitucion` sobre las firmas
- **Tests de aceptación:** ninguno, porque es contrato
- **Paralelizable:** no, porque es contrato
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** cualquier cuerpo que no sea `todo!()`, salvo `format_version`, las constantes, las derivaciones y `SimConfig::default()`; `Cargo.toml`; `specs/`
- **Estado:** pendiente

### P02-T07 · `world`, `sim` y `desync`, con el mundo de prueba

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T06 y **P01-T09** (`engine_core::profile`: el mundo de prueba abre zonas con `span!` y `Simulation::step` llama a `tick_mark`)
- **Contrato que usa:** `world`, `sim` y `desync` de P02-T06; todo lo de P02-T02 a P02-T05; `rng`, `hash`, `profile` y `Tick` de `engine_core`
- **Criterios que cubre:** CA-11, CA-12, CA-15
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t07.rs` y el mundo de prueba en `engine/sim/tests/common/mod.rs` (`MundoPrueba`, `MundoV2`, `MundoVacio` y el mundo adrede no determinista de CA-15), que escribe `autor-tests` en esta tarea y reutilizan P02-T08, P02-T09 y P02-T10. `save_body`, `load`, `replay`, `prepare_replay` y `expect_checkpoints` siguen en `todo!`
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** `save` y `replay` y sus cinco métodos de `Simulation`; benchmarks. Si el diff pasa de 400 líneas, se parte: `world` y `desync` primero, `sim` después, con el mismo archivo de tests
- **Estado:** pendiente

### P02-T08 · `save`, `save_body` y `load`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T07
- **Contrato que usa:** `save` de P02-T06 y `Simulation` de P02-T07; el sobre de `engine_core::save` y `GAME_VERSION`
- **Criterios que cubre:** CA-13
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t08.rs`, con el mundo de prueba de `tests/common/mod.rs` y pools de 3, 6 y 1 hilos
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** `replay` y sus tres métodos; migraciones de guardados; el benchmark `guardado.rs`
- **Estado:** pendiente

### P02-T09 · `replay`, `replay`, `prepare_replay` y `expect_checkpoints`

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T08
- **Contrato que usa:** `replay` de P02-T06 y `Simulation` de P02-T07 y P02-T08; el sobre de `engine_core::save`
- **Criterios que cubre:** CA-14
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t09.rs`, con el mundo de prueba, `Replay<(), u8>` para los bytes fijados y pools de 6 hilos para la reproducción
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** replay desde un guardado; el visor de replays (hito 1); vaciado del registro por trozos (spec de hilos, hito 1)
- **Estado:** pendiente

### P02-T10 · Cierre: `tracy`, benchmarks y criterios transversales

- **Tipo:** implementación
- **Crate:** `engine_sim`
- **Depende de:** P02-T09
- **Contrato que usa:** todo el crate; `profile` de `engine_core` con la feature `tracy`
- **Criterios que cubre:** CA-18, CA-19, CA-20, CA-21
- **Tests de aceptación:** `engine/sim/tests/aceptacion_sim_t10.rs`; CA-18 se mide con un `Pool` de 1 hilo dentro de `pool.install`. CA-20 es métrica: los cuatro benchmarks corren con `cargo bench -p engine_sim`, sin umbral, e incluyen el mundo de prueba con `#[path = "../tests/common/mod.rs"]`. Los dos `cargo tree` de CA-21 los compara el orquestador con "Dependencias permitidas" en `/verificar`; el test de CA-19 lee los doce archivos de `src/` con `include_str!`
- **Paralelizable:** no, porque toca `engine_sim`
- **Revisores:** revisor-constitucion, cazador-determinismo
- **Fuera de alcance:** umbrales de rendimiento (hito 2); lo que `rayon` asigne con más de un hilo
- **Estado:** pendiente

## Oleadas

| Oleada | Tareas | Nota |
| --- | --- | --- |
| 1 | P02-T01 | contrato A, va solo; abre `rayon` y `engine_core` en el workspace |
| 2 | P02-T02 | `buffer` y la tabla hasta el borrado diferido; puede partirse en dos |
| 3 | P02-T03 | conjunto activo, permutación, hash y `serde` de la tabla |
| 4 | P02-T04 | chunks y eventos; primera vez que corre `rayon` |
| 5 | P02-T05 | órdenes y grid |
| 6 | P02-T06 | contrato B, va solo |
| 7 | P02-T07 | bucle de simulación y mundo de prueba; **necesita P01-T09 integrada** |
| 8 | P02-T08 | guardado |
| 9 | P02-T09 | replay |
| 10 | P02-T10 | cierre; primera vez que corre entera la puerta adicional, con `--features tracy` y `cargo bench` |

**Camino crítico:** las diez tareas, en serie: P02-T01 → P02-T02 → P02-T03 → P02-T04 → P02-T05 → P02-T06 → P02-T07 → P02-T08 → P02-T09 → P02-T10, con P01-T09 como dependencia externa de P02-T07. Las oleadas 1 a 6 no necesitan P01-T09 y pueden avanzar mientras el autor actualiza el Windows SDK; si P01-T09 se integra antes de llegar a la oleada 7, la cadena no se detiene. Al integrarse P02-T10 termina el hito 0 por parte del motor, y queda la spec de la simulación sintética como escenario de los tests de la constitución.

# `engine_sim`

Simulación determinista a tick fijo: tablas SoA con el handle común (`Table<C>`, `Column<T>`, conjunto activo como prefijo, borrado diferido por fila descendente), fases paralelas por chunks de `CHUNK` filas (`par_map`, `par_scan`, `ChunkScratch`), eventos ordenados sin asignar (`SortedEvents`), cola de órdenes selladas por tick, bucle `Simulation<W>` con hash por tick y por fase, replay, guardado y grid espacial de orden estable. No sabe qué es un soldado ni toca gráficos.

- **Spec:** `specs/architecture/engine_sim.md`. Sus firmas, derivaciones y dependencias solo cambian en una tarea de tipo `contrato`, que va sola. Los módulos `snapshot`, `thread`, `nav` y `visibility` están reservados hasta su spec.
- **Reglas que le afectan:** 2 (sin gráficos), 3 (`#![forbid(unsafe_code)]`), 4 (`step` es un tick), 5 (SoA: una `Column<T>` por campo), 6 (todo lo que influye está en `World`, la cola y el tick; lo derivado se reconstruye en `rebuild_derived`), 7 (ningún `static`), 9 (vecinos solo por `Grid`, con contador), 13 (chunks fijos, eventos en orden de chunk, reducciones secuenciales, aleatoriedad de `engine_core::rng`), 17 (`Tick`/`Ticks` nada más), 19 (capacidades fijas: `TableFull`, `columna llena`, `eventos llenos`, nunca crecer).
- **Dependencias:** `engine_core`, `rayon`, `serde`, `thiserror`, con `nombre.workspace = true`. `glam` y `bytemuck` solo por los reexports de `engine_core`; `postcard` solo como dependencia de desarrollo; `rustc-hash`, `xxhash-rust` y `ron` no se usan.
- **Comandos:** `cargo check -p engine_sim --all-targets`, `cargo test -p engine_sim`, `cargo bench -p engine_sim`. Puerta adicional, además de la de `CLAUDE.md` raíz: `cargo test -p engine_sim --release`, `cargo clippy -p engine_sim --features tracy --all-targets -- -D warnings`, `cargo test -p engine_sim --features tracy`, `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_sim --no-deps` y `cargo tree -p engine_sim -e normal` contra la lista de dependencias permitidas.
- **Tests de aceptación:** `tests/aceptacion_sim_tNN.rs`, uno por tarea; los escribe `autor-tests` y no se tocan. Los tests de determinismo corren con pools de 1, 3, 6 y 12 hilos.
- **`clippy.toml`:** el de esta carpeta prohíbe `std::collections::HashMap`, `HashSet` y `RandomState` (regla 13, ADR-0009).

Nunca:

- `HashMap` o `HashSet` de la biblioteca estándar, `par_bridge`, `current_num_threads` en la lógica, reducciones paralelas (`sum`, `reduce`, `fold`, `min_by`, `max_by` sobre `par_iter`), `thread_rng`, `Instant`, `SystemTime`, `static`.
- Un `Vec` que crezca dentro de `step`; todo buffer es `FixedVec`, `Column`, `ChunkScratch` o `SortedEvents` con capacidad fijada al crear.
- Borrar filas a mitad de fase: solo `mark_removed` y `apply_removals` al final del tick.
- Grabar en el replay las órdenes de `ctx.submit`: la IA se regenera al reproducir porque es determinista; el replay solo lleva las de `Simulation::submit`.
- Decidir nada de juego: tablas, fases, órdenes, eventos y escenarios concretos son de `game_rules`; esta crate es genérica sobre `World`.

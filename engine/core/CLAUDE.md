# `engine_core`

Base compartida del motor: tiempo en ticks (`Tick`, 20 Hz), handle común a todas las tablas, hashers deterministas, aleatoriedad sin estado, sobre binario de guardados y replays, carga de RON con hash de datos, perfilado con Tracy y reexport de `glam`. No conoce entidades, tablas, gráficos ni juego.

- **Spec:** `specs/architecture/engine_core.md`. Es el contrato de todos los demás crates: sus firmas, derivaciones y dependencias solo cambian en una tarea de tipo `contrato`, que va sola y se integra antes de abrir paralelismo.
- **Reglas que le afectan:** 3 (`#![forbid(unsafe_code)]`), 4 y 17 (`Tick` sobre `u64`, sin flotantes de tiempo), 6 (tipos canónicos con `serde` y `postcard` estable), 7 (nada global mutable), 13 (hashers fijos del ADR-0009, hash de estado por bytes en orden fijo, `hash(semilla, tick, handle, propósito)`), 18 (hash de datos en el sobre), 19 (las primitivas no asignan).
- **Dependencias:** solo las de la sección "Dependencias permitidas" de la spec, con `nombre.workspace = true`. Ningún crate del workspace. Si falta una, para y pregunta.
- **Comandos:** `cargo check -p engine_core --all-targets`, `cargo test -p engine_core`, `cargo bench -p engine_core`. Puerta adicional del crate, además de la de `CLAUDE.md` raíz: `cargo test -p engine_core --release`, `cargo clippy -p engine_core --features tracy --all-targets -- -D warnings`, `cargo test -p engine_core --features tracy`, `RUSTDOCFLAGS="-D warnings" cargo doc -p engine_core --no-deps` y `cargo tree -p engine_core -e normal` contra la lista de dependencias permitidas.
- **`clippy.toml`:** el de esta carpeta prohíbe `std::collections::HashMap`, `HashSet` y `RandomState` (regla 13, ADR-0009); hay copias iguales en `engine/sim/` y `game/rules/`.
- **Tests de aceptación:** `tests/aceptacion_core_tNN.rs`, uno por tarea; los escribe `autor-tests` y no se tocan.

Nunca:

- `std::collections::HashMap` ni `HashSet` con `RandomState`, ni en la API ni dentro; `Instant`, `SystemTime`, hilos ni `rayon`.
- Tipos de `rustc-hash`, `xxhash-rust`, `postcard` o `ron` en firmas públicas, salvo los alias de `collections`.
- Leer o escribir archivos fuera de `data`, ni vigilar archivos; el hot-reload es de la app.
- Asignar en `time`, `handle`, `hash`, `rng`, `profile` ni en las consultas de `collections`.
- Números o reglas de juego, ids de `Purpose` o hashes dorados: eso es de las specs de features.

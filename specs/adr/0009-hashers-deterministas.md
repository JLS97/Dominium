# ADR-0009: Hashers deterministas: `rustc-hash` para colecciones y xxh3 para estado, datos y aleatoriedad

- **Estado:** aceptado
- **Fecha:** 2026-10-26
- **Hito afectado:** 0 en adelante

## Contexto

La regla 13 de la constitución exige el mismo resultado con cualquier número de hilos y prohíbe `HashMap` y `HashSet` con el hasher por defecto dentro de la simulación, porque el SipHash de la biblioteca estándar se siembra al azar en cada proceso. Hacen falta dos cosas distintas:

1. Un hasher fijo para las colecciones con hash de `engine_core`, `engine_sim` y `game_rules`.
2. Una función de hash sobre bytes para el hash de estado por tick y por sistema (reglas 6 y 13, buscador de desincronizaciones), para el hash de los datos con los que se juega (regla 18) y para la aleatoriedad sin estado `hash(semilla, tick, handle, propósito)`.

El GDD v2.1 decía "un hasher determinista" sin nombrarlo, y la spec de `engine_core` no puede cerrar sus dependencias sin esa decisión. Restricciones: determinismo solo en la misma máquina; cero asignaciones en el camino caliente (regla 19); licencias permitidas por `deny.toml`; crates pequeños y mantenidos, porque el código lo generan agentes y cada dependencia es superficie de error.

## Opciones consideradas

1. **`rustc-hash` para colecciones y `xxhash-rust` (xxh3) para bytes.** A favor: dos crates minúsculos y muy usados, sin semilla aleatoria; xxh3 está entre los más rápidos para entradas cortas y largas y tiene versión en streaming. En contra: dos dependencias en vez de una; FxHash no resiste HashDoS, lo que no importa en un juego sin red.
2. **`foldhash` con estado fijo para todo.** A favor: un solo crate, muy rápido. En contra: su API por defecto es sembrada y hay que acordarse de usar el estado fijo en cada uso; poco habitual como hash de contenido y sin streaming de serie.
3. **`ahash` con semillas fijas.** A favor: rápido. En contra: su `RandomState` es aleatorio por defecto, justo lo que la regla 13 prohíbe; su salida depende de las instrucciones del procesador y puede cambiar entre versiones del crate.
4. **`blake3` para el hash de estado.** A favor: criptográfico, colisiones imposibles en la práctica. En contra: varias veces más lento que xxh3 en entradas cortas, y la detección de desincronizaciones no necesita resistir a un adversario.

## Decisión

La opción 1. `rustc-hash` (`FxHashMap`, `FxHashSet`, `FxBuildHasher`) para toda colección con hash en `engine_core`, `engine_sim` y `game_rules`. `xxhash-rust` con la feature `xxh3` para el hash de estado por tick y por sistema (`Xxh3` en streaming sobre las columnas SoA en orden fijo, 64 bits), para el hash de datos de guardados y replays, y para la aleatoriedad sin estado: `xxh3_64` sobre los 32 bytes de `(semilla, tick, handle, propósito)` en little-endian. Las versiones se fijan en `[workspace.dependencies]` en la primera tarea de contrato del hito 0, y la spec de `engine_core` expone tipos envoltorio para que el resto del código no nombre los crates.

## Consecuencias

- Más fácil: hashes reproducibles entre ejecuciones e hilos, y un solo lugar, `engine_core`, donde cambiar de algoritmo.
- Más difícil y a vigilar: iterar un `FxHashMap` es determinista para la misma secuencia de operaciones, pero su orden es arbitrario y puede cambiar al subir `hashbrown` o `rustc-hash`. Sigue prohibido que el orden de iteración de un mapa influya en el resultado; lo revisa `cazador-determinismo`. `rustc-hash` 2.x cambió el algoritmo respecto a 1.x: si se sube de versión mayor, los hashes dorados se regeneran y se anota en el GDD. Los hashes de estado dorados valen solo para la misma versión de `xxhash-rust` y la misma plataforma.
- Comprobación: el test de la regla 13 (pools de 1, 3, 6 y 12 hilos, mismo hash por tick) y una búsqueda automática de `std::collections::HashMap`, `HashSet` y `RandomState` en `engine_sim` y `game_rules`; el mecanismo exacto lo fija la spec de la simulación sintética.
- Licencias: `rustc-hash` es MIT o Apache-2.0; `xxhash-rust` es BSL-1.0, ya permitida en `deny.toml`.
- Se revisa si una medición del hito 2 muestra el hash de estado por encima de su presupuesto (unos 3 MB en 0,3 ms) o si aparece una colisión real en el buscador de desincronizaciones.

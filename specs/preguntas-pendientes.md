# Preguntas pendientes para el autor

Aquí se recogen las dudas que surgen al escribir specs, planes o código y que ningún agente puede responder. Solo escribe aquí la sesión principal; los subagentes devuelven sus preguntas en su informe. Las preguntas abiertas del GDD siguen en su sección final, y aquí van las nuevas.

Cada pregunta es una casilla sin marcar en la sección "Pendientes", con este formato: un id `Q-NNN`, la fecha, el archivo que la origina, la pregunta, una línea de contexto y las opciones si las hay. Debajo, una línea "Respuesta:" que rellena el autor.

Cuando el autor responde, la sesión principal lleva la respuesta a la spec, al GDD o a un ADR, marca la casilla, anota dónde quedó y la mueve a "Respondidas". Las ya incorporadas se pueden borrar.

## Pendientes

Ninguna por ahora.

## Respondidas

- [x] **Q-001** · 2026-10-25 · `specs/gdd/plus-ultra-dominium-contexto-gdd.md:339` · ¿Dónde corre la CI del hito 0 y con qué runner? Contexto: el GDD exige una CI con `check-arch`, clippy, tests y `cargo deny` como entregable del hito 0, pero el repositorio no tiene remoto ni carpeta de CI, y el determinismo solo se garantiza en la misma máquina (regla 13), así que un runner Linux no puede comparar hashes dorados calculados en Windows. Depende de la pregunta 11 del GDD (plataforma). Opciones: (a) GitHub Actions con runner Windows; (b) runner Linux que solo compara ejecuciones dentro del mismo trabajo; (c) sin CI remota en el hito 0, con la puerta local antes de cada integración.
  Respuesta: (c) ahora y (a) en cuanto exista el remoto en GitHub, con Windows como única plataforma de la fase A (pregunta 11 del GDD). Respondida el 2026-10-26. Incorporada en: GDD (hito 0, aceptación, preguntas respondidas y cambios v2.2).
- [x] **Q-002** · 2026-10-25 · `specs/gdd/plus-ultra-dominium-contexto-gdd.md:283` · ¿Qué hasher determinista usamos? Contexto: la regla 13 exige un hasher fijo y el stack del GDD dice "un hasher determinista" sin nombrarlo; la spec de `engine_core` no puede cerrar sus dependencias ni el tipo del hash de estado por tick sin esa decisión. `/spec arquitectura engine_core` presentará las opciones con pros y contras (una para mapas con claves fijas y otra para el hash de estado) y la decisión irá a un ADR.
  Respuesta: (a) `rustc-hash` para colecciones y `xxhash-rust` con xxh3 para el hash de estado, el hash de datos y la aleatoriedad sin estado. Respondida el 2026-10-26. Incorporada en: ADR-0009 y GDD (stack y determinismo en la práctica).
- [x] **Q-003** · 2026-10-25 · `specs/constitution.md:97` · ¿El asignador contador de la regla 19 viene de un crate externo o se escribe propio con una excepción a la regla 3? Contexto: implementar `GlobalAlloc` exige `unsafe impl`, y la regla 3 junto con `unsafe_code = "deny"` del workspace lo prohíben también en tests y benches. Opciones: (a) crate externo, elegido en la spec de simulación sintética como dependencia de desarrollo; (b) asignador propio en un archivo con excepción registrada en un ADR.
  Respuesta: (a) crate externo como dependencia de desarrollo; candidato `allocation-counter`, que se verifica y fija en la spec de la simulación sintética. Respondida el 2026-10-26. Incorporada en: GDD (asignaciones).
- [x] **Q-004** · 2026-10-25 · `specs/constitution.md:40` · ¿Dónde viven los datos RON del hito 0 y qué contienen? Contexto: las reglas 11 y 18 se comprueban en el hito 0 (carga de todos los datos y rechazo por hash de datos), pero no existe ningún RON y `assets/README.md` declara la carpeta vacía hasta el hito 5. Opciones: (a) `assets/data/` desde ya, con los parámetros del escenario sintético; (b) otra carpeta, por ejemplo `data/`, separada de los assets de arte; (c) aplazar 11 y 18 al hito 1 y cambiar la tabla de la constitución.
  Respuesta: (b) carpeta `data/` en la raíz, separada de `assets/`. Respondida el 2026-10-26. Incorporada en: `data/README.md`, GDD (árbol y contenido data-driven) y README raíz.
- [x] **Q-005** · 2026-10-25 · `specs/architecture/README.md:3` · ¿Las specs de arquitectura de `engine_tools` y `game_tools` entran en el hito 0? Contexto: el README de arquitectura pide siete specs, una por crate, y el "Siguiente paso" del GDD y la guía del autor solo prevén cinco (core, sim, render, rules y app). Opciones: (a) cinco ahora y las dos de herramientas cuando un hito las necesite; (b) las siete en el hito 0.
  Respuesta: (b) `engine_core` y `engine_sim` antes del hito 0; `engine_render`, `game_rules` y `game_app` antes del hito 1; `engine_tools` y `game_tools` cuando un hito pida su primer binario. Respondida el 2026-10-26. Incorporada en: GDD (hito 0 y siguiente paso), `specs/architecture/README.md`, `.claude/README.md` y `CLAUDE.md`.

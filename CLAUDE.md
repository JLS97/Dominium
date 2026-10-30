# Plus Ultra: Dominium — contexto para agentes

RTS en Rust con motor propio, inspirado en Imperivm. Un solo desarrollador; todo el código lo generan agentes siguiendo Spec Driven Development. Idioma del proyecto: español en comentarios, specs y mensajes; inglés en identificadores de código.

## Lee antes de tocar nada

1. `specs/constitution.md`: reglas inviolables, con su motivo y su comprobación. Manda sobre la tabla resumen del GDD.
2. `specs/gdd/plus-ultra-dominium-contexto-gdd.md`: visión, mecánicas, arquitectura, plan de hitos y preguntas abiertas.
3. La spec del crate o de la feature que vayas a tocar, en `specs/architecture/`, `specs/engine/` o `specs/game/`.
4. Los ADR de `specs/adr/`, para no reabrir decisiones cerradas.

## Reglas que ningún agente rompe

- No inventes mecánicas de los Imperivm originales. Si falta un detalle, pregunta al autor y márcalo como pendiente.
- Las preguntas abiertas del GDD las responde el autor. No las des por respondidas.
- Las dependencias van de `game/` a `engine/`, nunca al revés. `engine_sim` y `game_rules` no conocen gráficos. `unsafe` solo en `engine_render`, siempre con comentario `SAFETY:`.
- La simulación es determinista con cualquier número de hilos, corre a tick fijo, mide el tiempo en ticks enteros y guarda los datos en SoA. Dentro de `sim` y `rules`: nada de `HashMap` ni `HashSet` con el hasher por defecto, nada de reducciones paralelas de flotantes, nada de asignaciones en el camino caliente.
- Contenido por datos en RON: unidades, facciones, habilidades, plantillas de ciudad y perfiles de IA. El guardado es binario y versionado.
- Ninguna feature sin spec con criterios de aceptación medibles.

## Mapa de crates

| Crate | Carpeta | Responsabilidad |
| --- | --- | --- |
| `engine_core` | `engine/core` | matemáticas, tiempo en ticks, handles, serialización, perfilado |
| `engine_sim` | `engine/sim` | tablas SoA, grid espacial, navegación, scheduler determinista, eventos |
| `engine_render` | `engine/render` | wgpu, instancing, skinning en GPU, terreno, culling y LOD en GPU |
| `engine_tools` | `engine/tools` | inspector de assets, horneado de impostores; sin conceptos de juego |
| `game_rules` | `game/rules` | unidades, facciones, economía, habilidades, IA; sin gráficos |
| `game_app` | `game/app` | ventana, cámara, entrada, HUD, pantallas |
| `game_tools` | `game/tools` | editor de mapas, visor de partidas IA contra IA |
| `xtask` | `xtask` | comprobaciones de arquitectura: `cargo xtask check-arch` |

## Antes de dar algo por terminado

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-arch
cargo deny check
```

## Flujo de trabajo

Spec, plan, tareas, implementación y verificación, en ese orden, con un comando por paso: `/spec`, `/planificar`, `/tareas`, `/implementar`, `/verificar` e `/integrar`. `/estado` resume dónde estamos. Plantillas: `specs/adr/0000-plantilla.md`, `specs/architecture/plantilla-crate.md`, `specs/game/plantilla-feature.md` y `specs/plans/plantilla-plan.md`. Cada decisión técnica cerrada tiene su ADR; cada crate tendrá su propio `CLAUDE.md` cuando exista su spec de arquitectura.

## Flujo multiagente

Esta sección es para la sesión principal. Si eres un subagente, sigue tu definición en `.claude/agents/` y el encargo que te han dado.

La sesión principal es el **orquestador**, y el autor decide con ella. Convierte specs en planes y tareas, delega, revisa e integra. Es la única que escribe en `specs/` y en la rama de integración. Los contratos compartidos los aprueba ella y los escribe un `implementador` en una tarea de tipo `contrato`. La guía para el autor está en `.claude/README.md`.

| Subagente | Para qué | Cuándo |
| --- | --- | --- |
| `explorador` | buscar y resumir sin llenar el contexto del orquestador | siempre que haya que leer mucho |
| `autor-tests` | tests de aceptación desde la spec, antes de implementar | `/implementar` |
| `implementador` | una tarea, un crate, un worktree | `/implementar` y correcciones de `/verificar` |
| `revisor-constitucion` | el diff contra la constitución y la spec, sin ver el razonamiento del implementador | `/verificar`, siempre |
| `cazador-determinismo` | no determinismo y estado no canónico, de forma adversarial | `/verificar`, si toca core, sim o rules |
| `revisor-spec` | calidad de la spec (criterios medibles, casos límite, contratos) y fidelidad (mecánicas inventadas, preguntas abiertas dadas por respondidas) | siempre en `/spec`; en `/verificar` con mecánicas o RON |
| `investigador-rendimiento` | presupuestos y best-of-N | desde el hito 2 |
| `analista-partidas` | partidas IA contra IA y ajustes en RON | desde el hito 4 |

Reglas del orquestador:

1. **Contratos primero y en serie.** El handle, `Tick`, las fases y eventos, la API de órdenes, el layout del snapshot, los formatos de replay y guardado y las dependencias de `[workspace.dependencies]` los cambia una sola tarea cada vez. Esas tareas se integran antes de abrir paralelismo.
2. **Un worktree por tarea.** Se crea con `git worktree add .worktrees/<pnn-tnn> -b tarea/<pnn-tnn>-<slug> <rama>`. Los worktrees solo ven commits, así que spec, plan y contrato se commitean antes de lanzar a nadie. Ningún subagente edita el checkout principal; lo bloquea un hook. En Bash, todo comando que deba correr en un worktree empieza por `cd "<worktree>" && ...`, porque el directorio vuelve a la raíz después de cada comando.
3. **Como mucho dos implementadores a la vez.** Compilar es el cuello de botella, porque cada worktree tiene su `target/`. Revisores y exploradores sí pueden ir en paralelo. No se lanzan benchmarks ni partidas IA contra IA mientras algo compila.
4. **Revisión sin anclaje.** Los revisores reciben tarea, spec, rama y worktree; nunca el informe del implementador ni la opinión del orquestador. Antes de presentar un hallazgo bloqueante, el orquestador lo confirma leyendo las líneas citadas.
5. **Integración en serie.** Una rama cada vez, actualizada con la rama de integración y con la puerta completa en verde antes del merge. Si un merge cambia un hash o un replay dorado sin que la spec lo justifique, no entra.
6. **Preguntas, no inventos.** Las dudas de diseño de los subagentes vuelven en su informe. El orquestador las apunta en `specs/preguntas-pendientes.md` y se las presenta agrupadas al autor. Nadie las responde por él.
7. **Encargos completos.** Un subagente solo sabe lo que pone su encargo: tarea, rutas de spec y plan, worktree y rama, criterios, contrato y lo que queda fuera de alcance. Las plantillas de encargo están en `/implementar` y `/verificar`.
8. **Commits en la rama de integración solo cuando el autor los pide,** o cuando un comando los prevé tras su aprobación. Los subagentes commitean en sus worktrees sin preguntar; la sesión principal no commitea en `main` por iniciativa propia.

Paralelismo por hito:

- **Hito 0:** secuencial, y en modo ligero (ver `/implementar`) para las tareas sin contrato nuevo; el flujo completo queda para los contratos y el código de simulación. Solo el spike de `wgpu` del hito 1 puede adelantarse en paralelo, porque es desechable.
- **Hito 1:** dos líneas, `engine_sim` y `engine_render`, después de congelar el snapshot y la API de órdenes. `game_app` las integra al final.
- **Hito 2:** best-of-N en los puntos calientes (flow fields, reordenación por celda, culling). Decide `investigador-rendimiento` con el benchmark, no un agente juez.
- **Hito 3:** casi secuencial. Lo valida el autor jugando.
- **Hito 4:** lotes de partidas sin gráficos con `analista-partidas`. Los ajustes van en RON.
- **Hito 6:** una facción por tarea, con Cartago sola primero, y solo con habilidades ya verificadas.

## Estado

Hito 0 en preparación, sin código todavía. Toolchain fijado; las cinco comprobaciones pasan sobre el esqueleto vacío. Constitución y ADR-0001 a ADR-0009 redactados. Specs de arquitectura de `engine_core` y `engine_sim` aprobadas el 26 de octubre de 2026, con su `CLAUDE.md` en cada crate; las de render, rules y app van antes del hito 1. Plan P01 de `engine_core` (`specs/plans/01-engine-core.md`) en borrador: el siguiente paso es `/tareas`, y después implementación y verificación en serie; detrás van el plan de `engine_sim`, que espera a las ocho primeras tareas de core, y la spec de la simulación sintética. Pendientes para el autor: ninguna pregunta en la bandeja; la línea en `/verificar` para ejecutar la puerta adicional de cada crate. El repositorio no tiene remoto: los commits son locales y el autor sincroniza. Flujo multiagente configurado en `.claude/`: subagentes, comandos como skills y hooks.

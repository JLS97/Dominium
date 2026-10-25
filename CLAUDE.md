# Plus Ultra: Dominium — contexto para agentes

RTS en Rust con motor propio, inspirado en Imperivm. Un solo desarrollador; todo el código lo generan agentes siguiendo Spec Driven Development. Idioma del proyecto: español en comentarios, specs y mensajes; inglés en identificadores de código.

## Lee antes de tocar nada

1. `specs/constitution.md`: reglas inviolables. Mientras no esté redactada, manda la tabla "Constitución" del GDD.
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

Spec, plan, tareas, implementación y verificación, en ese orden. Plantillas: `specs/adr/0000-plantilla.md`, `specs/architecture/plantilla-crate.md` y `specs/game/plantilla-feature.md`. Cada decisión técnica cerrada tiene su ADR; cada crate tendrá su propio `CLAUDE.md` cuando exista su spec de arquitectura.

## Estado

Hito 0 en preparación. Toolchain fijado en `rust-toolchain.toml`; las cinco comprobaciones pasan sobre el esqueleto vacío. Sin constitución redactada ni ADR todavía.

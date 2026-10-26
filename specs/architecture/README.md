# Specs de arquitectura

Una spec por crate, con el nombre del crate: `engine_core.md`, `engine_sim.md`, `engine_render.md`, `engine_tools.md`, `game_rules.md`, `game_app.md`, `game_tools.md`. Plantilla en `plantilla-crate.md`.

Cada spec dice qué hace el crate, qué no hace nunca, qué API pública expone, de qué puede depender y qué pruebas lo protegen. Cuando exista la spec de un crate, ese crate recibe su propio `CLAUDE.md` que apunta a ella.

Orden, por decisión del autor del 26 de octubre de 2026 (Q-005): `engine_core` y `engine_sim` antes del hito 0; `engine_render`, `game_rules` y `game_app` antes del hito 1; `engine_tools` y `game_tools` cuando un hito pida su primer binario.

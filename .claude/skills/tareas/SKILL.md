---
name: tareas
description: Divide un plan aprobado en tareas pequeñas con crate, dependencias, contrato, revisores y oleadas
argument-hint: <ruta del plan>
disable-model-invocation: true
---

Divide en tareas el plan `$ARGUMENTS` y escribe su sección "Tareas" con el formato de `specs/plans/plantilla-plan.md`. Eres el orquestador.

## Reglas

- **Contrato primero.** Si la feature tiene API nueva, la primera tarea es de tipo `contrato`: tipos y firmas públicas con cuerpos `todo!()`, que compilan y pasan `check-arch`. Va sola y se integra antes que cualquier otra, porque `autor-tests` y los implementadores trabajan contra ella. También añade a `[workspace.dependencies]` lo que vaya a necesitar la feature, incluidos `proptest` y `criterion` si hay tests de propiedades o benchmarks, para que ninguna otra tarea toque el `Cargo.toml` raíz.
- **Tamaño.** Una tarea toca un crate (salvo un contrato que justifique más), se verifica sola y su diff previsible no pasa de unas 400 líneas. Si es más grande, pártela.
- **Tareas ligeras.** Si una tarea no crea ni cambia contratos, no toca `engine_core` ni código de simulación con determinismo en juego, y su diff previsible no pasa de unas 100 líneas, márcala de tipo `ligera`: la hace la sesión principal sin worktree ni `autor-tests`, por el modo ligero de `/implementar`. Es la norma en el hito 0 y la excepción después.
- **Contratos compartidos.** Una tarea que toca `engine_core` o un contrato compartido nunca va en paralelo con otra.
- **Cobertura.** Cada tarea lista los criterios (`CA-n`) que cubre, y todo criterio de la spec tiene al menos una tarea.
- **Tests de aceptación.** Toda tarea de implementación con criterios automatizables tiene su archivo `<crate>/tests/aceptacion_<feature>_tNN.rs`, uno por tarea para que dos tareas no choquen al integrarse. Si no lo tiene, di por qué.
- **Revisores.**
  - `revisor-constitucion`: siempre.
  - `cazador-determinismo`: si toca `engine_core`, `engine_sim` o `game_rules`.
  - `revisor-spec`: si toca mecánicas o datos de juego.
  - `investigador-rendimiento`: si la tarea tiene presupuestos.
- **Ids.** `P<NN del plan>-T<NN>`, por ejemplo `P01-T03`. El worktree será `.worktrees/p01-t03` y la rama `tarea/p01-t03-<slug corto>`.
- **Estados.** En el plan solo se escriben `pendiente` e `integrada (AAAA-MM-DD)`. Lo que está en curso se ve en `git worktree list` y en las ramas `tarea/*`.

## Al final de la sección

- **Oleadas:** qué tareas pueden ir juntas, con un máximo de dos implementaciones a la vez.
- **Camino crítico:** la cadena de dependencias más larga.

Presenta las tareas al autor. Con su aprobación, pon `Estado: aprobado` en la cabecera del plan y haz commit con el mensaje `tareas: <slug>`.

Siguiente paso: `/implementar <ruta del plan>`.

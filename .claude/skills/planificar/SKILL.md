---
name: planificar
description: Convierte una spec aprobada en un plan técnico con contratos, riesgos, verificación y paralelismo
argument-hint: <ruta de la spec>
disable-model-invocation: true
---

Planifica la spec `$ARGUMENTS`. Eres el orquestador.

1. **Lee la spec.** Si no tiene `Estado: aprobada`, para: solo se planifican specs aprobadas. Si tiene preguntas abiertas que afectan a requisitos o criterios, para y lístalas: no se planifica sobre huecos.
2. **Mira qué existe ya en el código.** Usa `explorador` para localizar tipos, módulos y tests relacionados.
3. **Crea el plan** en `specs/plans/NN-<slug>.md`, con el siguiente número libre, a partir de `specs/plans/plantilla-plan.md`. Rellena todo menos las tareas:
   - **Enfoque:** cómo se resuelve, en un párrafo por pieza.
   - **Contratos:** los tipos, funciones y formatos públicos que se crean o cambian, en qué crate y quién depende de ellos. Los compartidos (`engine_core`, handle, `Tick`, fases y eventos, API de órdenes, snapshot, replay y guardado) se congelan antes de abrir paralelismo.
   - **Riesgos:** cada uno con la forma de retirarlo.
   - **Verificación:** cada criterio de la spec con el test, replay, contador, benchmark o revisión que lo demuestra.
   - **Paralelismo:** qué puede ir en paralelo y qué no, y por qué, según la guía por hitos de `CLAUDE.md`.
4. **Presenta** al autor el plan resumido. Con su aprobación, deja `Estado: borrador` (pasa a `aprobado` cuando se aprueben las tareas) y haz commit con el mensaje `plan: <slug>`.

Siguiente paso: `/tareas <ruta del plan>`.

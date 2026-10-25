---
name: integrar
description: Integra en serie una rama de tarea ya verificada en la rama de integración, con la puerta completa en verde
argument-hint: <P01-T03 o nombre de rama>
disable-model-invocation: true
---

Integra la tarea o rama `$ARGUMENTS`. Eres el orquestador. Una sola rama cada vez.

## 1. Comprobaciones previas

- El checkout principal está limpio y en la rama de integración.
- `/verificar` dio "lista para integrar" en esta conversación, o el autor lo confirma.
- Las "decisiones no cubiertas por la spec" de los informes del implementador y de `/verificar` están incorporadas a la spec, o rechazadas con una corrección pedida. Si falta alguna, incorpórala ahora: edita la spec y haz commit con el mensaje `spec: <nombre>, decisiones de P01-T03`. Una spec que no describe el código no sirve para la siguiente tarea.

## 2. Alcance

Revisa `git diff --name-only <base>...<rama>`:

- **Rechaza** la rama si toca `specs/constitution.md`, `specs/gdd/`, `.claude/`, algún `CLAUDE.md`, `rust-toolchain.toml` o `deny.toml` sin que la tarea lo diga.
- **Tests de aceptación:** solo pueden cambiar en los commits de `autor-tests`. Compruébalo con `git log --format="%h %s" --name-only <base>..<rama> -- "*/tests/aceptacion_*"`. Todos los commits que salgan deben ser de tipo "tests de aceptación".

## 3. Actualizar la rama de la tarea

En el worktree: `cd "<worktree>" && git merge --no-edit <rama de integración>`.

- Si hay conflictos en contratos (`engine_core`, API de órdenes, snapshot, formatos), aborta con `git merge --abort` y para: decide el autor.
- Si el único conflicto es `Cargo.lock`, quédate con la versión de la rama de integración (`git checkout --theirs Cargo.lock`), regenéralo con `cargo check --workspace` y termina el merge con `git add Cargo.lock` y `git commit --no-edit`.
- Si son conflictos triviales en otro sitio, lanza `implementador` con un encargo de resolución: worktree, rama, "hay un merge en curso con la rama de integración", archivos en conflicto y qué conservar de cada lado. Los conflictos en tests de aceptación los resuelve `autor-tests`, no el implementador.

## 4. Puerta completa

En el worktree, ya con lo último de la rama de integración, pasan las cinco comprobaciones de `CLAUDE.md`. Además pasan los tests de determinismo y los replays dorados cuando existan. Si algo falla, no se integra.

Si un replay dorado cambia de resultado, la spec de la tarea tiene que justificarlo. Si no lo justifica, no se integra.

## 5. Merge

En el checkout principal:

1. `git merge --no-ff --no-commit <rama>`. No puede haber conflictos, porque el paso 3 ya los resolvió.
2. En el plan, marca la tarea como `integrada (AAAA-MM-DD)`.
3. `git add` del plan y `git commit -m "Integra P01-T03: <título>"`.

## 6. Limpieza

Pide confirmación al autor y después ejecuta `git worktree remove .worktrees/p01-t03` y `git branch -d tarea/p01-t03-<slug>`.

## 7. Resumen

Di qué entró, qué tareas quedan desbloqueadas para la siguiente oleada y qué preguntas siguen pendientes.

---
name: implementar
description: Ejecuta tareas de un plan con autor-tests e implementador, cada tarea en su propio git worktree
argument-hint: <ruta del plan> [P01-T03 P01-T04]
disable-model-invocation: true
---

Ejecuta tareas del plan indicado en `$ARGUMENTS`. Eres el orquestador: preparas, delegas y compruebas. No implementas tú, salvo en el modo ligero.

## Modo ligero

Para las tareas de tipo `ligera` del plan (sin contrato nuevo, sin tocar `engine_core` ni código de simulación con determinismo en juego, menos de unas 100 líneas previsibles), la ceremonia completa cuesta más que el código. En ese caso:

1. La sesión principal implementa la tarea en el checkout principal, en la rama de integración, sin worktree ni `autor-tests`.
2. Antes de proponer el commit pasan las cinco comprobaciones de `CLAUDE.md`, y `revisor-constitucion` revisa los cambios sin commit con el encargo habitual más la línea `modo: ligero`, la ruta absoluta del checkout principal en lugar de worktree y rama, y `HEAD` como commit base.
3. El commit lo pide el autor. El mensaje lleva el id de la tarea y el plan la marca como `integrada` en el mismo commit.

Todo lo demás, y siempre los contratos y el código de simulación, va por el flujo completo que sigue.

## 1. Comprobaciones previas

- `git status --porcelain` tiene que estar vacío. Los worktrees solo ven lo que está en commits. Si la spec, el plan o el contrato no están commiteados, para y pide permiso para hacerlo.
- La rama actual es la rama de integración, en adelante BASE. Anota también el commit base con `git rev-parse HEAD`.
- El plan tiene `Estado: aprobado` o `en curso`. Si está en `aprobado`, cámbialo a `en curso` y haz commit con el mensaje `plan: P01 en curso` antes de crear worktrees.

## 2. Elegir tareas

- Las que indique `$ARGUMENTS` o, si no indica ninguna, la siguiente oleada: las tareas cuyas dependencias están todas en estado `integrada`.
- **Como mucho dos implementadores a la vez**, porque cada uno compila su propio `target/`. Una tarea de tipo `contrato`, o que toque `engine_core` o un contrato compartido, va sola.
- Salta las tareas que ya tienen rama `tarea/pNN-tMM-*` o worktree: están en curso.
- Si una tarea no está lista (preguntas abiertas que la afectan, contrato sin integrar), dilo y sáltala.

## 3. Preparar el worktree de cada tarea

```bash
git worktree add .worktrees/p01-t03 -b tarea/p01-t03-<slug-corto> <BASE>
```

Usa ids en minúsculas y nombres cortos: en Windows las rutas largas dan problemas, y cada worktree tendrá su propio `target/`. Anota la ruta absoluta del worktree.

## 4. Tests de aceptación

Este paso se salta en las tareas de tipo `contrato` y en las que no tienen archivo de tests en el plan.

Lanza `autor-tests` para todas las tareas de la oleada a la vez, en un solo mensaje. Este es el encargo:

```
Tarea: P01-T03 · <título>
Plan: <ruta>  ·  Spec: <ruta>
Criterios que cubre (texto literal de la spec):
- CA-2: ...
- CA-3: ...
Worktree: <ruta absoluta>  ·  Rama: tarea/p01-t03-<slug>
Contrato: <archivos y tipos públicos contra los que se escriben los tests>
Archivo de tests: <crate>/tests/aceptacion_<feature>_tNN.rs
Devuelve el informe con el formato de tu definición.
```

Revisa cada informe. Si los tests no compilan contra el contrato, o si quedan criterios sin cubrir sin un motivo, corrige el encargo y repítelo una vez. Si vuelve a fallar, para y reporta.

## 5. Implementación

Lanza `implementador` para las tareas de la oleada a la vez, en un solo mensaje. Este es el encargo:

```
Tarea: P01-T03 · <título>
Plan: <ruta>  ·  Spec: <ruta>
Criterios que cubre: CA-2, CA-3 (texto literal)
Worktree: <ruta absoluta>  ·  Rama: tarea/p01-t03-<slug>
Tests de aceptación (no se tocan): <rutas>
Contrato que usa y no puede cambiar: <tipos y archivos>
Fuera de alcance: <lista>
Devuelve el informe con el formato de tu definición.
```

En una tarea de tipo `contrato`, la línea del contrato pasa a ser `Contrato que defines: <firmas y dependencias de workspace de la spec>`, y no hay línea de tests.

No le pases tu razonamiento ni la conversación. El encargo y las rutas bastan, y lo demás lo lee él.

## 6. Al terminar cada implementador

- **Lee el informe**, pero no te fíes solo de él. Compruébalo tú con `cd "<worktree>" && git log --oneline <commit base>..HEAD && git diff --stat <commit base>...HEAD`.
- **Compila el worktree tú:** `cd "<worktree>" && cargo check --workspace --all-targets && cargo xtask check-arch`. Si falla, relanza `implementador` en el mismo worktree con el error como encargo de corrección. No hay hook que lo haga: en la app de escritorio el informe del subagente no llega como texto.
- **Preguntas.** Si el informe dice `Estado: bloqueado` o trae preguntas para el autor, añádelas a `specs/preguntas-pendientes.md` y haz commit con el mensaje `preguntas: P01-T03`.
- **Resumen al autor:** tarea, estado, rama, comprobaciones, decisiones no cubiertas por la spec y preguntas.

Siguiente paso: `/verificar P01-T03`. Aquí no se integra nada; eso lo hace `/integrar`.

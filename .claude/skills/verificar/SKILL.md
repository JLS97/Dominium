---
name: verificar
description: Revisión independiente y en paralelo de una rama de tarea antes de integrarla
argument-hint: <P01-T03 o nombre de rama>
disable-model-invocation: true
---

Verifica la tarea o rama `$ARGUMENTS`. Eres el orquestador.

## 1. Localizar

- Busca la rama y su worktree con `git worktree list` y `git branch --list "tarea/*"`.
- La base es `git merge-base <rama de integración> <rama>`.
- Lee el bloque de la tarea en su plan para saber qué revisores le tocan.

## 2. Puerta automática

En el worktree, en este orden, y para en el primer fallo:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-arch
cargo deny check
```

Si algo falla, no lances revisores. Lanza `implementador` en el mismo worktree con el error como encargo de corrección y vuelve a empezar.

## 3. Revisores en paralelo

Elige los revisores según el plan y según `git diff --name-only <base>...<rama>`:

| Revisor | Cuándo |
| --- | --- |
| `revisor-constitucion` | siempre |
| `cazador-determinismo` | si el diff toca `engine/core`, `engine/sim` o `game/rules` |
| `revisor-spec` (modo `diff`) | si toca `specs/game/`, archivos `.ron` o mecánicas en `game/rules` |
| `investigador-rendimiento` | si la tarea tiene presupuestos en su spec (hito 2 en adelante) |

Lanza a la vez, en un solo mensaje, todos menos `investigador-rendimiento`. El encargo lleva tarea, rutas del plan y de la spec, lo que queda fuera de alcance según el plan, rama, ruta absoluta del worktree, commit base y la lista de archivos cambiados (`git diff --name-only <base>...<rama>`). `revisor-spec` no tiene Bash: lee esos archivos en el worktree. No les pases el informe del implementador ni tu opinión: la revisión vale porque no está anclada.

`investigador-rendimiento` va después, cuando no haya nada compilando, con el mismo encargo y la instrucción de medir sin editar ni hacer commits en la rama en revisión.

## 4. Consolidar

- Quita los duplicados.
- Antes de presentar un hallazgo bloqueante o "a corregir", abre tú las líneas citadas y confirma que es real. Los falsos positivos se descartan diciendo por qué.
- Si `cazador-determinismo` confirma un fallo, lanza `autor-tests` en modo regresión para que añada al archivo de aceptación de la tarea el test que lo atrapa, antes de que el implementador lo corrija.

## 5. Veredicto al autor

- **Lista para integrar:** propón `/integrar <tarea>`.
- **Necesita cambios:** con el OK del autor, lanza `implementador` en el mismo worktree con la lista de hallazgos confirmados y repite `/verificar`. Como mucho dos vueltas; a la tercera, para y pregunta.
- **Necesita una decisión del autor:** pregúntale y añade la pregunta a `specs/preguntas-pendientes.md`.

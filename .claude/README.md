# Flujo multiagente: guía para el autor

Esta carpeta configura Claude Code para Plus Ultra: Dominium. Cuando abres Claude Code en la raíz del repo, la sesión principal carga `CLAUDE.md`, que la convierte en **orquestador**. Además tiene disponibles los subagentes, los comandos y los hooks de aquí. Las reglas del orquestador están en la sección "Flujo multiagente" de `CLAUDE.md`. Esta guía explica cómo ponerlo en marcha y cómo usarlo.

## Qué hay

```
.claude/
├── settings.json        permisos, hooks y límite de subagentes en paralelo
├── agents/              ocho subagentes
├── skills/              /estado /spec /planificar /tareas /implementar /verificar /integrar, una carpeta por comando con SKILL.md
└── hooks/               dos scripts de PowerShell
.worktrees/              un worktree por tarea en curso (ignorado por git)
specs/plans/             planes y tareas, con plantilla
specs/preguntas-pendientes.md   bandeja de preguntas para ti
```

| Subagente | Modelo | Puede editar |
| --- | --- | --- |
| `explorador` | haiku | nada |
| `autor-tests` | el de la sesión | `tests/`, `benches/` y el `Cargo.toml` de su crate, solo en su worktree |
| `implementador` | el de la sesión | su worktree, salvo specs, `CLAUDE.md`, `.claude/`, tests de aceptación, `rust-toolchain.toml` y `deny.toml` |
| `revisor-constitucion` | el de la sesión | nada |
| `cazador-determinismo` | el de la sesión | nada |
| `revisor-spec` | sonnet | nada |
| `investigador-rendimiento` | el de la sesión | su worktree, con las mismas exclusiones que el implementador |
| `analista-partidas` | sonnet | solo `.ron`, en su worktree |

| Hook | Cuándo | Qué hace |
| --- | --- | --- |
| `inicio-sesion.ps1` | al abrir o reanudar una sesión | pone en el contexto la rama, los cambios sin commit, los worktrees abiertos y el número de preguntas pendientes |
| `guarda-ediciones.ps1` | antes de cada edición | aplica la columna "Puede editar" de la tabla de arriba |

`settings.json` también fija `CLAUDE_BASH_MAINTAIN_PROJECT_WORKING_DIR=1`: después de cada comando de Bash, el directorio vuelve a la raíz del repo, para que un `cd` a un worktree no haga que el siguiente commit caiga en la rama equivocada.

Los permisos de `settings.json` dejan pasar sin preguntar `cargo check/clippy/test/fmt/build/bench/xtask/deny` y los comandos de git de lectura, `add`, `commit` y `worktree add`. Te preguntan antes de editar la constitución, el GDD, el `CLAUDE.md` raíz, la configuración de `.claude/`, `rust-toolchain.toml` y `deny.toml`, y antes de `push`, `reset`, `checkout`, `rebase`, borrar worktrees o ramas. El `push --force` está prohibido.

## Puesta en marcha (una vez)

1. **Commit de la configuración.** Los worktrees solo ven lo que está en commits, y los subagentes necesitan los hooks y las specs.

   ```bash
   git add .claude CLAUDE.md .gitignore specs
   git commit -m "Flujo multiagente: subagentes, comandos y hooks"
   ```

2. **Abre Claude Code en la raíz del repo** y acepta confiar en la carpeta: sin eso, los hooks del proyecto no se ejecutan. Comprueba:
   - `/agents` lista los ocho subagentes.
   - `/hooks` muestra SessionStart y PreToolUse.
   - Al empezar la sesión aparece el bloque "Contexto de inicio".
3. **Prueba la guarda y los permisos.** Pide a la sesión:
   - "Crea el worktree `.worktrees/prueba` con la rama `tarea/prueba` y lanza el implementador con un encargo mínimo de prueba (tarea `PRUEBA`, sin plan, spec, contrato ni tests, y diciéndole que es una prueba de los hooks y que eso no es motivo para terminar con `Estado: bloqueado`) para que añada un comentario en `engine/core/src/lib.rs` dentro de ese worktree con la herramienta Edit." Tiene que editar sin pedirte permiso.
   - "Ahora lanza el implementador, con el mismo encargo mínimo, para que añada un comentario en `engine/core/src/lib.rs` del checkout principal." Tiene que bloquearlo el hook con el mensaje "solo edita dentro de su worktree".
   - Si el primero te pide permiso o el segundo no se bloquea, mira la salida de `claude --debug`. Al acabar, borra el worktree y la rama de prueba.
4. **Recomendado para compilar en paralelo:**
   - `sccache`, para que cada worktree no recompile `wgpu` y el resto de dependencias desde cero. Instálalo con `cargo install sccache --locked` y define la variable de entorno de usuario `RUSTC_WRAPPER=sccache`.
   - Rutas largas en git: `git config core.longpaths true`.
   - En VS Code, que rust-analyzer no indexe los worktrees: añade `"rust-analyzer.files.exclude": [".worktrees"]` en la configuración del workspace.
   - Cada worktree con su `target/` ocupa varios GB. `/integrar` los borra al terminar.

## Cómo empezar a usarlo

La primera sesión, según el "Siguiente paso" del GDD:

1. `/estado`: comprueba que todo se ve bien y qué queda pendiente.
2. Contesta las preguntas abiertas del GDD que afecten al hito 0. Las demás pueden esperar.
3. `/spec arquitectura engine_core`. Revisa y aprueba. Después `engine_sim`, y luego `engine_render`, `game_rules` y `game_app`. Las specs de arquitectura de core y sim son los contratos de todo lo demás, así que merecen calma.
4. `/spec motor simulacion-sintetica`: la simulación sintética del hito 0. Sus criterios de aceptación recogen todas las comprobaciones de la constitución marcadas "hito 0" en su tabla de estado: avance por ticks enteros (regla 4), guardar y cargar con el mismo hash (6), sin globales mutables (7), contadores lineales de vecinos (9), carga de todos los datos (11), mismo hash con pools de 1, 3, 6 y 12 hilos (13), binario sin gráficos que reporta ticks por segundo (14), tipo `Tick` y sin flotantes de tiempo (17), rechazo por hash de datos (18) y cero asignaciones por tick (19), además de las 10.000 unidades sintéticas con paseo aleatorio, fases paralelas, eventos y borrados.
5. `/planificar specs/engine/simulacion-sintetica.md`, luego `/tareas specs/plans/01-simulacion-sintetica.md`.
6. Por cada tarea: `/implementar specs/plans/01-simulacion-sintetica.md P01-T01`, después `/verificar P01-T01` y por último `/integrar P01-T01`. En el hito 0 van de una en una, y las tareas marcadas `ligera` en el plan las hace la sesión principal sin worktree, con la puerta de comprobaciones y `revisor-constitucion` antes del commit.

Un día normal sigue el mismo ciclo: `/estado` → `/implementar <plan>` (lanza la siguiente oleada) → `/verificar` de cada tarea → `/integrar` de una en una. Tú intervienes en tres puntos: apruebas specs, planes y tareas; contestas preguntas; y das el visto bueno a cada integración.

## Ajustes

- **Coste.** Los subagentes con modelo "el de la sesión" usan el modelo de la sesión principal. Para abaratar, cambia `model:` en su archivo a `sonnet`. Los primeros candidatos son `cazador-determinismo` y `revisor-constitucion` en tareas pequeñas. No lo haría con `implementador` ni con `autor-tests`.
- **Paralelismo.** `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=6` en `settings.json` es un tope de seguridad: da para dos implementadores y una ronda de revisores. El límite real de dos implementadores a la vez es una regla del orquestador en `CLAUDE.md`.
- **Permisos personales.** Lo que quieras permitir solo en tu máquina va en `.claude/settings.local.json`, que no se versiona.

## Limitaciones conocidas

- **La guarda es una red, no una jaula.** Bloquea Edit y Write, pero un subagente podría escribir un archivo con Bash. Por eso `/integrar` revisa el alcance de cada rama antes del merge.
- **Solo Windows.** Los hooks usan `powershell.exe`. Si el proyecto pasa a Linux, hay que instalar PowerShell 7 y cambiar `powershell.exe` por `pwsh` en `settings.json`.
- **No hay hook de fin de tarea.** Lo hubo (`comprobar-al-terminar.ps1`, evento SubagentStop) y se retiró el 26 de octubre de 2026: en la app de escritorio el informe del subagente llega por una herramienta de entrega y no como texto, así que `last_assistant_message` va vacío y el hook no podía leer la línea `Worktree:`. La compilación del worktree al terminar la hace el orquestador en el paso 6 de `/implementar`; los tests completos y el resto de la puerta los pasan `/verificar` e `/integrar`.
- **Agentes que dependen de lo que aún no existe.** `analista-partidas` necesita el binario sin gráficos con juego real (hito 3 o 4). `investigador-rendimiento` necesita los escenarios del hito 2. Los dos están marcados como inactivos en su definición y se revisan al llegar su hito.
- **Revisor de render pendiente.** Cuando empiece el hito 1 se escribe `revisor-render` (shaders, ciclo de vida de recursos de GPU, sincronización, regla 8). Hasta entonces `revisor-constitucion` cubre el render con su lista.

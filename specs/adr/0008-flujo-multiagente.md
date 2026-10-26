# ADR-0008: Flujo multiagente con un worktree por tarea

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** 0 en adelante

## Contexto

Todo el código lo generan agentes y el autor revisa y decide. Eso trae riesgos propios: tests que pasan por construcción porque los escribe quien escribe el código; revisiones ancladas en el razonamiento de quien implementó; código que se desvía de la spec sin que nadie lo apunte; mecánicas de los Imperivm originales inventadas por un agente para rellenar un hueco; y dos agentes pisándose en el mismo checkout. Además, compilar es el cuello de botella de cualquier flujo con agentes en Rust. Spec Driven Development exige que la spec mande sobre el código, no al revés.

## Opciones consideradas

1. **Una sola sesión secuencial** que escribe spec, código, tests y se revisa. A favor: simple y barato. En contra: el mismo agente escribe los tests y el código, así que "pasa los tests" no significa nada; se revisa a sí mismo; el contexto se satura con el GDD, las specs y el código; no hay paralelismo.
2. **Varias sesiones a mano en ramas, sin worktrees.** A favor: paralelismo. En contra: cambiar de rama en un checkout compartido rompe compilaciones a medias y confunde a los agentes; sin guardas, cualquiera edita cualquier cosa; la coordinación la lleva el autor de memoria.
3. **Orquestador con subagentes de rol.** La sesión principal escribe specs, planifica, delega, revisa e integra. Cada tarea va en su propio worktree bajo `.worktrees/`. Hooks que limitan qué edita cada rol. Tests de aceptación escritos por un agente distinto antes de implementar. Revisores que no reciben el informe del implementador. Integración en serie con la puerta completa en verde. A favor: cada riesgo del contexto tiene un contrapeso concreto, y el autor interviene en tres puntos: aprobar specs y planes, responder preguntas y dar el visto bueno a cada integración. En contra: ceremonia; cada worktree compila su propio `target/`, que son gigabytes y minutos; ocho definiciones de agente que mantener.

| Dimensión | Sesión única | Ramas a mano | Orquestador |
| --- | --- | --- | --- |
| Tests que significan algo | no | depende | sí, por construcción |
| Revisión sin anclaje | no | manual | sí |
| Aislamiento entre tareas | no aplica | frágil | worktree por tarea |
| Coste de compilación | uno | uno por rama activa | uno por worktree, con `sccache` |
| Ceremonia | mínima | media | alta, con modo ligero |

## Decisión

La opción 3, con dos límites para que la ceremonia no mande: un **modo ligero** para las tareas pequeñas sin contrato nuevo, que en el hito 0 son la norma, y **como mucho dos implementadores a la vez**. Las reglas del orquestador están en la sección "Flujo multiagente" de `CLAUDE.md`; la configuración, en `.claude/`; la guía para el autor, en `.claude/README.md`.

## Consecuencias

- Más fácil: que "pasa los tests" signifique "cumple la spec"; una revisión que ve lo que la CI no ve; que las dudas vuelvan al autor en vez de rellenarse con inventos; integrar sin pisarse.
- Más difícil: el coste de compilación por worktree, que `sccache` reduce pero no elimina; mantener ocho definiciones de agente, dos de ellas inactivas hasta su hito; y las specs tienen que traer criterios `CA-n` y firmas concretas, porque sin eso `autor-tests` y `/planificar` no tienen de qué agarrarse.
- Vigilar: la ceremonia frente al avance. Si una tarea ligera tarda más por el flujo que por el código, es señal de ajustar. Y los hooks dependen de campos documentados de Claude Code (`agent_type` en PreToolUse): se vuelven a comprobar al subir de versión. El hook de SubagentStop se retiró el 26 de octubre de 2026 porque en la app de escritorio no recibe el informe del subagente; la compilación del worktree al terminar la hace el orquestador.
- Se revisa si: Claude Code cambia la semántica de hooks o subagentes, o si al terminar el hito 1 el flujo no ha detectado nada que la puerta automática no detectara ya.

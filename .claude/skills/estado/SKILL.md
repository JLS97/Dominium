---
name: estado
description: Resume el estado del proyecto (rama, worktrees, planes, preguntas) y propone el siguiente paso
disable-model-invocation: true
---

Resume el estado de Plus Ultra: Dominium. Solo lectura: no edites, no commitees y no lances implementadores. Si hay mucho que leer, usa `explorador`.

Reúne:

1. La rama actual, los cinco últimos commits y los cambios sin commit (`git status --short`).
2. Los worktrees de tareas (`git worktree list`) y las ramas `tarea/*`, con su último commit. Indica si ya pasaron `/verificar` en esta conversación; si no lo sabes, dilo.
3. Los planes de `specs/plans/`: cuántas tareas hay en cada estado y cuáles están desbloqueadas, es decir, con todas sus dependencias integradas.
4. Las preguntas sin responder de `specs/preguntas-pendientes.md` y las que siguen abiertas en la sección final del GDD.
5. El hito actual, según la sección "Estado" de `CLAUDE.md` y el plan de hitos del GDD.

Responde en menos de 25 líneas y termina con un único siguiente paso recomendado, con el comando exacto que lo ejecuta.

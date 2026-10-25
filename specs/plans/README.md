# Planes y tareas

Un archivo por spec que se implementa, `NN-slug.md`, con el plan y sus tareas. Plantilla en `plantilla-plan.md`.

Los escribe la sesión principal con `/planificar` y `/tareas`, y el autor los aprueba antes del commit. Cada tarea tiene un id `PNN-TNN` que se usa en el worktree (`.worktrees/pnn-tnn`), en la rama (`tarea/pnn-tnn-slug`) y en los mensajes de commit.

En el archivo, el estado de una tarea solo es `pendiente` o `integrada`. Lo que está en curso se ve con `git worktree list` y en las ramas `tarea/*`. Así el plan solo cambia cuando algo entra en la rama de integración.

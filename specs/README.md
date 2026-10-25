# Specs

Las specs son la fuente de verdad del proyecto y se versionan con el código.

| Carpeta | Contenido |
| --- | --- |
| `constitution.md` | Reglas inviolables y cómo se comprueban |
| `adr/` | Decisiones técnicas cerradas, una por archivo, con plantilla |
| `gdd/` | Documento de contexto y GDD núcleo |
| `architecture/` | Una spec por crate: responsabilidad, API pública y lo que no debe hacer |
| `engine/` | Specs de features del motor |
| `game/` | Specs de features del juego |
| `reviews/` | Revisiones externas o independientes, como registro; no son fuente de verdad |
| `plans/` | Un plan con sus tareas por cada spec que se implementa, con plantilla |
| `preguntas-pendientes.md` | Dudas para el autor que surgen al trabajar; las respuestas se llevan a la spec, al GDD o a un ADR |

Orden de trabajo para cualquier feature: spec, plan, tareas, implementación y verificación. Ninguna feature sin spec con criterios de aceptación medibles.

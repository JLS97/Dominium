# ADR: registros de decisión de arquitectura

Una decisión técnica cerrada por archivo, numerada. Plantilla en `0000-plantilla.md`.

Un ADR no se edita cuando cambia la decisión: se escribe otro que lo sustituye y se marca el antiguo como sustituido. Cambiar una regla de la constitución exige un ADR (ver `../constitution.md`).

## Índice

| ADR | Decisión | Estado |
| --- | --- | --- |
| [0001](0001-rust-frente-a-cpp.md) | Rust frente a C++ | aceptado |
| [0002](0002-motor-propio-frente-a-bevy.md) | Motor propio frente a Bevy, con la razón real y el coste asumido | aceptado |
| [0003](0003-wgpu-frente-a-ash.md) | `wgpu` frente a Vulkan directo con `ash`, con deberes | aceptado |
| [0004](0004-tablas-soa-sin-ecs.md) | Tablas SoA con un handle común, sin ECS | aceptado |
| [0005](0005-animacion-esqueleto-horneado.md) | Animación de multitudes con esqueleto horneado y skinning en GPU | aceptado |
| [0006](0006-tick-20-hz.md) | Tick de simulación a 20 Hz | aceptado |
| [0007](0007-guardado-binario-postcard.md) | Guardado binario versionado con `postcard`; RON solo para autoría | aceptado |
| [0008](0008-flujo-multiagente.md) | Flujo multiagente con un worktree por tarea, tests de aceptación aparte e integración en serie | aceptado |
| [0009](0009-hashers-deterministas.md) | Hashers deterministas: `rustc-hash` para colecciones y xxh3 para estado, datos y aleatoriedad sin estado | aceptado |

## Candidatos a ADR en próximos hitos

- Sistema héroe-ejército (opción a, b o c), cuando el autor decida; se cierra en el hito 2.
- Modo de presentación y uso del SMT, con las mediciones del hito 2.
- Mapas: editor propio, procedurales o mezcla; hito 7.
- Interfaz definitiva de juego; antes del hito 5.

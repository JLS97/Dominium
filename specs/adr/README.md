# ADR: registros de decisión de arquitectura

Una decisión técnica cerrada por archivo, numerada: `0001-rust.md`, `0002-motor-propio-frente-a-bevy.md`, etc. Plantilla en `0000-plantilla.md`.

Un ADR no se edita cuando cambia la decisión: se escribe otro que lo sustituye y se marca el antiguo como sustituido.

## Pendientes de escribir

1. Rust frente a C++.
2. Motor propio frente a Bevy. La razón real: el bucle del agente, tiempo de compilación y deriva de la API; y el coste asumido, alrededor de un tercio del motor en infraestructura.
3. `wgpu` frente a Vulkan directo con `ash`, con los deberes: versión fijada, features nativas probadas en el hito 1, anillo de staging propio, golden images WARP contra WARP.
4. Tablas SoA con handle común, sin ECS.
5. Animación de multitudes con esqueleto horneado y skinning en GPU, no vertex animation textures.
6. Tick de simulación a 20 Hz.
7. Guardado binario versionado con `postcard`; RON solo para autoría.

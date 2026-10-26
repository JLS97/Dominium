# Datos

Contenido del juego en RON, editado a mano y versionado como texto: unidades, facciones, héroes, edificios, habilidades, plantillas de ciudad y perfiles de IA (regla 11 de la constitución). Va separado de `assets/`, que guarda arte y audio y puede acabar en Git LFS. Decisión del autor del 26 de octubre de 2026 (Q-004 de `specs/preguntas-pendientes.md`).

- Todo archivo de aquí lo carga un test (regla 11) y entra en el hash de datos de guardados y replays (regla 18). Cambiar un dato invalida los replays dorados.
- Hot-reload solo en builds de desarrollo.
- Hito 0: el escenario de la simulación sintética (unidades, semilla y ticks). La ruta y los campos exactos los fija `specs/engine/simulacion-sintetica.md`.
- Los números de juego que vengan de los Imperivm originales llevan su origen en la spec correspondiente. Nadie los inventa.

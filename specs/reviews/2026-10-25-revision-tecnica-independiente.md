# Revisión técnica independiente del GDD v2

- **Fecha:** 25 de octubre de 2026
- **Quién:** un agente de IA actuando como programador senior de motores, a petición del autor, con el GDD v2 como única entrada y sin conocer las conclusiones de la revisión anterior.
- **Qué es:** una opinión técnica con estimaciones aproximadas. No es fuente de verdad. Lo que se adoptó de ella está en el GDD v2.1, con la marca (v2.1), y las decisiones cerradas tendrán su ADR. Lo que haya que medir se mide en los hitos 0, 1 y 2.

---

**Resumen.** Sim/render separados y dependencias en un solo sentido: correcto. El resto de lo que importa está mal planteado o ausente: presupuestos que no describen ninguna necesidad real (144 Hz, "tick a 10×"), la técnica de animación cara (VAT), y ni identidad de unidades, ni orden de eventos, ni definición de "canónico" cuando hay cálculo incremental.

## 1. CPU vs GPU

Coste por tick en el 5600X, un hilo, 15k vivas y 3k en melé:

- Grid (counting sort estable por celda): 10–20 ns/unidad → 0,2–0,3 ms.
- Vecinos (3×3 celdas, 40–70 candidatos en melé): 200–400 ns por unidad luchando → 1,5–3 ms. Coste dominante, limitado por fallos de caché, no por ALU.
- Movimiento (flow field + separación + integración): 30–60 ns → 0,5–0,9 ms.
- Combate (objetivo, cooldown, eventos, disparadores): 100–200 ns por luchador → 0,3–0,6 ms.
- Estado por unidad: 20–50 ns → 0,3–0,7 ms.
- Flow fields: 10–30 µs por sector de 32×32; escala con sectores, no con unidades. El riesgo son los picos (muro nuevo, 50 grupos re-rutando), no la media.
- IA estratégica/operativa: 50–500 µs por facción, repartida; tope ~0,3 ms/tick para 10 facciones.

Total 4–8 ms en un hilo; 1–2 ms con 5 workers; 3–4 ms en un 4c de 2017 con 3. Frente a 33 ms sobra. **En tiempo real la CPU no es el problema; lo es en headless (ticks/s) y en los picos de navegación.**

GPU, GTX 1060 a 1080p, ~4k unidades en frustum, LODs que promedien 300 triángulos: 1,2 M tri en el pase principal, ~3 M con tres cascadas. Con triángulos pequeños y fetch de animación la 1060 hace ~1 Gtri/s reales → ~3 ms de vértices; relleno con overdraw y PCF ~3 ms; depth de cascadas ~2 ms; terreno ~2 ms; UI y post ~1 ms. **≈11 ms: 60 FPS sí, 144 no.** En la 3070 (~3× en raster) ≈4 ms.

Conclusión: la sim se diseña contra la CPU (caché e hilos: SoA, unidades reordenadas por celda cada N ticks, enteros para cantidades de juego, cero asignaciones por tick); el render contra la GPU, con cero trabajo por unidad y por frame en CPU. La fila "subida de datos por frame" está mal: es **por tick** (15k × ~20 B = 300 KB) y el vertex shader interpola entre los dos últimos buffers. Interpolar o hacer culling en CPU a 144 Hz son 2,2 M actualizaciones/s: un núcleo quemado.

## 2. Hilos

- **Principal**: winit, entrada, egui y codificación de render. Con GPU-driven son 20–40 draws indirectos; un hilo de render aparte solo añade sincronización.
- **Sim**: posee el `World`, tick fijo con acumulador, órdenes por canal selladas con el tick en que se aplican (eso es el replay), publica snapshots.
- **IO**: assets y guardado desde una copia del canónico (15k × ~200 B ≈ 3 MB, <1 ms de copia).
- **Pool rayon**: núcleos físicos − 1: 5 en el 5600X, 3 en el 4c/8t. El SMT da poco en bucles SoA y compite con el hilo principal; se valida en hito 2. Solo lo usa la sim.

Determinismo en paralelo: cada fase es "map sobre entrada congelada, salida por slot". Los eventos se escriben en buffers por chunk con `par_chunks(512)` (tamaño fijo: el número de chunks no depende de los hilos), se concatenan en orden de chunk, se ordenan de forma estable por (objetivo, origen, tipo) y se aplican en una fase secuencial: daño, muertes, borrados. Aleatoriedad sin estado: `hash(semilla, tick, handle, propósito)`. Grid con orden estable (celda, slot): así el orden de vecinos, y cualquier suma de flotantes, es fijo. La prueba que importa no es "dos ejecuciones, misma semilla", sino **misma semilla con pool de 1, 3, 6 y 12 hilos, mismo hash por tick**.

Snapshots: anillo de 4 buffers preasignados tras `Arc`; la sim escribe en uno libre, el render retiene (anterior, actual) y sube dos storage buffers y un alfa. Tiempo de render = tiempo de sim − 1 tick − ~5 ms de margen, suavizado; sin esto, FPS altos con stutter.

Emular el objetivo en la referencia: CPU con `start /affinity FF` (4 núcleos físicos con SMT), pool de 3, plan de energía con procesador al 99 % (sin boost, 3,7 GHz) y ×1,3 sobre lo medido por el IPC y reloj de un i5 de 2017. GPU: la 3070 rinde ~3×, así que el presupuesto en la 3070 es **un tercio** del objetivo, medido con timestamp queries, nunca en FPS; relleno comprobado a 4K. Memoria: asignador rastreado con tope de 2,5 GB de VRAM (hay 1060 de 3 GB) y Job Object de 3 GB de working set para reventar pronto.

Presupuestos (objetivo 4c/8t, 1060, 8 GB, 1080p, p99):

| Métrica | Objetivo | En la referencia |
| --- | --- | --- |
| Frame | 16,6 ms; GPU ≤ 12 (unidades 5, sombras 3, terreno 2, resto 2); CPU render ≤ 4 | GPU ≤ 4 ms |
| Tick tiempo real, 15k | ≤ 10 ms, 3 workers | ≤ 6 ms, 5 workers |
| Headless 15k | ≥ 150 ticks/s | ≥ 400 |
| Headless 300 unidades | ≥ 5.000 ticks/s | ≥ 10.000 |
| Pico de navegación | ≤ 2 ms/tick, resto diferido | igual |
| Working set / VRAM | ≤ 3 / ≤ 2,5 GB | igual |
| Guardar | hitch ≤ 50 ms, archivo ≤ 20 MB | igual |

El de 300 unidades manda: una partida de fase A de 4 h son 432k ticks a 30 Hz; a 10× tarda 43 min, inútil para equilibrar. Hacen falta 100×. Y 20 Hz, no 30: un tercio menos de ticks por hora y el render lo esconde.

## 3. Animación: VAT no

Soldado LOD0 de 1.500 vértices, 15 clips, 450 frames a 30 fps:

- **VAT**: posición + normal ≈ 8 B por vértice y frame → 5,4 MB por malla y LOD. Ocho mallas por facción, 11 facciones, dos LODs: ~650 MB de VRAM, y cada malla o arma nueva es un rehorneado. Colgar un arma de la mano exige hornear además el hueso: ya estás a medio camino del esqueleto. La herramienta hace skinning en CPU por frame, layout por malla, y se reejecuta al tocar cualquier malla.
- **Esqueleto horneado + skinning en GPU**: rig compartido de 24–32 huesos, 24 B por hueso y frame en f16 → **~350 KB para todo el set de todas las facciones humanas**. Vértice: 4 lecturas de storage buffer y una multiplicación, ~1,5–2× el vértice VAT (+1 ms sobre ~3 en la 1060). Blending: lerp de dos frames. Props: malla instanciada con (clip, frame, hueso); armas y estandartes gratis. Pipeline: el glTF ya trae skin y clips; "hornear" es muestrearlos a un buffer al cargar, ~200 líneas, sin herramienta aparte. Héroes y tropa por el mismo camino; el documento propone dos.

Recomendación: esqueleto horneado, 4 influencias en LOD0, 1–2 en LOD1/2 e **impostores** (billboard con pocos ángulos y un ciclo de 4 frames) en LOD3, que es donde las 15k lejanas dejan de costar. Desfase de fase por instancia. VAT solo paga en deformación no rígida, que no tienes.

## 4. Diez riesgos

1. **Métricas equivocadas.** "6,9 ms" y "3,3 ms a 10×" no describen ninguna necesidad. Sustituir por la tabla del punto 2.
2. **Determinismo incompleto.** Faltan grid estable, chunking fijo, RNG sin estado y el reparto de la IA entre ticks como estado canónico. El hot-reload de RON rompe replays: hash de datos en save y replay, hot-reload solo en dev.
3. **Canónico/derivado mal delimitado.** Un flow field incremental o una caché de IA es derivado pero cambia velocidades; al cargar, si no se reconstruye idéntico y en el mismo punto, el hash de la regla 6 falla. O se reconstruye completo y determinista al cargar, o su progreso es canónico. Y RON para saves de 15k unidades es lento y enorme: binario versionado (`postcard`); RON solo para autoría.
4. **Sin modelo de identidad.** Afecta a objetivos, héroe→ejército, selección, órdenes e interpolación. Se decide en hito 0 (punto 5).
5. **Hito 2 mide la GPU con cápsulas.** Sin skinning, textura ni sombras no mide nada; el presupuesto real aparece en hito 5 con la fase A encima. Malla humanoide placeholder con rig, 3 LODs, impostor y cascadas en hito 2.
6. **VAT.** Punto 3.
7. **Flow fields sin lo que los rompe.** Sin coste por densidad, un portal estrecho da oscilación y apelotonamiento. "Formaciones encima" no es un diseño: la formación es una entidad con su camino y slots a los que los soldados llegan con *arrive*, con embudo en portales más estrechos que ella. Puerta, choque frontal y muro en caliente como aceptación del hito 2.
8. **ECS aplazado a después del hito 2.** Héroe↔soldados, edificio↔cola, proyectil↔objetivo se fijan en hito 1. Decidir ya: sin ECS; edificios, héroes y proyectiles son tablas más con los mismos handles. `hecs` añade un segundo modelo de identidad y serialización por nada.
9. **`wgpu`, correcto con deberes.** Fijar versión, subir una vez por hito. `MULTI_DRAW_INDIRECT_COUNT`, `INDIRECT_FIRST_INSTANCE` y `TEXTURE_BINDING_ARRAY` son features nativas: probarlas en hito 1 en la 3070 y en WARP. `Queue::write_buffer` por frame asigna staging: anillo mapeado propio. WARP es lento y da píxeles distintos al hardware: golden images WARP contra WARP, con tolerancia, en escenas pequeñas.
10. **El ADR de Bevy da la razón menor y oculta el coste.** La razón real con código generado es el tiempo de compilación y la deriva de API en el bucle del agente. El coste: carga de assets, escena, entrada, integración de UI y depuración son ~30 % del motor y no están en ningún hito. Y "mismo hash" no detecta comportamiento erróneo pero determinista: pruebas de propiedades (nadie dentro de un muro, HP ≥ 0, grid consistente) y visor de replays en hito 1.

## 5. Lo que falta

- **Identidad y borrado**: handle = slot + generación; tablas densas con swap-remove e indirección `handle→slot`/`slot→handle`; borrado diferido al final del tick, por slot descendente, nunca a mitad de fase. Conjunto activo: la mayoría de las 15k están de guarnición y no deben pasar por combate.
- **Desempates de eventos**: dos matan al mismo en el mismo tick: crédito, experiencia, oro cartaginés.
- **Grid de Europa**: si son 8×6 km a 1 m, 48 M celdas: 48 MB de coste estático, bien; el campo de integración (2 B/celda) solo cabe por sectores. ~47k sectores y 100–200k portales: el A* de una ruta larga cuesta ms → caché de rutas y segundo nivel jerárquico. Pasabilidad por facción como capa "dueño de puerta" por celda, no 11 grids. LRU de campos por grupo con tope de memoria.
- **Muerte y spawn en render**: casar por handle; spawn sin lerp; las muertes salen de la tabla viva, así que el render necesita una lista derivada de cadáveres (handle, pos, clip, tick) alimentada por eventos, o la unidad desaparece sin animación.
- **Guardado**: colas de órdenes, formaciones, peticiones de navegación pendientes, cooldowns, temporizadores de IA, proyectiles, flujos RNG, hash de datos.
- **Streaming**: terreno por chunks desde IO con tope de subida por frame (~2 MB).
- **Frame pacing**: Mailbox frente a Fifo, `timeBeginPeriod` y el desfase sim–render del punto 2.
- **Asignaciones**: arenas por hilo para vecinos, eventos y scratch de Dijkstra; snapshots reciclados; `collect` prohibido en el camino caliente.
- **Tiempo entero**: tick `u64`, duraciones en ticks; nunca `f32` segundos en la sim. Hash de estado cada N ticks en release (3 MB ≈ 0,3 ms).

## 6. Hitos 0–2

- **Hito 0**: "determinismo con entidades vacías" no prueba nada. Tabla sintética de 10k unidades con paseo aleatorio, fases paralelas, eventos, swap-remove y hashes con pool de 1/3/6/12. Fijar aquí handles, enteros, orden de fases, pipeline de eventos, formato de replay (semilla + hash de datos + órdenes) y el buscador de desync que hashea por sistema. Todo lo demás depende de esto.
- **Hito 1**: 20 contra 20 vale como corte del bucle, pero con el spike de `wgpu` (indirecto, arrays de texturas, timestamps), interpolación en GPU con anillo de snapshots, cadáveres y spawns, y "cargar y continuar 1.000 ticks con otro número de hilos".
- **Hito 2**: tal cual pasa con cápsulas en campo abierto y aplaza los fallos a los hitos 5 y 10. Cambiar: malla skinned placeholder con LODs, impostor y cascadas; escenarios de marcha de 10k, choque 2×1,5k, 3k por una puerta de 4 celdas, muro en caliente, 10 IAs stub con presupuesto; medición con afinidad a 4 núcleos y un tercio de GPU; ticks/s a 15k y a 300; guardar/cargar a 10k; contadores de RAM y VRAM; carrera headless de 500k ticks con memoria y tiempo de tick planos, porque las fugas nacen aquí, no en el hito 4.

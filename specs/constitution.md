# Constitución de Plus Ultra: Dominium

Versión 1 · 25 de octubre de 2026 · Redactada a partir de la tabla "Constitución" del GDD v2.1

Estas reglas no las rompe ningún agente ni ninguna persona. Cada una lleva su motivo y su comprobación. Lo que el compilador pueda imponer, lo impone el compilador; lo que no, un test, un contador o una revisión explícita. Los presupuestos de rendimiento no son reglas: son objetivos medibles y van en su propia sección.

Las reglas conservan el número que tienen en el GDD, porque las comprobaciones y sus mensajes de error los citan. Aquí están agrupadas por tema para leerlas.

## Cómo leer una regla

- **Enunciado:** lo que se cumple siempre.
- **Motivo:** por qué existe, en una o dos frases. Si el motivo deja de ser cierto, la regla se revisa con un ADR.
- **Comprobación:** quién la hace cumplir y cuándo. *Compilador* significa que un atributo o un lint lo impide. *Automática* significa un test o un contador que corre en CI. *Revisión* significa que no se puede automatizar y hay que mirarlo en cada cambio.

## Estructura del código

### Regla 1. Las dependencias van de `game/` a `engine/`, nunca al revés.

- **Motivo:** el motor no sabe qué es un legionario, una aldea o la comida. Así el juego cambia sin tocar el motor, el motor puede separarse a otro repositorio sin esfuerzo y los agentes no crean acoplamientos circulares sin querer.
- **Comprobación:** automática. `cargo xtask check-arch` lee el grafo resuelto del workspace y falla si un crate `engine_*` depende, directa o transitivamente, de un crate `game_*`.

### Regla 2. `engine_sim` y `game_rules` no conocen ni tocan gráficos ni ventana.

- **Motivo:** la simulación tiene que correr sin GPU para los tests, las partidas IA contra IA y la CI; y ni el render puede filtrar estado hacia la simulación ni la simulación hacia el render.
- **Comprobación:** automática. `check-arch` falla si cualquiera de los dos depende, directa o transitivamente, de `engine_render`, `wgpu`, `wgpu-core`, `wgpu-hal`, `naga`, `winit` o `egui`.

### Regla 3. No hay `unsafe` fuera de `engine_render`.

- **Motivo:** el código lo escriben agentes. El compilador detecta lo que un revisor humano no ve, pero solo si no se le apaga.
- **Comprobación:** compilador y automática. Cada crate salvo `engine_render` lleva `#![forbid(unsafe_code)]` en la raíz de `src/lib.rs` y de `src/main.rs`; `check-arch` lo verifica. El workspace fija además `unsafe_code = "deny"` como red para cualquier target nuevo. En `engine_render`, cada bloque `unsafe` lleva un comentario `SAFETY:` que explica la invariante; eso es revisión.

### Regla 16. Las herramientas que conocen conceptos de juego viven en `game/tools`.

- **Motivo:** es la regla 1 aplicada a las herramientas. Un editor de mapas coloca aldeas; no puede vivir en `engine/`.
- **Comprobación:** automática, por la regla 1: `engine_tools` no puede depender de `game_*`.

### Regla 11. Unidades, facciones, habilidades, plantillas de ciudad y perfiles de IA se definen por datos.

- **Motivo:** once facciones, nueve perfiles de IA y meses de equilibrado no pueden pasar por recompilar. Los datos son la frontera entre diseño y código.
- **Comprobación:** automática. Un test carga todos los archivos de datos del repositorio y falla si alguno no se puede leer o no valida. Pendiente de escribir en el hito 0.

## Simulación

### Regla 4. La simulación corre a tick fijo de 20 Hz y el render interpola.

- **Motivo:** el determinismo exige que cada tick haga exactamente lo mismo, sin un `dt` variable. 20 Hz y no 30 porque una partida de cuatro horas son 288.000 ticks y las partidas IA contra IA tienen que reproducirse en un minuto. La interpolación en el render esconde el tick al jugador. Ver ADR-0006.
- **Comprobación:** automática y revisión. La simulación solo avanza con una llamada por tick entero y no acepta tiempo en segundos; un test lo comprueba. Que el render interpola es revisión de su spec.

### Regla 5. Datos en SoA en el código caliente.

- **Motivo:** el coste de la simulación lo domina la caché, no la aritmética. Un vector por campo, no un vector de structs.
- **Comprobación:** revisión. No se automatiza. Las tablas de `engine_sim` son la única forma de guardar entidades; un vector de structs dentro de una fase por unidad es una violación.

### Regla 6. Todo lo que influye en la simulación es canónico y serializable; lo derivado es función pura del canónico y se reconstruye completo al cargar.

- **Motivo:** partidas de quince horas que se guardan y se cargan, y replays que se reproducen. Si un dato influye en el resultado y no se guarda, cargar cambia la partida. Si un cálculo es incremental, como un flow field a medias, su progreso es canónico.
- **Comprobación:** automática. Test: guardar, cargar, avanzar N ticks y obtener el mismo hash de estado que la partida que no se guardó. Pendiente del hito 0 con la simulación sintética y del hito 1 con la real.

### Regla 7. Los sistemas de `sim` son paralelizables: sin estado global compartido.

- **Motivo:** el paralelismo y el determinismo mueren con un global mutable. Cada fase lee entrada congelada y escribe por slot.
- **Comprobación:** compilador y revisión. Lints contra `static mut` y contra globales mutables en `engine_sim` y `game_rules`; el resto es revisión de que cada fase es un map sobre entrada congelada.

### Regla 9. Nada de algoritmos cuadráticos sobre el total de unidades; todo pasa por el grid espacial.

- **Motivo:** con quince mil unidades, cualquier bucle de todos contra todos son más de doscientos millones de operaciones por tick.
- **Comprobación:** automática. Contadores deterministas de comprobaciones de vecinos por tick; un test con N y con 2N unidades exige que crezcan de forma lineal. Pendiente del hito 0.

### Regla 10. El grid de navegación admite cambios en tiempo de ejecución y pasabilidad por facción.

- **Motivo:** murallas que se construyen y puertas que solo abre su dueño son fase B, pero rehacer la navegación entonces costaría más que diseñarla así desde el principio.
- **Comprobación:** automática. Tests de navegación con un muro añadido en caliente y con una puerta que una facción pasa y otra no. Pendiente del hito 2.

### Regla 13. La simulación es determinista con cualquier número de hilos: mismos inputs, mismo resultado.

- **Motivo:** replays, buscador de desincronizaciones, equilibrado reproducible y tests que valen algo. Determinismo en la misma máquina; entre plataformas no es requisito.
- **Comprobación:** automática y revisión. Test: misma semilla con pool de 1, 3, 6 y 12 hilos, mismo hash de estado en cada tick. Lo que lo hace posible y se revisa en cada cambio: hasher fijo (nunca `HashMap` ni `HashSet` con el hasher por defecto dentro de la simulación), chunks de tamaño fijo en las fases paralelas, grid espacial con orden estable, aleatoriedad sin estado derivada de semilla, tick y handle, y ninguna reducción paralela de flotantes. Pendiente del hito 0.

### Regla 14. La simulación puede ejecutarse sin gráficos y acelerada.

- **Motivo:** partidas IA contra IA para equilibrar, pruebas de resistencia de cientos de miles de ticks y CI sin GPU.
- **Comprobación:** automática. Un binario sin gráficos juega una partida corta en CI y reporta ticks por segundo. Pendiente del hito 0 en versión sintética y del hito 3 con juego real.

### Regla 17. El tiempo de la simulación se mide en ticks enteros; sin segundos en flotante en `sim` ni en `rules`.

- **Motivo:** los flotantes acumulan error, dependen del orden de las operaciones y rompen el determinismo en partidas largas. Un cooldown dura ticks, no segundos.
- **Comprobación:** compilador y automática. Un único tipo `Tick` en `engine_core` sobre `u64`, y las duraciones como diferencias de ticks. Un test de CI busca en `engine_sim` y `game_rules` flotantes con nombre de tiempo (`seconds`, `dt`, `elapsed`) y falla si los encuentra. Pendiente del hito 0.

### Regla 18. Guardados y replays llevan el hash de los datos con los que se jugó; el hot-reload solo existe en builds de desarrollo.

- **Motivo:** cambiar un archivo de datos cambia el resultado de un replay y puede dejar un guardado incoherente. El hot-reload es una herramienta de desarrollo, no una característica del juego.
- **Comprobación:** automática y compilador. Un test rechaza cargar un guardado o un replay con hash de datos distinto; el hot-reload está detrás de una feature de desarrollo que no entra en release. Pendiente del hito 0.

### Regla 19. Cero asignaciones de memoria en el camino caliente de la simulación.

- **Motivo:** las asignaciones por tick son parones y fragmentación en partidas de quince horas. Arenas por hilo, buffers reciclados y nada de `collect` en las fases por unidad.
- **Comprobación:** automática. Un asignador contador global en los tests; un test ejecuta un tick tras el calentamiento y exige cero asignaciones. Pendiente del hito 0.

## Render

### Regla 8. El render no tiene trabajo por unidad en la CPU más allá de la copia del snapshot: culling, LOD e interpolación en GPU.

- **Motivo:** quince mil unidades a 144 Hz son más de dos millones de actualizaciones por segundo si las hace la CPU. La simulación entrega un snapshot por tick y la GPU hace el resto.
- **Comprobación:** automática. Contador de draw calls por frame, que no puede depender del número de unidades, y perfil con `tracy` en el hito 2. Pendiente del hito 1.

## Juego e IA

### Regla 15. La IA juega con la misma API de órdenes que el jugador.

- **Motivo:** así la IA es determinista, queda grabada en el replay, no puede hacer trampas que el diseño no haya decidido y se prueba sin gráficos. Sus ventajas, cuando las tiene, son datos del perfil, no accesos privilegiados.
- **Comprobación:** revisión. El módulo de IA de `game_rules` solo lee estado a través de la interfaz de percepción y solo produce órdenes; no muta la simulación. Pendiente del hito 3.

### Regla 12. Los presupuestos de rendimiento son medibles y tienen umbral.

- **Motivo:** el rendimiento es un requisito del juego, no un deseo. Sin umbral se degrada un poco en cada cambio y nadie lo nota hasta que es tarde.
- **Comprobación:** automática. Benchmarks con `criterion` y umbral en la máquina de referencia; en CI, contadores y ticks por segundo con tolerancia, porque el cronómetro en CI es ruido. Pendiente del hito 2.

## Presupuestos

Los presupuestos se miden en milisegundos por pase y en ticks por segundo, en percentil 99 y nunca en FPS. Hay dos máquinas: la de **referencia**, donde se mide, y la **mínima**, que es el objetivo real y se emula desde la referencia.

| Nivel | Máquina | Objetivo |
| --- | --- | --- |
| Referencia | Ryzen 5 5600X, 16 GB, RTX 3070 8 GB, Windows 10 | 6,9 ms por frame a 1080p; simulación acelerada para pruebas |
| Mínima | 4 núcleos / 8 hilos, GTX 1060 6 GB o equivalente, 8 GB RAM | 16,6 ms por frame a 1080p; simulación en tiempo real |

| Métrica, percentil 99 | Objetivo en la mínima | Equivalente en la referencia |
| --- | --- | --- |
| Frame completo | 16,6 ms | 6,9 ms |
| GPU por frame | ≤ 12 ms: unidades 5, sombras 3, terreno 2, resto 2 | ≤ 4 ms |
| CPU de render por frame | ≤ 4 ms | ≤ 2 ms |
| Tick en tiempo real con 15.000 unidades | ≤ 10 ms con 3 workers | ≤ 6 ms con 5 workers |
| Sin gráficos con 15.000 unidades | ≥ 150 ticks/s | ≥ 400 ticks/s |
| Sin gráficos con 300 unidades | ≥ 5.000 ticks/s | ≥ 10.000 ticks/s |
| Pico de navegación | ≤ 2 ms por tick; el resto se difiere | igual |
| Memoria | working set ≤ 3 GB; VRAM ≤ 2,5 GB | igual |
| Guardar partida | parón ≤ 50 ms; archivo ≤ 20 MB | igual |
| Snapshot de simulación a render | ~300 KB por tick, no por frame | igual |

**Cómo se emula la mínima desde la referencia:** afinidad del proceso a 4 núcleos físicos, pool de 3 workers y plan de energía sin boost para la CPU, multiplicando lo medido por 1,3; un tercio del presupuesto de GPU, medido con timestamp queries por pase, y relleno comprobado a 4K; asignador rastreado con tope de 2,5 GB de VRAM y límite de 3 GB de working set. El detalle y el razonamiento están en la sección de escala del GDD.

Los presupuestos se revisan con los datos del hito 2. Cambiarlos no exige ADR, pero sí anotar en el GDD qué medición los cambió.

## Cómo cambiar una regla

1. Se escribe un ADR que explique por qué el motivo de la regla ya no es cierto o por qué otra cosa pesa más.
2. En el mismo cambio se actualiza o se retira la comprobación: ninguna regla queda sin comprobación ni ninguna comprobación sin regla.
3. En el mismo cambio se actualizan esta constitución y la tabla resumen del GDD.
4. Los números de regla no se reutilizan. Una regla retirada conserva su número con la nota "retirada por ADR-XXXX".

## Estado de las comprobaciones

| Regla | Comprobación | Estado |
| --- | --- | --- |
| 1, 2, 16 | `cargo xtask check-arch`, grafo de dependencias | hecha |
| 3 | `#![forbid(unsafe_code)]` verificado por `check-arch`; `SAFETY:` por revisión | hecha |
| 4 | test de avance por ticks enteros | hito 0 |
| 5 | revisión | siempre |
| 6 | test guardar, cargar, N ticks, mismo hash | hito 0 sintético, hito 1 real |
| 7 | lints contra globales mutables | hito 0 |
| 8 | contador de draw calls | hito 1 |
| 9 | contadores lineales de vecinos | hito 0 |
| 10 | tests de muro en caliente y puerta por facción | hito 2 |
| 11 | test de carga de todos los datos | hito 0 |
| 12 | benchmarks con umbral; contadores en CI | hito 2 |
| 13 | mismo hash con pools de 1, 3, 6 y 12 hilos | hito 0 |
| 14 | binario sin gráficos en CI con ticks por segundo | hito 0 sintético, hito 3 real |
| 15 | revisión del módulo de IA | hito 3 |
| 17 | tipo `Tick` y búsqueda de flotantes de tiempo | hito 0 |
| 18 | rechazo por hash de datos; hot-reload tras feature de desarrollo | hito 0 |
| 19 | asignador contador | hito 0 |

Las comprobaciones marcadas "hito 0" entran como criterios de aceptación de la spec `specs/engine/simulacion-sintetica.md`; las de hitos posteriores, en la spec de la feature que las haga posibles. Cuando el test exista, esta tabla pasa a decir "hecha" en el mismo commit.

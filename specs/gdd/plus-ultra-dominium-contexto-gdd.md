# Plus Ultra: Dominium — Documento de contexto y GDD núcleo

30 de octubre de 2026 · **v2.3**, con las specs de arquitectura de `engine_core` y `engine_sim` aprobadas y las decisiones Q-006 a Q-011; la v2.2 recogió las decisiones del autor previas al hito 0; la v2.1 se revisó tras la primera ronda de crítica, una revisión técnica independiente y las respuestas del autor

## Propósito del documento

Este documento recoge todo lo decidido hasta ahora sobre el proyecto, para que cualquier agente de IA (o yo mismo) pueda continuar sin perder contexto. Es la fuente de verdad provisional hasta que existan la constitución y las specs de arquitectura.

- **Estado:** GDD núcleo completo y revisado dos veces. Esqueleto del repositorio creado. La constitución está redactada en `specs/constitution.md` y ADR-0001 a ADR-0009 en `specs/adr/`; la tabla de reglas de este documento es su resumen. El plan de hitos cubre toda la fase A y esboza la fase B.
- **Origen de las mecánicas:** lo que describe los juegos originales se marca como **recordada** (el autor la tiene clara), **por verificar** (pendiente de comprobar jugando o con guías fiables) o **adoptada** (no se sabe si era así en el original, pero se queda porque gusta). Lo que no viene del original se marca como **propia**.
- **Marcas de versión:** lo añadido en cada revisión lleva la marca **(v2)**, **(v2.1)**, **(v2.2)** o **(v2.3)**. Las cuatro secciones siguientes lo resumen.
- **Regla para agentes:** no inventar mecánicas de los Imperivm originales. Si falta un detalle, se pregunta al autor. Las preguntas abiertas de la sección final las responde el autor; ningún agente las da por respondidas.

## Cambios en la v2

- La visión se divide en dos fases: **A, réplica** a la escala original, y **B, Plus Ultra**, con escala y mecánicas propias.
- Nuevo pilar 6: **la IA es el juego**. Nueva sección de inteligencia artificial y hito propio.
- El sistema héroe-ejército pasa de detalle pendiente a **decisión de diseño previa al hito 2**, con tres opciones sobre la mesa.
- Reclutamiento y veteranía se adaptan a la escala de la fase B.
- El **abastecimiento** se declara mecánica estrella de la fase B.
- La construcción urbana se orienta a **parcelas y plantillas**, para que la IA pueda jugarla.
- Nueva sección de **arte, assets, audio e interfaz**.
- Fuera de alcance explícito: multijugador y campaña.
- Decisiones técnicas cerradas: `wgpu`, tablas SoA, reglas de determinismo, estado canónico frente a derivado, flow fields por sectores, sistema de efectos acotado y herramientas con conocimiento del juego fuera de `engine/`.
- La constitución se presenta como tabla **regla → comprobación**, separando reglas de presupuestos.
- Escalera de hitos completa: del 0 al 7 en fase A y del 8 al 11 en fase B.

## Cambios en la v2.1

- Respuestas del autor incorporadas: máquina de referencia, ventajas de la IA, niebla de guerra, relación entre IAs, estrategia de assets y dedicación.
- Dos niveles de máquina: **referencia** (la del autor) y **mínima** (el objetivo real, más modesta), con el método para emular la mínima desde la referencia.
- Presupuestos reescritos tras la revisión técnica independiente (`specs/reviews/2026-10-25-revision-tecnica-independiente.md`): se mide en ticks por segundo y en milisegundos de GPU por pase, no en FPS ni en un factor de aceleración. El tick pasa a **20 Hz**.
- Conclusión de contra qué se diseña cada parte: **la simulación contra la CPU, el render contra la GPU**, y el **modelo de hilos** completo.
- Animación de multitudes: **esqueleto horneado con skinning en GPU e impostores** sustituyen a las vertex animation textures.
- **Sin ECS**: todas las entidades son tablas con un único tipo de handle. Decisión cerrada, no aplazada.
- Arquitectura ampliada con lo que faltaba: identidad y borrado, orden y desempate de eventos, tiempo entero, asignaciones, snapshots e interpolación en GPU, cadáveres, navegación jerárquica, guardado binario, streaming y frame pacing.
- Hitos 0, 1 y 2 reescritos para que midan lo que de verdad hay que medir; la infraestructura de motor (carga de assets, escena, entrada, interfaz, depuración) queda repartida por hitos.
- Constitución: reglas 4, 6 y 13 precisadas y tres reglas nuevas (tiempo entero, hash de datos, sin asignaciones en el camino caliente).
- El ADR de Bevy (ADR-0002) da la razón real y el coste asumido.
- Metodología: flujo multiagente con Claude Code (ADR-0008), planes en `specs/plans/` y bandeja de preguntas en `specs/preguntas-pendientes.md`. Las plantillas de spec llevan criterios `CA-n`, firmas concretas y presupuestos, que es lo que el flujo necesita.

## Cambios en la v2.2

- Decisiones del autor previas al hito 0, recogidas desde `specs/preguntas-pendientes.md` (Q-001 a Q-005): plataforma solo Windows en la fase A; puerta local en lugar de CI remota hasta que exista el remoto, y entonces GitHub Actions con runner Windows; hashers deterministas `rustc-hash` y xxh3 (ADR-0009); asignador contador como crate externo de desarrollo; datos RON en `data/`, separados de `assets/`; specs de arquitectura de `core` y `sim` antes del hito 0 y las de `render`, `rules` y `app` antes del hito 1.
- Flujo multiagente: retirado el hook de fin de tarea; la compilación del worktree al terminar la hace el orquestador (consecuencias de ADR-0008).

## Cambios en la v2.3

- Specs de arquitectura de `engine_core` y `engine_sim` aprobadas el 26 de octubre de 2026 (`specs/architecture/`), con sus decisiones de redacción confirmadas por el autor: handle de 8 bytes con generación `NonZeroU32`, aritmética de ticks con pánico explícito igual en dev y release, sobre de guardado con versión de cabecera y versión de cuerpo compuesta (16 bits del motor y 16 del juego), conjunto activo como prefijo denso de cada tabla, orden que se aplica en el tick siguiente al que se envía, replay que graba solo las órdenes externas y regenera la IA, y grid de vecinos reconstruido cada tick con tope de 2^20 celdas.
- Decisiones del autor Q-006 a Q-011 (`specs/preguntas-pendientes.md`): hash de datos de 64 bits; columnas SoA a bytes con la API segura de `bytemuck`; solo `engine_core` declara `rustc-hash`, `xxhash-rust` y `bytemuck`, y los demás usan sus reexports; varios números aleatorios de la misma tupla por un índice dentro del propósito, sin contador canónico; solo los `*.ron` de `data/` entran en el hash de datos; y `-C target-cpu` y `-C target-feature` prohibidos en el workspace hasta que el hito 2 mida, vigilados por `check-arch`.
- Plan P01 de `engine_core` en borrador (`specs/plans/01-engine-core.md`): diez tareas en serie, tres de ellas de contrato.
- Stack: `thiserror` para los errores de los crates y `allocation-counter` como contador de asignaciones en tests (Q-003, fijado en la spec de `engine_core`).

## Visión

Plus Ultra: Dominium es un RTS para uso personal inspirado en la saga Imperivm (I, II y III), en 3D al estilo de Imperivm III. Primero se replica lo más fielmente posible la experiencia original del modo libre; después se añaden mecánicas propias.

**Dos fases (v2):**

- **Fase A, réplica.** Reproducir el modo libre de Imperivm III a su escala original, con todos los números de economía y combate definidos como datos. Termina cuando una partida completa contra varias IAs es divertida, estable y se puede guardar y cargar.
- **Fase B, Plus Ultra.** Subir el dial de escala, añadir las mecánicas propias y las tres facciones nuevas. La arquitectura se diseña desde el día uno para la fase B, pero el contenido y el equilibrado se hacen primero para la fase A.

- **Tono:** Antigüedad estilizada, histórica con elementos míticos. La fidelidad histórica no es un requisito.
- **Ambición de escala:** ejércitos más grandes que en el original. Es el objetivo de la fase B, no de la A.
- **Autor:** programador con experiencia, ya ha hecho juegos en Godot y domina por igual Rust y C++. Todo el código se generará con IA. Dedicación alta.
- **Fuera de alcance (v2):** multijugador (el determinismo deja la puerta abierta, pero no se diseña para ello), campaña o escenarios con guion, y herramientas de modding públicas (el contenido es data-driven de todos modos).

**Nombre:** *Plus Ultra* ("más allá") es el nombre de la saga y *Dominium* ("dominio, soberanía") el de esta primera entrega. El lema invierte la advertencia de las Columnas de Hércules y conecta con crecer desde cero, con los atlantes, con Hispania y con la idea de partir de Imperivm para ir más allá.

## Pilares de diseño

1. **Todo está conectado.** La gracia está en cómo se mezclan economía, expansión, progresión y guerra. Ningún sistema funciona aislado.
2. **Crecer desde cero.** Se empieza con casi nada y la progresión se nota: mejorar la base, desbloquear tipos de unidad y elegir mejoras.
3. **Economía en tensión.** Oro, comida y población son interdependientes. Descuidar cualquiera tiene consecuencias.
4. **Carrera por el territorio.** Las aldeas neutrales son la fuente principal de recursos y población. Hay que llegar antes que el rival y luego mantenerlas.
5. **El ritmo lo marcas tú, pero la IA también.** Puedes construir con calma o acelerar cuando quieras, pero la IA puede adelantarse e ir a por ti.
6. **La IA es el juego (v2).** El único rival es la IA, así que cada mecánica pasa este filtro: ¿puede jugarla bien una IA que yo pueda programar y probar? Si la respuesta es no, se simplifica la mecánica, no se complica la IA.

## Bucle principal

1. **Inicio:** base mínima y pocas unidades. Explorar y empezar a reclutar.
2. **Expansión:** carrera por las aldeas neutrales para ganar oro, comida y población.
3. **Consolidación:** mejorar la base, desbloquear unidades y mejoras, y mantener las aldeas conquistadas.
4. **Escalada:** decidir cuándo atacar, o reaccionar si la IA golpea primero.
5. **Resolución:** conquista de los rivales.

Minuto a minuto: revisar recursos, decidir qué producir o mejorar, mover tropas para capturar o defender aldeas y reaccionar a la presión enemiga.

## Escala, presupuestos y configuración de partida

**Datos del original (recordados):** de 5 a 10 facciones por mapa; batallas de hasta unas 300 unidades simultáneas; partidas de 3 a 6 horas en mapas estándar con IA neutra de nivel medio, y más de 15 horas en mapas grandes con IA agresiva.

| Objetivo | Unidades por batalla | Unidades vivas en el mapa | Cuándo |
| --- | --- | --- | --- |
| Fase A, réplica | unas 300, como el original | las que dé la economía original | hitos 3 a 7 |
| Prueba de estrés (v2) | 3.000 luchando | 10.000 moviéndose | hito 2, contenido desechable |
| Fase B, aspiracional | 2.000 a 3.000 | 10.000 a 15.000 | hito 8 |

**Dos niveles de máquina (v2.1).** "144 FPS estables" es un presupuesto, no una regla, y además no describe la máquina objetivo. Se mide en milisegundos por pase y en ticks por segundo, en percentil 99 y nunca en FPS, sobre la máquina de **referencia**, que es la del autor. El objetivo real es una máquina **mínima** más modesta, y la referencia se usa para emularla.

| Nivel | Máquina | Objetivo |
| --- | --- | --- |
| Referencia (donde se mide) | Ryzen 5 5600X, 16 GB, RTX 3070 8 GB, Windows 10 | 6,9 ms por frame (144 Hz) a 1080p; simulación acelerada para pruebas |
| Mínima (objetivo real) | 4 núcleos / 8 hilos, GTX 1060 6 GB o equivalente, 8 GB RAM | 16,6 ms por frame (60 Hz) a 1080p; simulación en tiempo real |

**Contra qué se diseña cada parte (v2.1).** Con 15.000 unidades vivas y 3.000 en melé, un tick de simulación cuesta del orden de 4 a 8 ms en un solo hilo del 5600X, dominado por las consultas de vecinos (fallos de caché, no aritmética), y de 1 a 2 ms con cinco workers. En tiempo real sobra; donde la CPU manda es en la simulación sin gráficos, medida en ticks por segundo, y en los picos de navegación. La GPU de la mínima, con unas 4.000 unidades en pantalla, tres cascadas de sombras y terreno, anda en unos 11 ms por frame: 60 FPS sí, 144 no. La referencia la triplica. Por tanto: **la simulación se diseña contra la CPU** (caché e hilos: SoA, unidades reordenadas por celda, enteros para las cantidades de juego, cero asignaciones por tick) **y el render contra la GPU**, con cero trabajo por unidad y por frame en la CPU. La simulación publica un snapshot por tick, no por frame, y la interpolación la hace el vertex shader.

**Cómo se emula la mínima desde la referencia (v2.1):**

- **CPU.** Afinidad del proceso a 4 núcleos físicos, pool de 3 workers, plan de energía al 99 % para quitar el boost, y lo medido se multiplica por 1,3 por el IPC y el reloj de un cuatro núcleos de 2017.
- **GPU.** La 3070 rinde unas 3 veces una 1060, así que el presupuesto en la referencia es un tercio del objetivo. Se mide con timestamp queries por pase. El relleno se comprueba renderizando a 4K.
- **Memoria.** Asignador rastreado con tope de 2,5 GB de VRAM (hay GTX 1060 de 3 GB) y límite de 3 GB de working set, para que reviente pronto.

**Presupuestos (v2.1), percentil 99:**

| Métrica | Objetivo en la mínima | Equivalente en la referencia |
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

El presupuesto que manda es el de 300 unidades sin gráficos: una partida de fase A de cuatro horas son 288.000 ticks a 20 Hz, y para equilibrar con partidas IA contra IA hace falta que se jueguen en un minuto, no en cuarenta. Por eso el tick es de **20 Hz** y no 30: un tercio menos de ticks por hora, y el render lo esconde con la interpolación.

**Configuración de IA por facción,** con dos ejes independientes:

- **Carácter** (ofensivo, defensivo o neutro): define *qué* decide la IA, como cuándo atacar o cuánto expandirse.
- **Dificultad** (fácil, moderada o difícil): define *lo bien* que lo ejecuta y, además, **bonificaciones de recursos** (v2.1, decidido): multiplicadores de ingresos de oro y comida y descuentos de reclutamiento, definidos por perfil como datos. La IA no recibe ventajas de información distintas de las que da su modo de percepción (ver sección de IA).

Son 9 perfiles definidos como datos, no como código. Las decisiones estratégicas de cada facción se reparten en el tiempo (no todas piensan en el mismo tick) para que 10 IAs no disparen el coste.

**Implicaciones:** guardar y cargar es un sistema crítico por la duración de las partidas; el rendimiento en la hora 10 debe ser igual que en el minuto 1; y la simulación debe poder ejecutarse sin gráficos y acelerada, para pruebas de resistencia y partidas IA contra IA de equilibrado.

## Facciones

El juego tendrá 11 facciones: las 8 de Imperivm III más griegos, atlantes y visigodos. Cada facción tiene una unidad especial propia. Los rasgos generales son **recordados**; las unidades especiales y sus habilidades salen de un resumen generado por IA y están **por verificar**.

| Facción | Rasgos generales | Unidad especial | Habilidad (por verificar) |
| --- | --- | --- | --- |
| Roma Republicana | Unidades más baratas con atributos algo inferiores a la imperial | Tribuno | Mucha vida y daño de penetración para romper líneas blindadas |
| Roma Imperial | Ejército profesional de élite, más caro (vélites, pretorianos, liberatus) | Pretoriano | Devuelve parte del daño cuerpo a cuerpo recibido |
| Egipto | Control del mapa y unidades míticas o especializadas (sacerdotes, guardianes del Nilo, carros de Osiris) | Guardián del Nilo | Inmunidad temporal al inicio del combate (lo de los 10 impactos parece inventado) |
| Cartago | Economía monetaria: mercenarios en vez de población, oro al perder unidades o vender población | Elefante de guerra | Muy caro, daño en área y salud descomunal |
| Germania | Combatientes y jinetes/arqueros teutones agresivos, cazadoras y valkirias | Valkiria | Potencia el fervor del ejército y devasta infantería ligera |
| Britania | Guerreros feroces, arqueros, lanceros, caledonios y highlanders | Highlander | Ignora parte de la defensa enemiga al luchar en inferioridad |
| Hispania | Milicianos, arqueros con vista de lince, honderos, jinetes y sacerdotisas | Guardia de élite | Defensa extrema por un coste mucho menor |
| Galia | Guerreros, hacheros, lanceros, guerreras, guerreros de Fand y druidas | Guerrero de Fand | Recupera salud al matar enemigos |
| Griegos | Nueva, por diseñar (fase B) | Por diseñar | Por diseñar |
| Atlantes | Nueva, por diseñar (fase B) | Por diseñar | Por diseñar |
| Visigodos | Nueva, por diseñar (fase B) | Por diseñar | Por diseñar |

Otras unidades citadas: los **carros de Osiris** (Egipto) envenenan con sus flechas, y según una guía el veneno desaparece si la unidad recupera más del 90% de salud; los **nobles** (Cartago) tienen una habilidad al rematar enemigos cuyas fuentes se contradicen (oro frente a asegurar una unidad).

**Principios de diseño de facciones:**

- Cada facción necesita identidad clara: unidades únicas, héroes propios, estilo de juego reconocible y quizá una mecánica exclusiva.
- Las facciones se definen como datos. Las dos Romas muestran que debe poder haber variantes que comparten base y cambian parámetros.
- Cartago es la prueba de fuego de la arquitectura: su economía debe poder expresarse sin parches, con reglas de reclutamiento y efectos de eventos configurables por facción.
- Incorporación progresiva: una facción estándar en el corte vertical (hito 3); Cartago es la segunda en entrar (hito 6) para validar el sistema de datos cuanto antes.
- **(v2)** Todas las facciones humanas comparten esqueleto y conjunto de animaciones; se diferencian por malla, textura, props y unidades especiales. Ver la sección de arte.

## Mecánicas

**Originales a conservar:** edificios y sus mejoras, aldeas neutrales conquistables y su mantenimiento, economía de oro, comida y población, héroes con experiencia y habilidades, unidades de apoyo como sanadores, y comercio.

**Unidades (recordado):** cada soldado se recluta de uno en uno, con coste en oro y población y mantenimiento en comida. Cada unidad tiene nivel propio, que mejora sus stats de combate. Por eso cada soldado es una entidad individual en la simulación; no hay escuadras agregadas.

- **(v2) Reclutamiento a escala.** En fase A el reclutamiento sigue el modelo original. En fase B la interfaz debe admitir colas largas o reclutamiento continuo por edificio; la simulación sigue creando soldados individuales.
- **(v2) Veteranía a escala.** El nivel individual se mantiene en la simulación en ambas fases. En fase B la interfaz lo agrega por ejército (nivel medio, distribución, insignias), porque a miles de unidades el dato individual es invisible.

**Héroes (recordado):** en el original cada héroe dirigía como máximo 50 unidades.

**(v2) El sistema héroe-ejército es el esquema de control del juego, no un detalle.** Con soldados individuales y sin escuadras, el héroe como "asa" del ejército es la respuesta de Imperivm a cómo se dan órdenes a muchas unidades. Condiciona la interfaz, las formaciones y la IA, así que se decide antes del hito 2. Opciones:

| Opción | Descripción | A favor | En contra |
| --- | --- | --- | --- |
| a) Tope que crece con el nivel | El héroe manda N unidades; N sube con su nivel | Fiel, simple, la IA lo entiende | No llega solo a miles de unidades |
| b) Jerarquía héroe → oficiales → unidades | Cada oficial es una entidad con su propio tope; el héroe manda oficiales | Escala sin perder el asa; permite órdenes por bloque | Más interfaz y más IA táctica |
| c) Sin tope, con radio de mando | Fuera del radio del héroe las unidades pierden moral o eficacia | Muy simple de simular | Pierde la decisión de composición; difícil de leer |

Recomendación de la revisión: **a) en fase A** por fidelidad, **b) en fase B** construida sobre a). Decisión del autor pendiente.

**Sistema de efectos y modificadores definido por datos.** Las habilidades especiales exigen daño reflejado, penetración, inmunidad por impactos, daño en área, curación al matar, auras, bonus condicionales, venenos con condición de cura y efectos económicos al rematar. Cada habilidad se compone de *disparadores* (al recibir golpe, al matar, al atacar, mientras se cumpla una condición) y *efectos* (modificar daño, curar, aplicar estado, generar recursos, potenciar aliados en un radio). Las habilidades nuevas se añaden en RON sin tocar código.

- **(v2) Acotado.** El sistema cubre exactamente los disparadores y efectos que salen de la tabla de facciones. No hay lenguaje de scripting. Si una habilidad nueva no cabe, se amplía la lista de efectos en código, con su spec.

**Niebla de guerra (v2.1, recomendación de la revisión, pendiente de confirmar por el autor).** Al autor le gustaba jugar sin niebla. Propuesta: la simulación mantiene desde el hito 3 un grid de visibilidad por equipo, barato (cada unidad y edificio sella su radio de visión cada pocos ticks; coste lineal y un byte por celda y equipo). Para el jugador la niebla es una **opción de partida**, desactivada por defecto, que solo afecta a render e interfaz: la simulación es idéntica con y sin ella. Para la IA, ese grid alimenta el modo de percepción *limitada* cuando se quiera. Así la niebla queda disponible para equilibrar sin obligar a nadie a jugarla, y los replays y las partidas IA contra IA no dependen de ella. Consecuencia: sin niebla, el paso "explorar" del bucle principal pasa a ser "llegar antes que el rival".

**Mecánicas propias (fase B):**

1. **Abastecimiento (v2, mecánica estrella de la fase B).** El autor lo cita entre las mecánicas del original; el detalle está **por verificar**. En fase B se amplía como contrapeso natural a los ejércitos enormes: un ejército lejos de aldeas o ciudades propias consume reservas, y sin reservas sufre penalizaciones y acaba desertando. Hace que la escala sea interesante y no solo grande, y da a la IA defensiva una herramienta real.
2. **Construcción urbana por parcelas (v2, revisada).** Dentro de las murallas hay parcelas predefinidas; cada parcela admite un conjunto de edificios y la ciudad crece añadiendo anillos de muralla con nuevas parcelas. La IA construye con plantillas de ciudad por facción y fase de partida. La colocación totalmente libre queda descartada salvo que la IA demuestre poder con ella. Siguen haciendo falta navegación dinámica, puertas con pasabilidad por facción y asedio de murallas.
3. **Jerarquía héroe-oficiales**, si se elige la opción b).
4. **Veteranía agregada en la interfaz** y **reclutamiento por colas**, descritos arriba.

El detalle de cada mecánica (abastecimiento, mantenimiento de aldeas, progresión de héroes) se escribirá *just in time*, cuando toque implementarla, y para las originales solo después de la sesión de verificación (ver metodología). Pendiente: qué significaba exactamente "mantener" una aldea en el original.

## Inteligencia artificial (v2)

La IA es el único rival y, por tanto, la mitad del juego. Tiene sección propia, hito propio y criterios de aceptación medibles.

- **Mismas reglas, misma API.** La IA juega con la misma API de órdenes que el jugador: el crate de IA solo lee estado y emite órdenes a la simulación. Esto hace la IA determinista, grabable en replays y comprobable sin gráficos.
- **Tres capas.** Estratégica (utility AI: expandir, construir, reclutar, cuándo atacar y a quién), operativa (ejércitos: composición, destino, retirada) y táctica (combate: formaciones, objetivos, uso de habilidades). Cada capa tiene su propio presupuesto de tiempo por tick y sus decisiones se reparten entre ticks. Ese reparto es estado canónico: se guarda y se reproduce.
- **Perfiles como datos.** Los 9 perfiles de carácter y dificultad son archivos de datos que ajustan pesos de utilidad, tiempos de reacción, calidad de ejecución y bonificaciones de recursos.
- **Ciudad por plantillas.** La IA construye su ciudad siguiendo plantillas por facción y fase de partida. Es la razón de que la construcción urbana vaya por parcelas.
- **Rivales entre sí (v2.1, decidido).** Las IAs también se atacan entre ellas. La capa estratégica elige objetivo entre todos los rivales por utilidad: distancia, fuerza relativa, fricción fronteriza por aldeas disputadas y agravios recientes. Un término de reparto evita que todas se vuelquen sobre el jugador. Sin alianzas ni diplomacia formal en fase A.
- **Percepción (v2.1).** La IA lee el estado a través de una interfaz de percepción con dos modos: *completa* (ve todo el mapa; por defecto en fase A, coherente con que reciba recursos extra) y *limitada* (solo lo visible para su equipo; futuro, para un modo sin ventajas). Es un flag por perfil, no un cambio de código.
- **Pruebas.** Partidas IA contra IA sin gráficos y aceleradas, con métricas: tiempo hasta la primera aldea, tiempo hasta el primer ataque, tasa de victoria por perfil y facción, y uso de recursos. Replays dorados como pruebas de regresión: un cambio que altera el resultado de un replay tiene que justificarse.
- **Aceptación (hito 4).** Criterios medibles del estilo "una IA moderada neutra gana a un jugador que no hace nada", "una IA difícil ofensiva ataca antes del minuto X" o "la tasa de victoria entre perfiles iguales con facciones distintas está dentro de un margen". Los números concretos se fijan con los datos del corte vertical.

## Arte, assets, audio e interfaz (v2)

El origen de los assets 3D es el mayor cuello de botella del proyecto, así que la dirección de arte se decide ahora, no después.

- **Dirección de arte.** Low poly estilizado, siluetas claras y paleta por facción. La iluminación, las sombras y la escala de los ejércitos hacen el trabajo que no hace el detalle de malla. Es el único estilo que un programador sin artista puede sostener para 11 facciones.
- **Rig humanoide compartido.** Un esqueleto de 24 a 32 huesos y un conjunto de animaciones para todas las unidades humanas; la variación viene de malla, textura y props. Caballos, elefantes y carros tienen rigs propios, pocos y reutilizados.
- **Animación de multitudes (v2.1, decidido).** Esqueleto horneado y skinning en GPU, no vertex animation textures. Los clips del glTF se muestrean al cargar a un buffer de matrices por hueso y frame; todas las facciones humanas comparten ese set, que ocupa del orden de medio megabyte. Cada instancia lleva clip, frame y un desfase de fase; el vertex shader lee cuatro huesos y mezcla dos frames. Armas, escudos y estandartes son mallas instanciadas colgadas de un hueso, sin coste de pipeline. LOD0 con cuatro influencias por vértice, LOD1 y LOD2 con una o dos, y LOD3 como impostor (billboard con pocos ángulos y un ciclo corto), que es donde las miles de unidades lejanas dejan de costar. Héroes y tropa van por el mismo camino. Las vertex animation textures habrían costado cientos de megabytes de VRAM y un rehorneado por cada malla o arma nueva.
- **Pipeline.** Modelos en glTF con skin y clips; al cargar se muestrean a buffers. En `engine/tools`: inspector de assets y horneado de impostores. No hace falta herramienta de horneado de animación.
- **Origen de los assets (v2.1).** Hay algo de presupuesto y el autor puede generar y modelar. Estrategia híbrida propuesta: comprar un pack base low poly estilizado de cuerpos humanoides, armas y edificios de la Antigüedad como punto de partida del rig compartido; animaciones compradas o de bibliotecas gratuitas retargeteadas a ese rig; generación con IA para props, texturas y edificios estáticos, donde la consistencia del rig no importa; modelado propio para unidades especiales y héroes. El pack base y el rig se eligen juntos al inicio del hito 5.
- **Audio.** Mínimo viable: efectos de combate, respuestas de unidad y música ambiental. Librería candidata: `kira`. Fuera del camino crítico hasta el hito 4.
- **Interfaz.** `egui` para herramientas y para el HUD de los primeros hitos. La interfaz definitiva de juego es una decisión pendiente que no bloquea nada antes del hito 5.
- **Pantalla de configuración de partida.** Mapa, facción propia, número de rivales y carácter y dificultad de cada uno. Entra en el hito 3 en versión mínima y en el hito 4 completa.

## Modo de juego

**Modo libre** como único modo. La partida se configura eligiendo el mapa (estándar, grandes y el de Europa), la facción propia y el número de rivales (de 5 a 10, según el mapa) con el carácter y la dificultad de cada uno.

Como el modo libre es todo el juego, los mapas son el contenido principal. Pendiente: condiciones de victoria y si los mapas se crean a mano con editor propio, proceduralmente o con una mezcla.

## Decisiones técnicas y organización

- **Motor propio ad hoc en Rust,** diseñado solo para este juego. Nada genérico "por si acaso". Se eligió Rust frente a C++ porque su compilador detecta errores que el código generado por IA podría introducir en silencio, y por la paralelización sencilla con `rayon`.
- **(v2.1) Por qué no Bevy, con la razón real.** Con código generado por IA lo que manda es el bucle del agente: tiempo de compilación y deriva de la API entre versiones. Que su renderer sea difícil de doblegar a multitudes GPU-driven es secundario. Y hay que asumir el coste de no usarlo: carga de assets, ciclo de vida de la escena, entrada, integración de interfaz y herramientas de depuración son alrededor de un tercio del motor, y ahora están repartidos por hitos.
- **(v2) `wgpu`, no `ash`, con deberes (v2.1).** Con `ash` habría que escribir un backend Vulkan completo (sincronización, memoria, descriptores) antes de ver un soldado. `wgpu` ofrece compute, storage buffers e indirect multi-draw, suficiente para culling en GPU e instancing con skinning. Deberes: versión fijada, que se sube una vez por hito; las features nativas que se necesitan (`MULTI_DRAW_INDIRECT_COUNT`, `INDIRECT_FIRST_INSTANCE`, `TEXTURE_BINDING_ARRAY`) se prueban en el hito 1 en la 3070 y en WARP; anillo propio de staging mapeado en vez de `write_buffer` por frame; y como WARP, el rasterizador por software de Windows, es lento y da píxeles distintos al hardware, las golden images se comparan WARP contra WARP, con tolerancia, en escenas pequeñas.
- **(v2.1) Sin ECS.** Soldados, héroes, edificios y proyectiles son tablas SoA explícitas con el mismo tipo de handle. Las relaciones héroe-soldado, edificio-cola y proyectil-objetivo se fijan en el hito 1, así que la decisión no se podía aplazar. Un ECS habría añadido un segundo modelo de identidad y de serialización para nada.
- **Prioridades:** rendimiento, buen aspecto visual y cero parones perceptibles, medidos como percentil 99 del frame.
- **Se reutilizan librerías** para lo que no hace único al juego (ventanas, carga de modelos, UI de herramientas). El esfuerzo propio va a la simulación de ejércitos, el render de tropas, el terreno y la IA.
- **Un solo repositorio** con workspace de cargo. Motor y juego se desarrollan juntos: el juego dicta qué necesita el motor.
- **(v2) Ciclo con agentes.** El cuello de botella real de un flujo con agentes es compilar y probar. Crates pequeños, enlazador rápido, `cargo check` en el bucle interno y tests que corren sin GPU.

```
plus-ultra/
├── engine/
│   ├── core/      # matemáticas, tiempo en ticks, handles, serialización, perfilado
│   ├── sim/       # tablas SoA, grid espacial, navegación, scheduler determinista, eventos
│   ├── render/    # wgpu, instancing, skinning en GPU, terreno, culling y LOD en GPU
│   └── tools/     # inspector de assets, horneado de impostores; sin conceptos de juego
├── game/
│   ├── rules/     # unidades, facciones, economía, habilidades, IA; sin gráficos
│   ├── app/       # ventana, cámara, entrada, HUD, pantallas
│   └── tools/     # editor de mapas, visor de partidas IA contra IA (v2)
├── xtask/         # comprobaciones de arquitectura: cargo xtask check-arch (v2.1)
├── .claude/       # flujo multiagente: subagentes, skills de comandos y hooks (v2.1)
├── specs/
│   ├── constitution.md
│   ├── adr/       # (v2)
│   ├── gdd/
│   ├── architecture/
│   ├── engine/
│   ├── game/
│   ├── plans/     # un plan con sus tareas por cada spec implementada (v2.1)
│   ├── reviews/   # (v2.1)
│   └── preguntas-pendientes.md   # bandeja de dudas para el autor (v2.1)
├── data/      # contenido en RON, editado a mano y sin LFS; en el hito 0, el escenario sintético (v2.2)
└── assets/
```

**Regla de oro:** las dependencias van en una sola dirección. El juego depende del motor y nunca al revés; ningún crate de `engine/` sabe qué es un legionario, una aldea o la comida. Por eso el editor de mapas, que coloca aldeas, vive en `game/tools` y no en `engine/tools`. Si algún día el motor se reutiliza, separar `engine/` en otro repo será trivial.

## Arquitectura del motor

- **Simulación separada del render.** Tick fijo de **20 Hz (v2.1)** con acumulador; el render interpola en la GPU entre los dos últimos snapshots, así los FPS no dependen del coste de la lógica.
- **Hilos (v2.1).** *Principal:* ventana, entrada, interfaz y codificación de render; con render GPU-driven son unas decenas de draws indirectos, y un hilo de render aparte solo añadiría sincronización. *Simulación:* posee el mundo, avanza a tick fijo, recibe las órdenes por un canal selladas con el tick en que se aplican (esa cola es el replay) y publica snapshots. *Entrada/salida:* assets y guardado, desde una copia del estado canónico de unos 3 MB. *Pool de `rayon`:* núcleos físicos menos uno (5 en la referencia, 3 en la mínima), usado solo por la simulación; el SMT se valida en el hito 2.
- **Datos en SoA** (structure of arrays): posiciones, salud, estado y objetivo en vectores separados, en tablas explícitas para todas las entidades.
- **Identidad y borrado (v2.1).** Handle = slot + generación, común a todas las tablas. Tablas densas con swap-remove e indirección handle→slot y slot→handle. El borrado se difiere al final del tick y se aplica por slot descendente, nunca a mitad de una fase. Conjunto activo: las unidades de guarnición no pasan por combate ni por separación.
- **Fases y eventos (v2.1).** Cada fase es un map sobre entrada congelada con salida por slot. Los eventos se escriben en buffers por chunk de tamaño fijo (`par_chunks(512)`, para que el número de chunks no dependa de los hilos), se concatenan en orden de chunk, se ordenan de forma estable por (objetivo, origen, tipo) y se aplican en una fase secuencial: daño, muertes, borrados. Los desempates (dos matan al mismo en el mismo tick: crédito, experiencia, oro cartaginés) salen de ese orden y se fijan en la spec de combate.
- **Determinismo en la práctica (v2, ampliado v2.1).** Aleatoriedad sin estado: `hash(semilla, tick, handle, propósito)`. Grid espacial con orden estable (celda, slot), así el orden de los vecinos y cualquier suma de flotantes es fijo. Sin `HashMap` ni `HashSet` con el hasher por defecto en la simulación: `rustc-hash` para colecciones y xxh3 para el hash de estado y la aleatoriedad sin estado (ADR-0009, v2.2). Map paralelo sí; reducciones paralelas de flotantes no. Objetivo: determinismo en la misma máquina con cualquier número de hilos; entre plataformas no es requisito. Hash del estado cada N ticks también en release, y un buscador de desincronizaciones que hashea por sistema. Para que los hashes dorados valgan también en el runner de CI, sin `-C target-cpu` ni `-C target-feature` en el workspace hasta que el hito 2 mida, vigilado por `check-arch` (Q-011, v2.3).
- **Tiempo entero (v2.1).** El tick es un `u64` y las duraciones se miden en ticks; nunca segundos en flotante dentro de `sim` ni de `rules`.
- **Asignaciones (v2.1).** Cero asignaciones en el camino caliente: arenas por hilo para vecinos, eventos y scratch de navegación; snapshots reciclados; `collect` prohibido en las fases por unidad. Un asignador contador lo comprueba en tests: un crate externo como dependencia de desarrollo (candidato `allocation-counter`, que se verifica y fija en la spec de la simulación sintética), nunca un `GlobalAlloc` propio, que exigiría `unsafe` contra la regla 3 (v2.2).
- **Estado canónico y derivado (v2, precisado v2.1).** Todo lo que influye en la simulación es canónico y se serializa: colas de órdenes, formaciones, peticiones de navegación pendientes, cooldowns, temporizadores de la IA, proyectiles y el hash de los datos con los que se juega; la aleatoriedad no tiene estado que guardar, porque es `hash(semilla, tick, handle, propósito)` con un índice en el propósito (ADR-0009, v2.2). Lo derivado es función pura del canónico y se reconstruye completo y determinista al cargar; si un cálculo es incremental (un flow field a medias), su progreso es canónico. Guardado binario versionado (`postcard`); RON solo para autoría. Hot-reload solo en builds de desarrollo, porque cambiar los datos invalida los replays.
- **Snapshots e interpolación (v2.1).** Anillo de cuatro buffers preasignados; la simulación escribe en uno libre y el render retiene el anterior y el actual, sube dos storage buffers y un alfa, y el vertex shader interpola. Unos 300 KB por tick a escala de fase B. El tiempo de render va un tick y un pequeño margen por detrás del de simulación, suavizado; sin eso hay stutter con FPS altos. Las unidades se casan por handle; las que nacen no se interpolan; las que mueren salen de la tabla viva, así que el render mantiene una lista derivada de cadáveres (handle, posición, clip, tick) alimentada por los eventos de muerte.
- **Grid espacial** para vecinos, combate, separación y selección; las unidades se reordenan por celda cada N ticks para la caché.
- **Navegación (v2, ampliado v2.1).** Flow fields por sectores con portales y campo de integración solo por sector; para rutas largas, un segundo nivel jerárquico y caché de rutas. Pasabilidad por facción como capa de "dueño de la puerta" por celda, no un grid por facción. Caché LRU de campos por grupo con tope de memoria. Coste por densidad para que un portal estrecho no produzca oscilación ni apelotonamiento. Las formaciones son entidades con su propio camino y sus slots, a los que los soldados llegan con *arrive*, con embudo en los portales más estrechos que la formación. Recálculo incremental cuando una muralla cambia el grid, con presupuesto por tick y el resto diferido.
- **Render GPU-driven:** instancing, skinning en GPU con esqueleto horneado, culling y LOD en GPU, impostores en el último LOD. Cada unidad es un modelo 3D individual con su animación; el instancing solo cambia cómo se envía a la GPU.
- **Terreno** por chunks con heightmap y sombras en cascada; streaming de chunks desde el hilo de entrada/salida con tope de subida por frame.
- **Frame pacing (v2.1).** Modo de presentación (Mailbox frente a Fifo) decidido midiendo; resolución del temporizador del sistema ajustada en Windows; y el desfase simulación-render descrito arriba.
- **Contenido data-driven** en RON con hot-reload en desarrollo: unidades, facciones, héroes, edificios, habilidades, plantillas de ciudad y perfiles de IA. Vive en `data/`, separado de `assets/` (v2.2). Solo los archivos `*.ron` entran en el hash de datos de guardados y replays (Q-010, v2.3).
- **IA estratégica** (utility AI) separada de la táctica, con decisiones repartidas entre ticks y siempre a través de la API de órdenes.

**Stack:** `wgpu`, `winit`, `glam`, `rayon`, `egui`, `gltf`, `meshopt`, `serde` con `ron` (autoría) y `postcard` (guardado), `rustc-hash` y `xxhash-rust` con xxh3 como hashers deterministas y `bytemuck` para ver columnas SoA como bytes sin `unsafe` (ADR-0009), `thiserror` para los errores, `kira` (candidato), `tracy-client`, `criterion`, `proptest`, `allocation-counter` (solo en tests, v2.3) y `cargo deny`.

## Constitución: reglas y cómo se comprueban

La constitución está redactada en `specs/constitution.md`, con el motivo de cada regla, su comprobación, el procedimiento de cambio y el estado de las comprobaciones; esa versión manda si difieren. Esta tabla es el resumen. Ningún agente puede romper estas reglas. **(v2)** Lo que el compilador pueda imponer, lo impone el compilador. Los presupuestos de rendimiento no son reglas y están en la sección de escala.

| # | Regla | Cómo se comprueba |
| --- | --- | --- |
| 1 | Las dependencias van de `game/` a `engine/`, nunca al revés | `cargo xtask check-arch` lee el workspace |
| 2 | `sim` y `game/rules` no conocen ni tocan gráficos ni ventana | El mismo: sin dependencia transitiva de `render`, `wgpu`, `winit` ni `egui` |
| 3 | No hay `unsafe` fuera de `render` | `#![forbid(unsafe_code)]` en cada crate salvo `render`; `check-arch` lo verifica |
| 4 | La simulación corre a tick fijo de 20 Hz y el render interpola | Test: la sim solo avanza en ticks enteros; revisión de spec |
| 5 | Datos en SoA en el código caliente | Revisión de código; no automatizable |
| 6 | Todo lo que influye en la simulación es canónico y serializable; lo derivado es función pura del canónico y se reconstruye completo al cargar | Test: guardar, cargar, avanzar N ticks y obtener el mismo hash que sin guardar |
| 7 | Los sistemas de `sim` son paralelizables: sin estado global compartido | Lints contra `static mut` y globales; revisión |
| 8 | El render no tiene trabajo por unidad en la CPU más allá de la copia del snapshot: culling, LOD e interpolación en GPU | Contador de draw calls por frame, independiente del número de unidades |
| 9 | Nada de algoritmos cuadráticos sobre el total de unidades; todo pasa por el grid espacial | Contadores deterministas: comprobaciones de vecinos por tick crecen linealmente con N |
| 10 | El grid de navegación admite cambios en tiempo de ejecución y pasabilidad por facción | Tests de navegación con muros añadidos en caliente |
| 11 | Unidades, facciones, habilidades, plantillas de ciudad y perfiles de IA se definen por datos | Test que carga todos los archivos de datos del repo |
| 12 | Presupuestos de rendimiento medibles y con umbral | Benchmarks con umbral en la máquina de referencia; contadores y ticks por segundo en CI |
| 13 | La simulación es determinista con cualquier número de hilos: mismos inputs, mismo resultado | Misma semilla con pool de 1, 3, 6 y 12 hilos, mismo hash por tick; hasher fijo; chunks de tamaño fijo; sin reducciones paralelas de flotantes |
| 14 | La simulación puede ejecutarse sin gráficos y acelerada | Binario headless en CI que juega una partida corta IA contra IA y mide ticks por segundo |
| 15 (v2) | La IA juega con la misma API de órdenes que el jugador | Revisión: el crate de IA solo emite órdenes y no muta estado |
| 16 (v2) | Las herramientas que conocen conceptos de juego viven en `game/tools` | El test de dependencias de la regla 1 |
| 17 (v2.1) | El tiempo de la simulación se mide en ticks enteros; sin segundos en flotante en `sim` ni en `rules` | Un único tipo `Tick(u64)`; lint o búsqueda contra flotantes con nombre de tiempo |
| 18 (v2.1) | Guardados y replays llevan el hash de los datos con los que se jugó; el hot-reload solo existe en builds de desarrollo | Test que rechaza cargar con hash distinto; hot-reload tras `cfg` de desarrollo |
| 19 (v2.1) | Cero asignaciones de memoria en el camino caliente de la simulación | Asignador contador en tests: cero asignaciones por tick tras el calentamiento |

**Presupuestos (no reglas):** la tabla de la sección de escala. El cronómetro vale en local, en la máquina de referencia; en CI mandan los contadores y los ticks por segundo con tolerancia.

## Metodología: Spec Driven Development

Todo el código se genera con IA siguiendo SDD: spec, plan, tareas, implementación y verificación. Las specs viven en el repo, versionadas con el código.

- **Constitución:** reglas inviolables (sección anterior).
- **ADR (v2):** una nota corta por decisión técnica cerrada, con contexto, opciones y consecuencias. Escritos ADR-0001 a ADR-0009 en `specs/adr/`.
- **Arquitectura:** una spec por crate con responsabilidad, API pública con firmas concretas (el contrato), dependencias permitidas, presupuestos y criterios de aceptación `CA-n`.
- **Features:** una spec por sistema con requisitos, contratos, criterios de aceptación `CA-n` medibles, casos límite y presupuestos.
- **Planes y preguntas (v2.1):** cada spec que se implementa tiene su plan con tareas en `specs/plans/`; las dudas que surgen al trabajar van a `specs/preguntas-pendientes.md` y solo las responde el autor.
- **Verificación:** reglas impuestas por herramientas (cargo, clippy estricto, `cargo deny`, `cargo xtask check-arch`); benchmarks con `criterion`; tests de propiedades (nadie dentro de un muro, salud nunca negativa, grid consistente) y de determinismo en la simulación; golden images WARP contra WARP y revisión visual en el render.
- **Revisiones (v2.1):** las revisiones externas o independientes se guardan en `specs/reviews/` como registro. No son fuente de verdad; lo adoptado se refleja aquí y en los ADR.
- **Contexto para agentes:** `CLAUDE.md` en la raíz y uno por crate cuando exista su spec de arquitectura.
- **Herramienta (v2, decidida en v2.1):** Claude Code con un flujo multiagente propio, descrito en la sección "Flujo multiagente" de `CLAUDE.md` y en `.claude/README.md`, y justificado en ADR-0008. La sesión principal es el orquestador y la única que escribe specs; siete comandos cubren el ciclo (`/spec`, `/planificar`, `/tareas`, `/implementar`, `/verificar`, `/integrar`, más `/estado`); ocho subagentes separan a quien escribe los tests de aceptación de quien implementa y revisan sin anclaje; cada tarea va en su worktree y se integra en serie. Para el hito 0 hay un modo ligero. Spec Kit se descartó por añadir ceremonia ajena al proyecto.
- **Mecánicas originales:** las escribe el autor de memoria y se marcan con su origen. La IA no las inventa.
- **Sesión de verificación (v2):** antes de escribir las specs de features del hito 3, el autor dedica una sesión a jugar Imperivm III (está en Steam como edición HD, por confirmar) con la tabla de facciones y la lista de pendientes delante: habilidades de unidades especiales, qué era mantener una aldea, abastecimiento, tope de héroes, niebla de guerra.
- **GDD *just in time*:** el núcleo está hecho; el detalle de cada mecánica se escribe al llegar a ella.

## Plan de implementación (v2, reescrito en v2.1)

Cada hito retira un riesgo concreto y tiene criterios de aceptación comprobables. Las decisiones de arquitectura pensadas para escalar se mantienen desde el principio, pero no se optimiza nada antes del hito 2.

**Infraestructura de motor repartida (v2.1).** Carga de assets glTF y texturas en los hitos 1 y 2; mapeo de entrada e integración de `egui` en el hito 1; visor de replays y tests de propiedades en el hito 1; buscador de desincronizaciones, `tracy` y asignador contador en el hito 0; inspector en juego en el hito 2; ciclo de vida de la escena y pantalla de configuración en el hito 3. Es alrededor de un tercio del motor y no se da por hecho.

### Fase A: réplica

**Hito 0 · Cimientos.** Riesgo que retira: una base de simulación que no sea determinista, que asigne memoria o que no escale en hilos, descubierta cuando ya hay juego encima.
- Repositorio y workspace (hecho), constitución redactada, ADR iniciales y specs de arquitectura de `engine_core` y `engine_sim`; las de `render`, `rules` y `app` van antes del hito 1 (v2.2). Puerta local con `check-arch`, clippy, tests y `cargo deny` antes de cada integración; CI remota en GitHub Actions con runner Windows cuando exista el remoto (v2.2).
- Simulación sintética: una tabla de 10.000 unidades con paseo aleatorio, fases paralelas, pipeline de eventos, swap-remove con handles de generación y hash de estado por tick.
- Aquí se fijan: el tipo de handle, el tiempo entero, el orden de fases, el pipeline de eventos, el formato de replay (semilla, hash de datos y órdenes selladas por tick) y el buscador de desincronizaciones que hashea por sistema. Carga de RON con hot-reload solo en desarrollo. `tracy` conectado.
- Aceptación: `cargo xtask check-arch` y `cargo test` pasan; la puerta de comprobaciones falla si se rompe una regla; el binario sin gráficos avanza 10.000 ticks con 10.000 unidades sintéticas y da el mismo hash con pool de 1, 3, 6 y 12 hilos; cero asignaciones por tick tras el calentamiento.

**Hito 1 · Primer combate.** Riesgo que retira: el bucle de simulación, render y entrada, y las features nativas de `wgpu`.
- Mapa pequeño con terreno, cámara estilo Imperivm III, 2 facciones con 20 soldados cada una, selección por caja, órdenes de mover y atacar, combate cuerpo a cuerpo básico hasta que un bando gana.
- Spike de `wgpu`: indirect multi-draw, arrays de texturas y timestamp queries, probados en la 3070 y en WARP. Interpolación en GPU con el anillo de snapshots. Cadáveres y nacimientos. Unidades como cápsulas por el camino de instancing.
- Visor de replays y primeros tests de propiedades.
- Aceptación: lo anterior, más guardar en mitad del combate, cargar y continuar 1.000 ticks con otro número de hilos obteniendo el mismo hash que sin guardar.

**Hito 2 · Prueba de estrés.** Riesgo que retira: una arquitectura que no cumple los presupuestos, descubierta tarde. Contenido desechable, arquitectura permanente.
- Malla humanoide placeholder con rig, 3 LODs, impostor si cabe y sombras en cascada, para que la medición de GPU signifique algo. Flow fields por sectores, grid espacial, culling y LOD en GPU, 10 IAs stub con presupuesto por tick.
- Escenarios: marcha de 10.000, choque de 1.500 contra 1.500, 3.000 por una puerta de 4 celdas, muro añadido en caliente.
- Medición en la referencia y emulando la mínima (afinidad a 4 núcleos, pool de 3, un tercio de GPU): ticks por segundo con 15.000 y con 300 unidades, guardar y cargar con 10.000, contadores de RAM y VRAM, y una carrera sin gráficos de 500.000 ticks con memoria y tiempo de tick planos.
- Se deciden: factor de aceleración definitivo, SMT, modo de presentación y sistema héroe-ejército.
- Aceptación: la tabla de presupuestos se cumple en los dos niveles; la puerta no produce oscilación ni apelotonamiento; contadores lineales; memoria plana en la carrera larga.

**Sesión de verificación del original** antes de las specs del hito 3.

**Hito 3 · Corte vertical feo.** Riesgo que retira: que el juego no sea divertido.
- Una facción estándar jugable contra una IA básica que expande y ataca. Base con edificios de reclutamiento, aldeas neutrales que se capturan y mantienen, oro, comida y población, héroe con ejército y experiencia, victoria por eliminación, pantalla mínima de configuración.
- Grid de visibilidad por equipo y opción de niebla de guerra para el jugador, desactivada por defecto.
- Sin arte: cápsulas, cajas y texto.
- Aceptación: una partida de principio a fin de entre media hora y una hora; el autor la juega varias veces y decide si el bucle engancha. Si no engancha, se ajusta aquí y no se avanza.

**Hito 4 · Partida completa.** Riesgo que retira: una IA que no da juego y partidas largas que se degradan.
- IA con sus tres capas, 9 perfiles con sus bonificaciones de recursos, presupuesto por tick, percepción completa, de 5 a 10 rivales por mapa que también se atacan entre sí.
- Guardado y carga robustos y versionados, velocidad de partida, pausa si se decide.
- Sanadores, mejoras de edificios, comercio, progresión completa de héroes.
- Partidas IA contra IA aceleradas con métricas y replays dorados en CI.
- Aceptación: una partida de varias horas sin degradación de rendimiento entre la primera y la última; los criterios de aceptación de la sección de IA.

**Hito 5 · Pipeline de arte.** Riesgo que retira: no poder producir assets para 11 facciones.
- Pack base y rig humanoide compartido elegidos juntos; carga de clips a buffers; impostores; LODs definitivos; terreno con texturas; golden images.
- Primera facción con modelos definitivos.
- Aceptación: una facción completa y bonita en pantalla dentro del presupuesto de GPU en los dos niveles.

**Hito 6 · Facciones de la fase A.** Riesgo que retira: un sistema de datos que no expresa las facciones.
- Las 8 de Imperivm III, Cartago la segunda en entrar. Sistema de efectos completado con las habilidades ya verificadas. Equilibrado con partidas IA contra IA.

**Hito 7 · Mapas.** Fin de la fase A.
- Decisión entre editor propio, procedural o mezcla. Mapas estándar, grandes y Europa, con la navegación jerárquica y el streaming de terreno que necesiten. Condiciones de victoria definitivas.

### Fase B: Plus Ultra

- **Hito 8 · Escala.** Subir el dial de la economía y de los topes, reclutamiento por colas, veteranía agregada, jerarquía héroe-oficiales si se eligió.
- **Hito 9 · Abastecimiento.** La mecánica estrella de la fase, con la IA defensiva aprovechándola.
- **Hito 10 · Ciudad por parcelas.** Anillos de muralla, puertas por facción, plantillas de ciudad para la IA, asedio.
- **Hito 11 · Facciones nuevas.** Griegos, atlantes y visigodos.

## Decisiones pendientes

- [x] `wgpu` frente a Vulkan directo con `ash`: **`wgpu`** (v2)
- [x] Herramienta de SDD: **Claude Code con flujo multiagente propio** (v2.1, ADR-0008)
- [x] Máquina de referencia y máquina mínima (v2.1)
- [x] Ventajas de la IA: recursos extra por dificultad, como datos (v2.1)
- [x] ECS: **ninguno**; tablas SoA con un handle común (v2.1)
- [x] Animación de multitudes: **esqueleto horneado y skinning en GPU**, con impostores (v2.1)
- [x] Tick de simulación: **20 Hz** (v2.1)
- [x] Formato de guardado: **binario versionado**; RON solo para autoría (v2.1)
- [ ] Niebla de guerra: recomendación hecha, pendiente de confirmar por el autor
- [ ] Sistema héroe-ejército: opción a), b) o c) para la fase A; se cierra en el hito 2
- [ ] Origen de los assets 3D; se decide en el hito 5 a más tardar
- [ ] Qué significaba "mantener" una aldea en el original
- [ ] Verificar las habilidades de las unidades especiales
- [ ] Diseño de griegos, atlantes y visigodos, y si los atlantes traen elementos fantásticos
- [ ] Condiciones de victoria
- [ ] Mapas: editor propio, procedurales o mezcla
- [ ] Interfaz definitiva de juego
- [ ] SMT, modo de presentación y factor de aceleración: se miden en el hito 2
- [ ] Otras mecánicas propias

## Preguntas abiertas para el autor

**Respondidas (v2.1 y v2.2):**

1. **Máquina de referencia.** Ryzen 5 5600X, 16 GB, RTX 3070 8 GB. El autor quiere que la máquina objetivo sea peor: ver los dos niveles de máquina en la sección de escala.
2. **Dedicación.** Mucha.
4. **Ventajas de la IA.** Recibe recursos extra además de jugar mejor.
5. **Niebla de guerra.** El autor prefería jugar sin ella. Recomendación en la sección de mecánicas, pendiente de confirmar.
7. **Arte.** Hay algo de presupuesto y el autor puede generar y modelar. Estrategia híbrida en la sección de arte.
8. **Relación entre IAs.** Se pelean entre sí.
11. **Plataforma.** Solo Windows en la fase A (v2.2). La CI, cuando exista el remoto, corre en un runner Windows.

**Pendientes:**

3. **Verificación del original.** ¿Tienes Imperivm III a mano para la sesión de verificación? ¿Hay guías o comunidades en las que confíes?
6. **Héroe-ejército.** ¿Opción a), b) o c) para la fase A? ¿Recuerdas si en el original las unidades sin héroe podían actuar con normalidad?
9. **Asedio y naval.** ¿Máquinas de asedio en la fase A? ¿Unidades navales fuera de alcance?
10. **Ritmo.** ¿Quieres pausa con órdenes y control de velocidad de partida? Cambia el diseño de la IA táctica y de la interfaz.
12. **Condiciones de victoria.** ¿Eliminación total o también rendición, control de territorio o tiempo?
13. **Alcance del hito 3.** ¿Qué facción estándar quieres como primera jugable?
14. **Alianzas (nueva).** Las IAs se pelean entre sí. ¿Quieres además alianzas o diplomacia, aunque sea en fase B?
15. **Resolución y monitor (nueva).** ¿Juegas a 1080p o a 1440p, y a 144 Hz? Fija el presupuesto de GPU.
16. **Tamaño de los mapas (nueva).** ¿Cuánto mide el mapa de Europa en el original, aproximadamente, y cuántas unidades vivas cabían? Fija la resolución del grid de navegación y la memoria de los sectores.

## Siguiente paso

1. Hecho: Rust 1.99.1 y `cargo-deny` instalados, versión fijada en `rust-toolchain.toml` y las cinco comprobaciones en verde sobre el esqueleto.
2. El autor responde las preguntas pendientes y confirma o rechaza la recomendación sobre la niebla.
3. Hecho: constitución redactada en `specs/constitution.md`.
4. Hecho: ADR-0001 a ADR-0009 en `specs/adr/`.
5. Hecho (26 de octubre de 2026): specs de arquitectura de `engine_core` y `engine_sim` aprobadas. Las de `render`, `rules` y `app`, antes del hito 1; las de `engine_tools` y `game_tools`, cuando un hito pida su primer binario (v2.2).
6. Plan P01 de `engine_core` en borrador (`specs/plans/01-engine-core.md`): `/tareas`, implementación y verificación en serie. Después, el plan de `engine_sim`, que espera a las ocho primeras tareas de core, y la spec de la simulación sintética (`/spec motor simulacion-sintetica`), que se puede escribir mientras tanto (v2.3).
7. Hito 0.
8. Specs de features del hito 1 (cámara, selección, movimiento, combate, snapshots, guardado) y a continuación plan, tareas e implementación.

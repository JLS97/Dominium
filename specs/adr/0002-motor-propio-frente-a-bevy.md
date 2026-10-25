# ADR-0002: Motor propio frente a Bevy

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** todos

## Contexto

En Rust la opción por defecto para un juego es Bevy: ECS, render, assets, escena, entrada, interfaz y herramientas en un solo paquete. El proyecto necesita quince mil unidades individuales, simulación determinista con cualquier número de hilos, tablas SoA, un hilo de simulación propio con tick fijo y render GPU-driven con skinning, culling y LOD en GPU. Y el código lo generan agentes, así que el tiempo de compilación y la estabilidad de las API que usan pesan más de lo habitual. Godot, que el autor conoce, se descartó antes de este documento: no está pensado para quince mil entidades individuales con control fino de los datos.

La revisión técnica independiente del 25 de octubre de 2026 pidió que este ADR diera la razón real y no la cómoda, y que dejara escrito el coste.

## Opciones consideradas

1. **Bevy.** A favor: trae hecho alrededor de un tercio de lo que un motor necesita y que no hace único al juego: carga de assets, escena, entrada, integración de interfaz y depuración; comunidad y ejemplos abundantes. En contra: su API cambia en cada versión menor, cada pocos meses, y el código generado por agentes se apoya en ejemplos que quedan obsoletos; sus tiempos de compilación alargan cada iteración del agente; su ECS impone un modelo de identidad, de scheduling y de serialización que choca con tablas SoA deterministas y con un hilo de simulación propio (ver ADR-0004); su render se puede doblegar a instancing GPU-driven con buffers propios, pero a contracorriente de su diseño.
2. **Motor propio ad hoc montando piezas sueltas:** `wgpu`, `winit`, `glam`, `egui`, `gltf`, `rayon`. A favor: arquitectura exacta para este juego y nada más; compilación rápida; los agentes trabajan sobre specs propias en vez de sobre una API ajena; cada regla de la constitución se puede imponer sin pelear con el framework. En contra: hay que construir ese tercio de infraestructura, y no es gratis; no hay comunidad que haya resuelto ya los problemas.

| Dimensión | Bevy | Motor propio |
| --- | --- | --- |
| Bucle del agente: compilación y deriva de API | lento e inestable | rápido y estable |
| Control del determinismo y de los datos | a contracorriente | total |
| Infraestructura ya hecha | alrededor de un tercio del motor | hay que hacerla |
| Riesgo de no terminar | menor por la infraestructura | mayor; se mitiga con la escalera de hitos |

## Decisión

Motor propio ad hoc en Rust, montado con piezas sueltas. La razón que pesa es el bucle del agente: tiempo de compilación y deriva de la API. Que el render de Bevy sea difícil de doblegar es secundario. Se asume el coste: la infraestructura (carga de assets, escena, entrada, interfaz, depuración) está repartida por hitos en el GDD y no se da por hecha, y se reutilizan librerías para todo lo que no hace único al juego.

## Consecuencias

- Más fácil: cumplir la constitución, medir con precisión y mantener a los agentes dentro de specs propias.
- Más difícil: carga de assets, ciclo de vida de la escena, entrada, interfaz y depuración. Están en el plan de hitos y cuentan como trabajo.
- Vigilar: la tentación de generalizar el motor "por si acaso". Nada genérico sin un uso en el juego.
- Se revisa si: un hito de infraestructura se come mucho más de lo previsto y, a la vez, Bevy ha estabilizado su API. Entonces se vuelve a comparar con números, no con opiniones.

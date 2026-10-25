# ADR-0004: Tablas SoA con un handle común, sin ECS

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** 0 en adelante

## Contexto

Quince mil soldados, más héroes, edificios, proyectiles y formaciones. Todo determinista, serializable, en SoA, con borrado diferido y un conjunto activo para que las guarniciones no pasen por combate. La forma de los datos se conoce de antemano. Las relaciones héroe-soldado, edificio-cola y proyectil-objetivo se fijan en el hito 1, así que la decisión no se podía aplazar al hito 2 como decía el GDD v2; la revisión técnica independiente lo señaló.

## Opciones consideradas

1. **ECS general** (`hecs`, `bevy_ecs`). A favor: flexibilidad para añadir componentes, consultas genéricas y, en `bevy_ecs`, scheduling. En contra: impone su propio modelo de identidad, distinto del de las tablas; itera por arquetipos, que no es SoA por campo en el sentido que la caché necesita; serialización ajena al estado canónico; orden de iteración y scheduling que no son deterministas sin trabajo extra; un segundo modelo mental para los agentes.
2. **Tablas SoA explícitas con un handle común.** Cada tipo de entidad es una tabla densa con un vector por campo. El handle es slot más generación, y es el mismo tipo para todo. Indirección handle→slot y slot→handle. Swap-remove diferido al final del tick, por slot descendente. Conjunto activo por tabla. A favor: exactamente la forma de datos que exige la constitución; serialización directa; orden de iteración estable; un solo modelo de identidad; código pequeño. En contra: añadir un campo es tocar la tabla; sin consultas genéricas; el código de tablas hay que escribirlo y probarlo.
3. **Híbrido:** tablas para soldados y un ECS para lo disperso. Descartado: dos modelos de identidad y dos serializaciones para unas decenas de edificios y héroes.

| Dimensión | ECS general | Tablas con handle común |
| --- | --- | --- |
| Determinismo | con trabajo extra | por construcción |
| Serialización del canónico | ajena | directa |
| Modelos de identidad | dos | uno |
| Añadir un campo o una entidad | fácil | manual |
| Código que mantener | el del ECS | unas pocas primitivas propias |

## Decisión

Tablas SoA explícitas para todas las entidades, con un único tipo de handle definido en `engine_core`, las primitivas de tabla (inserción, borrado diferido, indirección, conjunto activo, iteración por chunks) en `engine_sim`, y las tablas concretas definidas por `game_rules`. Sin ECS en ningún crate.

## Consecuencias

- Más fácil: determinismo, serialización, regla 19 (arenas y buffers por tabla) y relaciones por handle entre cualquier par de entidades.
- Más difícil: añadir campos y entidades nuevas es más manual. Si el hito 3 lo pide, una macro pequeña o un generador de tablas; no antes.
- Vigilar: que nadie meta un vector de structs por comodidad (regla 5) ni un segundo tipo de identificador.

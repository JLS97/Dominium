# ADR-0005: Animación de multitudes con esqueleto horneado y skinning en GPU

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** 2 y 5

## Contexto

Quince mil unidades animadas en pantalla a escala de fase B, un rig humanoide compartido por todas las facciones humanas, muchas mallas por facción, armas y estandartes colgados de los huesos, y héroes con más detalle. Presupuesto de GPU de la máquina mínima: cinco milisegundos para unidades. VRAM de la mínima: 2,5 GB. El GDD v2 proponía vertex animation textures; la revisión técnica independiente hizo los números y la decisión cambió.

## Opciones consideradas

1. **Vertex animation textures.** Posición y normal por vértice y por frame horneadas en una textura; el vertex shader hace una o dos lecturas. A favor: el vértice más barato posible; sirve para deformaciones sin esqueleto. En contra: memoria por malla y por LOD, del orden de cinco megabytes por malla con quince clips, y cientos de megabytes para once facciones; cada malla o arma nueva exige rehornear; sin mezcla de clips más allá de un fundido; colgar un arma de la mano exige hornear el hueso igualmente; herramienta de horneado aparte que se reejecuta al tocar cualquier malla.
2. **Esqueleto horneado con skinning en GPU.** Los clips del glTF se muestrean al cargar a un buffer de matrices por hueso y frame, compartido por todas las mallas del rig. Cada instancia lleva clip, frame y un desfase de fase. El vertex shader lee cuatro huesos y mezcla dos frames. A favor: medio megabyte para todo el set humano; mallas nuevas sin horneado; mezcla de clips; armas y estandartes como mallas instanciadas colgadas de un hueso; héroes y tropa por el mismo camino. En contra: vértice entre una y media y dos veces más caro que con VAT; exige que todas las mallas humanas estén rigueadas al esqueleto compartido.
3. **Skinning en CPU.** Descartado: trabajo por unidad en la CPU (regla 8).

| Dimensión | VAT | Esqueleto en GPU |
| --- | --- | --- |
| VRAM para once facciones | cientos de megabytes | menos de un megabyte |
| Pipeline por malla o arma nueva | rehornear | nada |
| Coste de vértice | 1x | 1,5x a 2x |
| Mezcla de clips y props | pobre | natural |
| Herramienta aparte | sí | no |

## Decisión

Esqueleto horneado y skinning en GPU. Cuatro influencias por vértice en LOD0, una o dos en LOD1 y LOD2, e impostores (billboard con pocos ángulos y un ciclo corto) en LOD3, que es donde las miles de unidades lejanas dejan de costar. Desfase de fase por instancia para que un ejército no marche al unísono. VAT no se usa: solo pagaría en deformación no rígida, y el juego no la tiene.

## Consecuencias

- Más fácil: el pipeline de assets, la memoria, las armas y estandartes, y tratar a los héroes como una unidad más con más huesos.
- Más difícil: los impostores necesitan su propio horneado, que vive en `engine_tools`; el rig compartido obliga a elegir el pack base y el esqueleto juntos al inicio del hito 5.
- Vigilar: el coste de vértice, medido en el hito 2 con la malla placeholder rigueada. Si LOD0 pesa, se bajan influencias o se acorta su distancia antes de tocar nada más.

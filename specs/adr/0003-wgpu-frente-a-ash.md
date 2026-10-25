# ADR-0003: wgpu frente a Vulkan directo con ash

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** 1 en adelante

## Contexto

El render es GPU-driven: instancing, skinning en GPU, culling y LOD en compute, sombras en cascada y terreno por chunks. Necesita compute, storage buffers, indirect multi-draw, arrays de texturas y timestamp queries. El objetivo es Windows. La CI no tiene GPU y necesita golden images. El código lo generan agentes, así que la superficie de error de la API importa tanto como su techo de rendimiento.

## Opciones consideradas

1. **`ash`, Vulkan directo.** A favor: control total y acceso a todo, incluidos mesh shaders, bindless completo y sincronización fina. En contra: hay que escribir un backend entero antes de ver un soldado: sincronización, gestión de memoria, descriptores, swapchain y capas de validación; es la mayor superficie de error posible para código generado; sin ruta para CI sin GPU salvo instalar un driver por software.
2. **`wgpu`.** A favor: API segura y validada, sin `unsafe` en el uso normal; compute, storage buffers, `MULTI_DRAW_INDIRECT_COUNT`, `INDIRECT_FIRST_INSTANCE`, `TEXTURE_BINDING_ARRAY` y timestamp queries como features nativas, suficiente para todo lo que el GDD describe; backends DX12 y Vulkan con el mismo código; adaptador de respaldo WARP, el rasterizador por software de Windows, para golden images en CI sin GPU. En contra: sin mesh shaders; una capa de abstracción con algo de coste; la API cambia entre versiones; `Queue::write_buffer` asigna un staging por llamada.
3. **DirectX 12 directo.** Descartado: las mismas pegas que `ash` y ata el motor a Windows sin ganar nada que el presupuesto necesite.

| Dimensión | `ash` | `wgpu` |
| --- | --- | --- |
| Tiempo hasta el primer soldado en pantalla | meses | semanas |
| Superficie de error con agentes | máxima | pequeña |
| Features que el GDD necesita | todas | todas |
| CI sin GPU | difícil | WARP |
| Techo de rendimiento | mayor | suficiente para el presupuesto |

## Decisión

`wgpu`, con deberes:

- Versión fijada en el workspace; se sube una vez por hito, con su propio commit.
- Las tres features nativas que el render necesita se prueban en el hito 1 en la RTX 3070 y en WARP, antes de construir nada encima.
- Anillo propio de staging mapeado para las subidas por tick, en vez de `write_buffer` por frame.
- Golden images comparadas WARP contra WARP, con tolerancia y en escenas pequeñas, porque WARP es lento y no da los mismos píxeles que el hardware.

## Consecuencias

- Más fácil: renderizar en el hito 1, CI con imágenes y portabilidad sin coste.
- Más difícil: nada de mesh shaders. Si algún día hacen falta, es otro ADR y probablemente otro backend.
- Vigilar: el coste de la abstracción, medido con timestamp queries en el hito 2; y la deriva de API, que se absorbe en un solo commit por hito.

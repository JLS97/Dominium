# ADR-0007: Guardado binario versionado con postcard; RON solo para autoría

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** 0 en adelante

## Contexto

El estado canónico ocupa unos tres megabytes a escala de fase B. El autoguardado en partidas de quince horas no puede producir un parón perceptible: el presupuesto es un parón de 50 milisegundos como máximo y un archivo de 20 megabytes como máximo. Los guardados y los replays llevan el hash de los datos con los que se jugó (regla 18), y un replay es semilla, hash de datos y órdenes selladas por tick. Los datos de contenido se escriben a mano en RON y se recargan en caliente en desarrollo. El GDD v2 proponía RON para todo; la revisión técnica independiente lo descartó para los guardados.

## Opciones consideradas

1. **RON para todo.** A favor: un solo formato, legible, ya en el stack. En contra: es texto; con miles de unidades es lento de escribir y de leer y los archivos son enormes; no cumple el presupuesto de guardado.
2. **`bincode`.** A favor: rápido, compacto y muy usado. En contra: la versión 2 cambió el formato y la API; el versionado sigue siendo responsabilidad del usuario.
3. **`postcard`.** A favor: formato estable y documentado, compacto gracias a los enteros de longitud variable, rápido, basado en serde, sin dependencias pesadas y pensado para formatos de larga vida. En contra: no es autodescriptivo; una struct que cambia rompe los guardados antiguos si no hay versionado explícito.
4. **Formato propio.** Descartado: trabajo sin ninguna ventaja sobre `postcard`.

| Dimensión | RON | `bincode` | `postcard` |
| --- | --- | --- | --- |
| Tamaño y velocidad con 15.000 unidades | malos | buenos | buenos |
| Estabilidad del formato | alta | cambió en la versión 2 | alta y documentada |
| Legible por una persona | sí | no | no |
| Versionado | manual | manual | manual |

## Decisión

`postcard` para guardados y replays, con una cabecera propia: número mágico, versión del formato, versión del juego y hash de los datos. Las migraciones entre versiones son explícitas: se conserva la struct antigua y se convierte a la nueva, o se decide romper la compatibilidad y se anota. RON queda solo para contenido de autoría: unidades, facciones, habilidades, plantillas de ciudad, perfiles de IA y mapas en edición.

## Consecuencias

- Más fácil: cumplir el presupuesto de guardado escribiendo desde una copia del canónico en el hilo de entrada y salida; replays pequeños que caben en CI como pruebas de regresión.
- Más difícil: cada cambio de una struct canónica es un cambio de versión del formato, con su migración o con la decisión explícita de romper compatibilidad. En desarrollo romper es aceptable si queda anotado.
- Vigilar: guardados dorados de cada versión en los tests, para que la carga de versiones anteriores no se rompa sin que nadie lo decida.

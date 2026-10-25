# Plan PNN: título

- **Spec:** ruta de la spec
- **Hito:**
- **Rama de integración:** main
- **Estado:** borrador | aprobado | en curso | terminado

## Enfoque

Cómo se resuelve, en un párrafo por pieza.

## Contratos

| Contrato | Crate | Quién lo usa | ¿Compartido? |
| --- | --- | --- | --- |
| | | | |

Los contratos compartidos (`engine_core`, handle, `Tick`, fases y eventos, API de órdenes, snapshot, replay y guardado) se congelan antes de abrir paralelismo.

## Riesgos

| Riesgo | Cómo se retira |
| --- | --- |
| | |

## Verificación

| Criterio | Cómo se demuestra | Tarea |
| --- | --- | --- |
| CA-1 | test, replay, contador, benchmark o revisión | PNN-T01 |

## Paralelismo

Qué puede ir en paralelo y qué no, y por qué.

## Tareas

### PNN-T01 · título

- **Tipo:** contrato | implementación | ligera | spike | datos
- **Crate:**
- **Depende de:**
- **Contrato que usa:**
- **Criterios que cubre:**
- **Tests de aceptación:** `crate/tests/aceptacion_<feature>_tNN.rs` | ninguno, porque…
- **Paralelizable:** sí | no, porque…
- **Revisores:** revisor-constitucion, …
- **Fuera de alcance:**
- **Estado:** pendiente | integrada (AAAA-MM-DD)

## Oleadas

| Oleada | Tareas | Nota |
| --- | --- | --- |
| 1 | PNN-T01 | contrato, va sola |

**Camino crítico:**

---
name: spec
description: Redacta una spec (arquitectura de un crate, feature del motor o del juego) o un ADR a partir del GDD y la constitución
argument-hint: <arquitectura|motor|juego|adr> <nombre>
disable-model-invocation: true
---

Redacta la spec `$ARGUMENTS`. Eres el orquestador: la escribes tú, con ayuda de subagentes para leer.

## Destino y plantilla

| Tipo | Archivo | Plantilla |
| --- | --- | --- |
| `arquitectura` | `specs/architecture/<crate>.md`, por ejemplo `engine_sim.md` | `specs/architecture/plantilla-crate.md` |
| `motor` | `specs/engine/<slug>.md` | `specs/game/plantilla-feature.md` |
| `juego` | `specs/game/<slug>.md` | `specs/game/plantilla-feature.md` |
| `adr` | `specs/adr/NNNN-<slug>.md`, con el siguiente número libre y su fila en `specs/adr/README.md` | `specs/adr/0000-plantilla.md` |

## Pasos

1. **Contexto.** Reúne las secciones relevantes del GDD, las reglas de la constitución, los ADR relacionados y las specs que ya existan. Para búsquedas amplias usa `explorador` y quédate con sus conclusiones, no con los archivos.
2. **Redacción**, siguiendo la plantilla:
   - Cada criterio de aceptación es medible, lleva un id (`CA-1`, `CA-2`…) y dice qué lo demuestra: un test, un replay, un contador o una métrica de la tabla de presupuestos.
   - En las de arquitectura, la API pública y las dependencias permitidas son el contrato. Escribe firmas concretas, porque `autor-tests` y los implementadores trabajarán contra ellas.
   - En las de juego, cada mecánica lleva su origen: recordada, por verificar, adoptada o propia.
   - Lo que no sepas va a "Preguntas abiertas". No rellenes huecos con suposiciones ni con mecánicas de los Imperivm originales.
3. **Revisión del borrador.** Lanza `revisor-spec` en modo `spec` sobre el borrador, siempre: comprueba la calidad (criterios medibles, casos límite, contratos con firmas) y, si la spec es de juego o toca mecánicas o datos, la fidelidad. Corrige lo que marque o conviértelo en pregunta. No pases a la presentación con hallazgos bloqueantes.
4. **CLAUDE.md del crate.** Si la spec es de arquitectura, crea también `<carpeta del crate>/CLAUDE.md`, de 10 a 20 líneas: responsabilidad, enlace a la spec, reglas que le afectan, comandos de prueba del crate y lo que no hace nunca. En el `lib.rs` o `main.rs` del crate, quita el "(pendiente)" de la línea `Spec:`.
5. **ADR.** Un ADR registra una decisión que ya ha tomado el autor. Si no está tomada, prepara las opciones con sus pros y contras y pregúntale. No decidas tú.
6. **Presentación.** Muestra al autor un resumen de cinco líneas, las decisiones que has tomado tú y las preguntas abiertas. Añade las preguntas a `specs/preguntas-pendientes.md`.
7. **Commit.** No commitees hasta que el autor apruebe. Con su aprobación: `git add` de los archivos y `git commit -m "spec: <nombre>"`.

Siguiente paso, una vez aprobada: `/planificar <ruta de la spec>`.

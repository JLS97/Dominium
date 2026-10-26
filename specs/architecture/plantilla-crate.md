# Crate `nombre`

- **Carpeta:**
- **Responsabilidad:** una frase.
- **Reglas de la constitución que le afectan:** números y, para cada una, qué significa en este crate.
- **ADR relacionados:**
- **Estado:** borrador | aprobada

## Qué hace

## Qué no hace nunca

## API pública

Es el contrato. `autor-tests` escribe contra estas firmas y los implementadores no las cambian salvo en una tarea de tipo `contrato`. Firmas concretas en Rust, no descripciones; todo lo que no esté aquí es privado.

```rust
pub struct Tick(pub u64);
pub fn step(world: &mut World, orders: &[Order]) -> TickReport;
```

Para cada tipo o función, una línea con lo que garantiza y lo que exige del llamante.

## Contratos compartidos que toca

Cuáles de los contratos compartidos (handle, `Tick`, fases y eventos, API de órdenes, layout del snapshot, formatos de replay y guardado) define o usa este crate, y si esta spec los crea o solo los consume.

## Dependencias permitidas

Lista cerrada: crates del workspace y externos, con la versión que irá en `[workspace.dependencies]`. Cualquier otro es una violación de las reglas 1 y 2.

De desarrollo, en lista aparte: lo que solo usan tests y benches (`proptest`, `criterion`, el asignador contador…), también con su versión, porque la tarea de contrato los añade a `[workspace.dependencies]` y `autor-tests` solo puede declararlos con `nombre.workspace = true`.

## Datos y formatos

Qué archivos lee o escribe y en qué formato.

## Presupuestos

Qué métricas de la tabla de presupuestos de la constitución le corresponden, con sus números.

## Criterios de aceptación

Numerados, medibles y con lo que los demuestra. `/planificar` los reparte en tareas y `autor-tests` escribe un test por criterio.

- **CA-1.** Texto del criterio. Lo demuestra: test `ca_01_...` | replay | contador | métrica.
- **CA-2.** …

## Casos límite

Los que rompen: cero entidades, el N máximo del presupuesto, guardar a mitad, cambiar el número de hilos, un muro añadido en caliente, lo que aplique.

## Fuera de alcance

## Preguntas abiertas

Las responde el autor. Si una afecta a un criterio, la spec no se aprueba hasta tenerla.

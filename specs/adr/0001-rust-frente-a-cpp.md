# ADR-0001: Rust frente a C++

- **Estado:** aceptado
- **Fecha:** 2026-10-25
- **Hito afectado:** todos

## Contexto

Motor propio para un RTS de hasta quince mil unidades individuales, con simulación determinista y paralela. El autor domina Rust y C++ por igual. Todo el código lo generan agentes de IA, con el autor como revisor. Las restricciones que mandan: que los errores que un agente introduce en silencio los detecte una herramienta y no una partida de diez horas; paralelismo de datos barato y seguro; un solo sistema de build que los agentes sepan usar sin ayuda.

## Opciones consideradas

1. **C++20 con CMake.** A favor: ecosistema de motores y herramientas gráficas maduro, control absoluto de memoria y layout, el autor lo domina. En contra: comportamiento indefinido y carreras de datos que compilan sin aviso, justo el tipo de error que un agente mete y nadie ve; build system y gestión de dependencias que consumen tiempo de agente; el paralelismo seguro depende de disciplina, no del compilador.
2. **Rust estable con cargo.** A favor: el compilador rechaza aliasing mutable, carreras de datos y usos tras liberar; `rayon` convierte un bucle en paralelo con una línea y el compilador garantiza que es seguro; cargo unifica build, tests, benchmarks y dependencias; clippy y rustfmt dan al agente un bucle de corrección automático. En contra: ecosistema gráfico más joven; tiempos de compilación largos que penalizan el bucle del agente; el borrow checker hace incómodos algunos patrones de tablas con índices, que se resuelven con handles.

| Dimensión | C++ | Rust |
| --- | --- | --- |
| Errores silenciosos del agente | altos | bajos |
| Paralelismo seguro | disciplina | compilador |
| Bucle del agente | CMake y sanitizers | cargo, clippy y tests |
| Ecosistema gráfico | maduro | suficiente con `wgpu` |
| Rendimiento alcanzable | igual | igual |

## Decisión

Rust estable, edición 2024, objetivo `x86_64-pc-windows-msvc`, versión fijada en `rust-toolchain.toml` y subida de forma deliberada una vez por hito. `unsafe` prohibido salvo en `engine_render` (regla 3 de la constitución).

## Consecuencias

- Más fácil: paralelizar la simulación, confiar en código generado y tener un solo flujo de herramientas.
- Más difícil: tiempos de compilación. Se mitigan con crates pequeños, `cargo check` en el bucle interno y un perfil de desarrollo que optimiza las dependencias pero no el código propio.
- Vigilar: la deriva de versiones de `wgpu` y `winit`. Se fijan y se suben una vez por hito.
- Se revisa si: nunca por sí sola. Es una decisión estructural; cambiarla es empezar otro proyecto.

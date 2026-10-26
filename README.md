# Plus Ultra: Dominium

RTS para uso personal inspirado en la saga Imperivm, con motor propio en Rust. Primero se replica el modo libre de Imperivm III a su escala original; después se va más allá: más escala, abastecimiento, ciudades por parcelas y tres facciones nuevas.

- Documento de contexto y GDD: [specs/gdd/plus-ultra-dominium-contexto-gdd.md](specs/gdd/plus-ultra-dominium-contexto-gdd.md)
- Constitución: [specs/constitution.md](specs/constitution.md)
- Decisiones técnicas (ADR): [specs/adr/](specs/adr/)
- Revisiones externas: [specs/reviews/](specs/reviews/)
- Contexto para agentes de IA: [CLAUDE.md](CLAUDE.md)

## Requisitos

- Rust estable con `rustfmt` y `clippy`, instalado con rustup: <https://rustup.rs>
- `cargo-deny`: `cargo install cargo-deny --locked`

## Comprobaciones

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-arch
cargo deny check
```

`cargo xtask check-arch` comprueba las reglas de la constitución que se pueden leer del workspace: dependencias solo de `game/` a `engine/`, sin gráficos en `engine_sim` ni en `game_rules`, y `#![forbid(unsafe_code)]` en todos los crates salvo `engine_render`.

## Estructura

```
engine/   motor: core, sim, render, tools (sin conceptos de juego)
game/     juego: rules, app, tools
xtask/    tareas de repositorio
specs/    constitución, ADR, GDD, specs de arquitectura y de features
data/     contenido en RON: unidades, facciones, perfiles de IA; en el hito 0, el escenario sintético
assets/   arte y audio (vacío hasta el hito 5)
```

## Estado

Esqueleto del workspace creado el 25 de octubre de 2026 y verificado: formato, clippy estricto, tests, `check-arch` y cargo deny pasan con el toolchain fijado. Ningún crate tiene código de juego todavía. El hito 0 está descrito en el GDD.

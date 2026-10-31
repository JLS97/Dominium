//! Perfilado con Tracy (regla 12): arranque del cliente, marcas de frame y de
//! tick, nombre de hilo, la zona `Span` y las macros `span!` y `plot!`.
//!
//! Todo depende de la feature `tracy` del crate. Sin ella, las funciones no
//! hacen nada, `Span` es un tipo de tamaño cero y las macros no asignan ni
//! llaman a `tracy-client`. Con ella, `tracy-client` arranca con el proceso
//! (sin `manual-lifetime`), así que las zonas se registran desde la primera
//! instrucción de `main`.
//!
//! Las macros se expanden en el crate que las usa, así que un `cfg` dentro de
//! `macro_rules!` no serviría: cada una tiene dos definiciones, una por
//! variante de la feature, y las dos se apoyan en ítems `#[doc(hidden)] pub`
//! de este módulo que no forman parte del contrato y pueden cambiar.
// Contrato B2: los cuerpos son marcadores pendientes y no usan sus parámetros.
// `expect` obliga a P01-T09 a retirar estas líneas al implementar el módulo.
#![expect(unused_variables, reason = "cuerpos pendientes de P01-T09")]
#![cfg_attr(
    feature = "tracy",
    expect(
        dead_code,
        reason = "la zona interna de `Span` no se construye ni se lee hasta P01-T09"
    )
)]

use core::marker::PhantomData;

/// Reexport de `tracy-client` para las macros `span!` y `plot!` con la feature activa.
/// No forma parte del contrato: puede cambiar o desaparecer.
#[cfg(feature = "tracy")]
#[doc(hidden)]
pub use tracy_client;

/// Con la feature `tracy`, llama a `Client::start()` de `tracy-client`; es idempotente y se puede llamar desde cualquier
/// hilo. Sin la feature no hace nada. El cliente de Tracy arranca con el proceso (sin `manual-lifetime`), así que las zonas
/// se registran desde la primera instrucción de `main`, se haya llamado o no a `start()`.
pub fn start() {
    todo!("P01-T09")
}

/// Cierto con la feature activa, desde el inicio del proceso; falso sin ella.
#[must_use]
pub fn is_active() -> bool {
    todo!("P01-T09")
}

/// Fin de un frame del hilo principal.
pub fn frame_mark() {
    todo!("P01-T09")
}

/// Fin de un tick de simulación: frame secundario llamado `tick`.
pub fn tick_mark() {
    todo!("P01-T09")
}

/// Nombre del hilo actual en el perfilador.
pub fn set_thread_name(name: &'static str) {
    todo!("P01-T09")
}

/// Zona de perfilado. Se cierra al soltarla. No es `Send` en ninguna configuración: se abre y se cierra en el mismo hilo.
/// Sin la feature es un tipo de tamaño cero.
#[must_use = "una zona que no se guarda en una variable se cierra en la misma línea"]
pub struct Span {
    /// Zona abierta en Tracy; se cierra al soltarla.
    #[cfg(feature = "tracy")]
    inner: tracy_client::Span,
    /// Un puntero crudo en un `PhantomData` quita `Send` y `Sync` sin ocupar memoria ni necesitar `unsafe`.
    _not_send: PhantomData<*mut ()>,
}

impl Span {
    /// Constructor de la macro `span!` sin la feature. `name` solo fija el tipo del literal.
    /// No forma parte del contrato: puede cambiar.
    #[cfg(not(feature = "tracy"))]
    #[doc(hidden)]
    pub fn disabled(name: &'static str) -> Span {
        todo!("P01-T09")
    }

    /// Constructor de la macro `span!` con la feature: abre la zona en `location`, que crea `span_location!`
    /// de `tracy-client` como estático. No forma parte del contrato: puede cambiar.
    #[cfg(feature = "tracy")]
    #[doc(hidden)]
    pub fn enabled(location: &'static tracy_client::SpanLocation) -> Span {
        todo!("P01-T09")
    }
}

/// Destino de la macro `plot!` sin la feature. `name` solo fija el tipo del literal.
/// No forma parte del contrato: puede cambiar.
#[cfg(not(feature = "tracy"))]
#[doc(hidden)]
pub fn plot_disabled(name: &'static str, value: f64) {
    todo!("P01-T09")
}

/// Destino de la macro `plot!` con la feature: `name` lo crea `plot_name!` de `tracy-client`, ya terminado en cero.
/// No forma parte del contrato: puede cambiar.
#[cfg(feature = "tracy")]
#[doc(hidden)]
pub fn plot_enabled(name: tracy_client::PlotName, value: f64) {
    todo!("P01-T09")
}

/// `let _zona = engine_core::span!("fase_mover");`. El nombre es un literal: no asigna.
#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! span {
    ($name:literal) => {
        $crate::profile::Span::disabled($name)
    };
}

/// `let _zona = engine_core::span!("fase_mover");`. El nombre es un literal: no asigna.
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! span {
    ($name:literal) => {
        $crate::profile::Span::enabled($crate::profile::tracy_client::span_location!($name))
    };
}

/// `engine_core::plot!("unidades_vivas", valor_f64)`. El nombre es un literal: no asigna.
#[cfg(not(feature = "tracy"))]
#[macro_export]
macro_rules! plot {
    ($name:literal, $value:expr) => {
        $crate::profile::plot_disabled($name, $value)
    };
}

/// `engine_core::plot!("unidades_vivas", valor_f64)`. El nombre es un literal: no asigna.
#[cfg(feature = "tracy")]
#[macro_export]
macro_rules! plot {
    ($name:literal, $value:expr) => {
        $crate::profile::plot_enabled($crate::profile::tracy_client::plot_name!($name), $value)
    };
}

#[cfg(not(feature = "wasmedge"))]
mod wasmtime;
#[cfg(not(feature = "wasmedge"))]
pub use self::wasmtime::*;
#[cfg(feature = "wasmedge")]
mod wasmedge;
#[cfg(feature = "wasmedge")]
pub use self::wasmedge::*;

#![no_std]
// The mandatory project convention requires an explicit `return` statement
// on every function's return value, which clippy's needless_return lint
// otherwise flags. The explicit-return convention wins; this lint is
// disabled crate-wide to keep `-D warnings` clean without violating it.
#![allow(clippy::needless_return)]

// === Module Wiring

mod contract;
mod error;
mod event;
mod types;

pub use contract::{PaymentRegistry, PaymentRegistryClient};
pub use error::PaymentRegistryError;
pub use types::{DataKey, PaymentRecord};

#[cfg(test)]
mod test;

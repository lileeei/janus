//! # Janus Interfaces (L1 - Public API Contract)
//!
//! This crate defines the public-facing API for interacting with browsers
//! using the Janus client. It provides protocol-agnostic traits (`Browser`, `Page`),
//! error types (`ApiError`), and common data structures used across different
//! browser implementations.

mod browser;
mod common;
mod error;
mod page;

pub use browser::*;
pub use common::*;
pub use error::*;
pub use page::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

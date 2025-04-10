//! # Janus Chrome Browser Implementation (L2)
//!
//! Implements the `janus-interfaces` traits (`Browser`, `Page`) for
//! Google Chrome / Chromium browsers using the Chrome DevTools Protocol (CDP).

use actix::Addr; // Re-export if needed internally

pub mod actors;
pub mod browser;
pub mod error; // Add error module
pub mod page;
pub mod protocol;

pub use browser::ChromeBrowser; // Expose the L2 implementation struct

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

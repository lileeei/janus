mod actor;
mod browser;
mod config;
mod error;
mod launcher;
mod page;
mod protocol;

pub use actor::ChromeBrowserActor;
pub use browser::*;
pub use config::{ChromeBrowserConfig, Viewport};
pub use error::ChromeError;
pub use launcher::ChromeLauncher;
pub use page::ChromePageActor;

use janus_core::prelude::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
mod actor;
mod browser;
mod config;
mod error;
mod page;
mod protocol;

pub use actor::ChromeBrowserActor;
pub use browser::*;
pub use config::{ChromeBrowserConfig, Viewport};
pub use error::ChromeError;
pub use page::ChromePageActor;

use janus_core::prelude::*;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

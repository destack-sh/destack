mod map;
mod table;

pub use map::*;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use table::watch_page_write;

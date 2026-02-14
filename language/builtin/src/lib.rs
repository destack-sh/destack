#[cfg(all(
    any(
        feature = "lib-bun",
        feature = "lib-deno",
        feature = "lib-node",
        feature = "lib-undici-types"
    ),
    not(any(feature = "versions-all", feature = "versions-latest"))
))]
compile_error!(
    "runtime lib families require an explicit version mode: versions-all or versions-latest"
);

mod core;
mod libs;

pub use core::*;
pub use libs::*;

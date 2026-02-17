#![feature(default_field_values)]
#![feature(if_let_guard)]

// preset selection
#[cfg(all(feature = "core", feature = "ide"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);
#[cfg(all(feature = "core", feature = "run"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);
#[cfg(all(feature = "core", feature = "full"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);
#[cfg(all(feature = "ide", feature = "run"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);
#[cfg(all(feature = "ide", feature = "full"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);
#[cfg(all(feature = "run", feature = "full"))]
compile_error!(
    "wasm preset features are mutually exclusive: choose only one of core, ide, run, or full"
);

// feature constraints
#[cfg(all(feature = "builtin-full", feature = "builtin-core"))]
compile_error!("choose only one builtin preset: builtin-full or builtin-core");
#[cfg(all(feature = "native-codegen", not(feature = "optimize")))]
compile_error!("native-codegen requires optimize");
#[cfg(all(feature = "deadlock-detection", not(feature = "parallel")))]
compile_error!("deadlock-detection requires parallel");

mod binding;

pub use binding::*;

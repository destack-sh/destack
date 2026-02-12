#![feature(default_field_values)]
#![feature(if_let_guard)]

// profile selection
#[cfg(all(feature = "profile-wasm-core", feature = "profile-wasm-ide"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);
#[cfg(all(feature = "profile-wasm-core", feature = "profile-wasm-run"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);
#[cfg(all(feature = "profile-wasm-core", feature = "profile-wasm-full"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);
#[cfg(all(feature = "profile-wasm-ide", feature = "profile-wasm-run"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);
#[cfg(all(feature = "profile-wasm-ide", feature = "profile-wasm-full"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);
#[cfg(all(feature = "profile-wasm-run", feature = "profile-wasm-full"))]
compile_error!(
    "wasm profile features are mutually exclusive: choose only one profile-wasm-* feature"
);

// feature constraints
#[cfg(all(feature = "builtin-full", feature = "builtin-wasm-core"))]
compile_error!("choose only one builtin profile: builtin-full or builtin-wasm-core");
#[cfg(all(feature = "native-codegen", not(feature = "optimize")))]
compile_error!("native-codegen requires optimize");
#[cfg(all(feature = "deadlock-detection", not(feature = "parallel")))]
compile_error!("deadlock-detection requires parallel");

mod binding;

pub use binding::*;

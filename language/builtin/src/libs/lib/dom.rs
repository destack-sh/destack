use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DOM_ASYNCITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "asynciterable.d.ts",
    include_str!(concat!("../../../lib/dom/asynciterable.d.ts")),
);
const LIB_DOM_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "index.d.ts",
    include_str!(concat!("../../../lib/dom/index.d.ts")),
);
const LIB_DOM_ITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "iterable.d.ts",
    include_str!(concat!("../../../lib/dom/iterable.d.ts")),
);

const DOM_CANONICAL_EXPORTS: &[&str] = &[
    "AbortController",
    "Crypto",
    "Document",
    "Element",
    "Event",
    "EventTarget",
    "Headers",
    "History",
    "Location",
    "Navigator",
    "Performance",
    "Request",
    "Response",
    "Storage",
    "URL",
    "URLSearchParams",
    "Window",
    "alert",
    "cancelAnimationFrame",
    "clearInterval",
    "clearTimeout",
    "confirm",
    "console",
    "crypto",
    "document",
    "fetch",
    "history",
    "localStorage",
    "location",
    "navigator",
    "performance",
    "prompt",
    "queueMicrotask",
    "requestAnimationFrame",
    "sessionStorage",
    "setInterval",
    "setTimeout",
    "window",
];

pub const LIB_DOM: BuiltinLib = BuiltinLib::ambient_lib("dom", &[LIB_DOM_INDEX_D_DS], &["es5"])
    .with_canonical_exports(DOM_CANONICAL_EXPORTS);
pub const LIB_DOM_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "dom.asynciterable",
    &[LIB_DOM_ASYNCITERABLE_D_DS],
    &["dom", "es2018.asynciterable"],
);
pub const LIB_DOM_ITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "dom.iterable",
    &[LIB_DOM_ITERABLE_D_DS],
    &["dom", "es2015.iterable"],
);

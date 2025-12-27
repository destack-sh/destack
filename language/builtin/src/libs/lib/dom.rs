use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_DOM_ASYNCITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "asynciterable.d.ds",
    include_str!(concat!("../../../lib/dom/asynciterable.d.ds")),
);
const LIB_DOM_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "index.d.ds",
    include_str!(concat!("../../../lib/dom/index.d.ds")),
);
const LIB_DOM_ITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "dom",
    "iterable.d.ds",
    include_str!(concat!("../../../lib/dom/iterable.d.ds")),
);

pub const LIB_DOM: BuiltinLib = BuiltinLib::ambient("dom", &[LIB_DOM_INDEX_D_DS], &["es5"]);
pub const LIB_DOM_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient(
    "dom.asynciterable",
    &[LIB_DOM_ASYNCITERABLE_D_DS],
    &["dom", "es2018.asynciterable"],
);
pub const LIB_DOM_ITERABLE: BuiltinLib = BuiltinLib::ambient(
    "dom.iterable",
    &[LIB_DOM_ITERABLE_D_DS],
    &["dom", "es2015.iterable"],
);

// canonical exports used by compiler, not exhaustive
pub const DOM_CANONICAL_EXPORTS: &[&str] =
    &["Document", "Element", "Event", "EventTarget", "Window"];

use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_NODE_V18_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "node/v18",
    "index.d.ts",
    include_str!(concat!("../../../lib/node/v18/index.d.ts")),
);
const LIB_NODE_V20_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "node/v20",
    "index.d.ts",
    include_str!(concat!("../../../lib/node/v20/index.d.ts")),
);
const LIB_NODE_V22_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "node/v22",
    "index.d.ts",
    include_str!(concat!("../../../lib/node/v22/index.d.ts")),
);
const LIB_NODE_V24_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "node/v24",
    "index.d.ts",
    include_str!(concat!("../../../lib/node/v24/index.d.ts")),
);

const NODE_CANONICAL_EXPORTS: &[&str] = &[
    "console",
    "process",
    "setImmediate",
    "setInterval",
    "setTimeout",
];

pub const LIB_NODE: BuiltinLib = BuiltinLib::ambient(
    "node",
    &[LIB_NODE_V22_INDEX_D_DS],
    &["esnext", "undici-types.v6"],
)
.with_specifier_aliases(&[("undici-types", "undici-types.v6")])
.with_canonical_exports(NODE_CANONICAL_EXPORTS);
pub const LIB_NODE_V18: BuiltinLib = BuiltinLib::ambient(
    "node.v18",
    &[LIB_NODE_V18_INDEX_D_DS],
    &["esnext", "undici-types.v5"],
)
.with_specifier_aliases(&[("undici-types", "undici-types.v5")])
.with_canonical_exports(NODE_CANONICAL_EXPORTS);
pub const LIB_NODE_V20: BuiltinLib = BuiltinLib::ambient(
    "node.v20",
    &[LIB_NODE_V20_INDEX_D_DS],
    &["esnext", "undici-types.v6"],
)
.with_specifier_aliases(&[("undici-types", "undici-types.v6")])
.with_canonical_exports(NODE_CANONICAL_EXPORTS);
pub const LIB_NODE_V22: BuiltinLib = BuiltinLib::ambient(
    "node.v22",
    &[LIB_NODE_V22_INDEX_D_DS],
    &["esnext", "undici-types.v6"],
)
.with_specifier_aliases(&[("undici-types", "undici-types.v6")])
.with_canonical_exports(NODE_CANONICAL_EXPORTS);
pub const LIB_NODE_V24: BuiltinLib = BuiltinLib::ambient(
    "node.v24",
    &[LIB_NODE_V24_INDEX_D_DS],
    &["esnext", "undici-types.v7"],
)
.with_specifier_aliases(&[("undici-types", "undici-types.v7")])
.with_canonical_exports(NODE_CANONICAL_EXPORTS);

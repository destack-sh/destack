use super::super::source::{BuiltinLib, BuiltinLibSource};

// bun v1.3 sources
const LIB_BUN_V1_3_INDEX_D_TS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.3/index.d.ts")),
);
const LIB_BUN_V1_3_EXPECT_TYPE_INDEX: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3/vendor/expect-type",
    "index.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.3/vendor/expect-type/index.d.ts"
    )),
);
const LIB_BUN_V1_3_EXPECT_TYPE_BRANDING: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3/vendor/expect-type",
    "branding.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.3/vendor/expect-type/branding.d.ts"
    )),
);
const LIB_BUN_V1_3_EXPECT_TYPE_MESSAGES: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3/vendor/expect-type",
    "messages.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.3/vendor/expect-type/messages.d.ts"
    )),
);
const LIB_BUN_V1_3_EXPECT_TYPE_OVERLOADS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3/vendor/expect-type",
    "overloads.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.3/vendor/expect-type/overloads.d.ts"
    )),
);
const LIB_BUN_V1_3_EXPECT_TYPE_UTILS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.3/vendor/expect-type",
    "utils.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.3/vendor/expect-type/utils.d.ts"
    )),
);

// bun v1.2 sources
const LIB_BUN_V1_2_INDEX_D_TS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2",
    "index.d.ts",
    include_str!(concat!("../../../lib/bun/v1.2/index.d.ts")),
);
const LIB_BUN_V1_2_EXPECT_TYPE_INDEX: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2/vendor/expect-type",
    "index.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.2/vendor/expect-type/index.d.ts"
    )),
);
const LIB_BUN_V1_2_EXPECT_TYPE_BRANDING: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2/vendor/expect-type",
    "branding.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.2/vendor/expect-type/branding.d.ts"
    )),
);
const LIB_BUN_V1_2_EXPECT_TYPE_MESSAGES: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2/vendor/expect-type",
    "messages.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.2/vendor/expect-type/messages.d.ts"
    )),
);
const LIB_BUN_V1_2_EXPECT_TYPE_OVERLOADS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2/vendor/expect-type",
    "overloads.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.2/vendor/expect-type/overloads.d.ts"
    )),
);
const LIB_BUN_V1_2_EXPECT_TYPE_UTILS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "bun/v1.2/vendor/expect-type",
    "utils.d.ts",
    include_str!(concat!(
        "../../../lib/bun/v1.2/vendor/expect-type/utils.d.ts"
    )),
);

const BUN_V1_3_SOURCES: &[BuiltinLibSource] = &[
    LIB_BUN_V1_3_INDEX_D_TS,
    LIB_BUN_V1_3_EXPECT_TYPE_INDEX,
    LIB_BUN_V1_3_EXPECT_TYPE_BRANDING,
    LIB_BUN_V1_3_EXPECT_TYPE_MESSAGES,
    LIB_BUN_V1_3_EXPECT_TYPE_OVERLOADS,
    LIB_BUN_V1_3_EXPECT_TYPE_UTILS,
];

const BUN_V1_2_SOURCES: &[BuiltinLibSource] = &[
    LIB_BUN_V1_2_INDEX_D_TS,
    LIB_BUN_V1_2_EXPECT_TYPE_INDEX,
    LIB_BUN_V1_2_EXPECT_TYPE_BRANDING,
    LIB_BUN_V1_2_EXPECT_TYPE_MESSAGES,
    LIB_BUN_V1_2_EXPECT_TYPE_OVERLOADS,
    LIB_BUN_V1_2_EXPECT_TYPE_UTILS,
];

const BUN_SPECIFIER_ALIASES: &[(&str, &str)] = &[("undici-types", "undici-types.v7")];

pub const LIB_BUN: BuiltinLib = BuiltinLib::ambient_lib(
    "bun",
    BUN_V1_3_SOURCES,
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_3: BuiltinLib = BuiltinLib::ambient_lib(
    "bun.v1.3",
    BUN_V1_3_SOURCES,
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);
pub const LIB_BUN_V1_2: BuiltinLib = BuiltinLib::ambient_lib(
    "bun.v1.2",
    BUN_V1_2_SOURCES,
    &["esnext", "node.v24", "undici-types.v7"],
)
.with_specifier_aliases(BUN_SPECIFIER_ALIASES);

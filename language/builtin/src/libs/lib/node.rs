use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_NODE_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "node",
    "index.d.ds",
    include_str!(concat!("../../../lib/node/index.d.ds")),
);

pub const LIB_NODE: BuiltinLib = BuiltinLib::ambient("node", &[LIB_NODE_INDEX_D_DS], &["esnext"]);

use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_WORKER_ASYNCITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "asynciterable.d.ts",
    include_str!(concat!("../../../lib/worker/asynciterable.d.ts")),
);
const LIB_WORKER_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "index.d.ts",
    include_str!(concat!("../../../lib/worker/index.d.ts")),
);
const LIB_WORKER_IMPORTSCRIPTS_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "importscripts.d.ts",
    include_str!(concat!("../../../lib/worker/importscripts.d.ts")),
);
const LIB_WORKER_ITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "iterable.d.ts",
    include_str!(concat!("../../../lib/worker/iterable.d.ts")),
);

const WORKER_DECLARED_SYMBOLS: &[&str] = &[
    "Worker",
    "WorkerGlobalScope",
    "console",
    "clearInterval",
    "clearTimeout",
    "fetch",
    "queueMicrotask",
    "self",
    "setInterval",
    "setTimeout",
];
pub const LIB_WORKER: BuiltinLib =
    BuiltinLib::ambient_lib("worker", &[LIB_WORKER_INDEX_D_DS], &[])
        .with_declared_symbols(WORKER_DECLARED_SYMBOLS);
pub const LIB_WEBWORKER: BuiltinLib =
    BuiltinLib::ambient_lib("webworker", &[LIB_WORKER_INDEX_D_DS], &[]);
pub const LIB_WORKER_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "worker.asynciterable",
    &[LIB_WORKER_ASYNCITERABLE_D_DS],
    &[],
);
pub const LIB_WEBWORKER_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "webworker.asynciterable",
    &[LIB_WORKER_ASYNCITERABLE_D_DS],
    &[],
);
pub const LIB_WORKER_IMPORTSCRIPTS: BuiltinLib = BuiltinLib::ambient_lib(
    "worker.importscripts",
    &[LIB_WORKER_IMPORTSCRIPTS_D_DS],
    &[],
);
pub const LIB_WEBWORKER_IMPORTSCRIPTS: BuiltinLib = BuiltinLib::ambient_lib(
    "webworker.importscripts",
    &[LIB_WORKER_IMPORTSCRIPTS_D_DS],
    &[],
);
pub const LIB_WORKER_ITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "worker.iterable",
    &[LIB_WORKER_ITERABLE_D_DS],
    &[],
);
pub const LIB_WEBWORKER_ITERABLE: BuiltinLib = BuiltinLib::ambient_lib(
    "webworker.iterable",
    &[LIB_WORKER_ITERABLE_D_DS],
    &[],
);

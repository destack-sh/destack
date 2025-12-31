use super::super::source::{BuiltinLib, BuiltinLibSource};

const LIB_WORKER_ASYNCITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "asynciterable.d.ds",
    include_str!(concat!("../../../lib/worker/asynciterable.d.ds")),
);
const LIB_WORKER_INDEX_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "index.d.ds",
    include_str!(concat!("../../../lib/worker/index.d.ds")),
);
const LIB_WORKER_IMPORTSCRIPTS_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "importscripts.d.ds",
    include_str!(concat!("../../../lib/worker/importscripts.d.ds")),
);
const LIB_WORKER_ITERABLE_D_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "worker",
    "iterable.d.ds",
    include_str!(concat!("../../../lib/worker/iterable.d.ds")),
);

const WORKER_CANONICAL_EXPORTS: &[&str] = &["Worker", "WorkerGlobalScope"];
pub const LIB_WORKER: BuiltinLib =
    BuiltinLib::ambient("worker", &[LIB_WORKER_INDEX_D_DS], &["es5"])
        .with_canonical_exports(WORKER_CANONICAL_EXPORTS);
pub const LIB_WORKER_ASYNCITERABLE: BuiltinLib = BuiltinLib::ambient(
    "worker.asynciterable",
    &[LIB_WORKER_ASYNCITERABLE_D_DS],
    &["worker", "es2018.asynciterable"],
);
pub const LIB_WORKER_IMPORTSCRIPTS: BuiltinLib = BuiltinLib::ambient(
    "worker.importscripts",
    &[LIB_WORKER_IMPORTSCRIPTS_D_DS],
    &["worker"],
);
pub const LIB_WORKER_ITERABLE: BuiltinLib = BuiltinLib::ambient(
    "worker.iterable",
    &[LIB_WORKER_ITERABLE_D_DS],
    &["worker", "es2015.iterable"],
);

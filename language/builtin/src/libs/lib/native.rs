use super::super::source::{BuiltinLib, BuiltinLibSource};

/// Declared symbols that are ambient (available without explicit import).
/// These are the core data structures available in native/WASM targets.
pub(crate) const NATIVE_DECLARED_SYMBOLS: &[&str] = &[
    // string types
    "String",
    // array types
    "Array",
    "ReadonlyArray",
    "arrayOf",
    "arrayFill",
    // slice types
    "Slice",
    // simd types
    "Vector",
    // map types
    "Map",
    "Record",
    // set types
    "Set",
    // host namespaces
    "console",
    "process",
];

// root
const LIB_NATIVE_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native",
    "index.ds",
    include_str!("../../../lib/native/index.ds"),
);

// host/
const LIB_NATIVE_HOST_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/host",
    "index.ds",
    include_str!("../../../lib/native/host/index.ds"),
);

const LIB_NATIVE_HOST_CONSOLE_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/host/console",
    "index.ds",
    include_str!("../../../lib/native/host/console/index.ds"),
);

const LIB_NATIVE_HOST_CONSOLE_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/host/console",
    "console.ds",
    include_str!("../../../lib/native/host/console/console.ds"),
);

const LIB_NATIVE_HOST_PROCESS_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/host/process",
    "index.ds",
    include_str!("../../../lib/native/host/process/index.ds"),
);

const LIB_NATIVE_HOST_PROCESS_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/host/process",
    "process.ds",
    include_str!("../../../lib/native/host/process/process.ds"),
);

// string/
const LIB_NATIVE_STRING_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/string",
    "index.ds",
    include_str!("../../../lib/native/string/index.ds"),
);

const LIB_NATIVE_STRING_STRING_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/string",
    "string.ds",
    include_str!("../../../lib/native/string/string.ds"),
);

// collections/
const LIB_NATIVE_COLLECTIONS_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/collections",
    "index.ds",
    include_str!("../../../lib/native/collections/index.ds"),
);

const LIB_NATIVE_COLLECTIONS_ARRAY_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/collections",
    "array.ds",
    include_str!("../../../lib/native/collections/array.ds"),
);

const LIB_NATIVE_COLLECTIONS_SLICE_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/collections",
    "slice.ds",
    include_str!("../../../lib/native/collections/slice.ds"),
);

const LIB_NATIVE_COLLECTIONS_MAP_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/collections",
    "map.ds",
    include_str!("../../../lib/native/collections/map.ds"),
);

const LIB_NATIVE_COLLECTIONS_SET_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/collections",
    "set.ds",
    include_str!("../../../lib/native/collections/set.ds"),
);

// memory/
const LIB_NATIVE_MEMORY_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/memory",
    "index.ds",
    include_str!("../../../lib/native/memory/index.ds"),
);

const LIB_NATIVE_MEMORY_ALLOC_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/memory",
    "alloc.ds",
    include_str!("../../../lib/native/memory/alloc.ds"),
);

const LIB_NATIVE_MEMORY_MEMORY_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/memory",
    "memory.ds",
    include_str!("../../../lib/native/memory/memory.ds"),
);

const LIB_NATIVE_MEMORY_BUFFER_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/memory",
    "buffer.ds",
    include_str!("../../../lib/native/memory/buffer.ds"),
);

// sync/
const LIB_NATIVE_SYNC_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/sync",
    "index.ds",
    include_str!("../../../lib/native/sync/index.ds"),
);

// NOTE: atomic.ds moved to @core/intrinsic/atomic.ds

// math/
const LIB_NATIVE_MATH_INDEX_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/math",
    "index.ds",
    include_str!("../../../lib/native/math/index.ds"),
);

const LIB_NATIVE_MATH_BITS_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/math",
    "bits.ds",
    include_str!("../../../lib/native/math/bits.ds"),
);

const LIB_NATIVE_MATH_FLOAT_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/math",
    "float.ds",
    include_str!("../../../lib/native/math/float.ds"),
);

const LIB_NATIVE_MATH_CHECKED_DS: BuiltinLibSource = BuiltinLibSource::new(
    "lib",
    "native/math",
    "checked.ds",
    include_str!("../../../lib/native/math/checked.ds"),
);

pub const LIB_NATIVE: BuiltinLib = BuiltinLib::ambient_lib(
    "native",
    &[
        // root
        LIB_NATIVE_INDEX_DS,
        // host/
        LIB_NATIVE_HOST_INDEX_DS,
        LIB_NATIVE_HOST_CONSOLE_INDEX_DS,
        LIB_NATIVE_HOST_CONSOLE_DS,
        LIB_NATIVE_HOST_PROCESS_INDEX_DS,
        LIB_NATIVE_HOST_PROCESS_DS,
        // string/
        LIB_NATIVE_STRING_INDEX_DS,
        LIB_NATIVE_STRING_STRING_DS,
        // collections/
        LIB_NATIVE_COLLECTIONS_INDEX_DS,
        LIB_NATIVE_COLLECTIONS_ARRAY_DS,
        LIB_NATIVE_COLLECTIONS_SLICE_DS,
        LIB_NATIVE_COLLECTIONS_MAP_DS,
        LIB_NATIVE_COLLECTIONS_SET_DS,
        // memory/
        LIB_NATIVE_MEMORY_INDEX_DS,
        LIB_NATIVE_MEMORY_ALLOC_DS,
        LIB_NATIVE_MEMORY_MEMORY_DS,
        LIB_NATIVE_MEMORY_BUFFER_DS,
        // sync/
        LIB_NATIVE_SYNC_INDEX_DS,
        // NOTE: atomic.ds moved to @core/intrinsic/atomic.ds
        // math/
        LIB_NATIVE_MATH_INDEX_DS,
        LIB_NATIVE_MATH_BITS_DS,
        LIB_NATIVE_MATH_FLOAT_DS,
        LIB_NATIVE_MATH_CHECKED_DS,
    ],
    &[],
)
.with_declared_symbols(NATIVE_DECLARED_SYMBOLS);

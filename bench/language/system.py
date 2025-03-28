from bench.language.core import (
    BENCH_BENCH_ID,
    BENCH_BUILTIN_PACKAGE_ID,
    SYSTEM_BENCH_ID,
    SYSTEM_MAIN_PACKAGE_ID,
    NodeReference,
    NodeType,
)

# builtin benches (pointers) :Builtins
BENCH_BENCH_PTR = NodeReference(
    node_type=NodeType.BENCH, id=BENCH_BENCH_ID, bench_id=BENCH_BENCH_ID
)
BENCH_BUILTIN_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=BENCH_BUILTIN_PACKAGE_ID, bench_id=BENCH_BENCH_ID
)
SYSTEM_BENCH_PTR = NodeReference(
    node_type=NodeType.BENCH, id=SYSTEM_BENCH_ID, bench_id=SYSTEM_BENCH_ID
)
SYSTEM_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=SYSTEM_MAIN_PACKAGE_ID, bench_id=SYSTEM_BENCH_ID
)

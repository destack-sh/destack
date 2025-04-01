from bench.language.core import (
    BENCH_BENCH_PACKAGE_ID,
    BENCH_ID,
    SYSTEM_ID,
    SYSTEM_SYSTEM_PACKAGE_ID,
    NodeReference,
    NodeType,
)

# builtin benches (pointers) :Builtins
BENCH_PTR = NodeReference(node_type=NodeType.BENCH, id=BENCH_ID, bench_id=BENCH_ID)
BENCH_BENCH_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=BENCH_BENCH_PACKAGE_ID, bench_id=BENCH_ID
)
SYSTEM_BENCH_PTR = NodeReference(node_type=NodeType.BENCH, id=SYSTEM_ID, bench_id=SYSTEM_ID)
SYSTEM_MAIN_PACKAGE_PTR = NodeReference(
    node_type=NodeType.PACKAGE, id=SYSTEM_SYSTEM_PACKAGE_ID, bench_id=SYSTEM_ID
)

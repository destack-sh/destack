from typing import TYPE_CHECKING

from bench.language import Bench, Package
from bench.language.bench import Branch
from bench.language.const import LOADED_BENCH_NODE_TYPES, SOURCE_NODE_TYPES, BenchError

if TYPE_CHECKING:
    pass


class BenchRuntimeError(BenchError, RuntimeError):
    pass


class NotRunnableError(BenchRuntimeError):
    pass


class RunHaltedError(BenchRuntimeError):
    pass


BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*SOURCE_NODE_TYPES)
    .ancestors(Bench, Branch)
    .select_all()
    .exclude(Bench.encryption_key)
)

from typing import TYPE_CHECKING

from bench.language import Bench, Package
from bench.language.const import NodeType
from bench.utils.tenacity import RetryOptions, is_retryable_grpc_error

if TYPE_CHECKING:
    pass


LOADED_BENCH_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.SERVER,
    NodeType.STORE,
    NodeType.DRIVE,
    NodeType.CLIENT,
    NodeType.MACHINE,
    NodeType.PACKAGE,
)
LOADED_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = (
    NodeType.DEPENDENCY,
    NodeType.UPGRADE,
    NodeType.SPACE,
    NodeType.LINK,
    NodeType.NOTICE,
    NodeType.BLOCK,
    NodeType.TRIGGER,
    NodeType.FIELD,
    NodeType.QUERY,
    NodeType.STEP,
    NodeType.VIEW,
)
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench)
    .select_all()
    .exclude(Bench.encryption_key)
)

REMOTE_CONNECTION_RETRY = RetryOptions(
    max_attempts=-1,
    retry_if=is_retryable_grpc_error,
)
RUNTIME_PARALLELISM = 1

from typing import TYPE_CHECKING

from bench.language import Bench, Package
from bench.language.bench import Branch
from bench.language.const import NodeType
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass


LOADED_BENCH_NODE_TYPES: bittuple[NodeType] = bittuple(
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
LOADED_PACKAGE_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.PACKAGE,
    NodeType.DEPENDENCY,
    NodeType.UPGRADE,
    NodeType.SPACE,
    NodeType.LINK,
    NodeType.ISSUE,
    NodeType.BLOCK,
    NodeType.TRIGGER,
    NodeType.FIELD,
    NodeType.QUERY,
    NodeType.VIEW,
    NodeType.STEP,
    NodeType.BADGE,
    NodeType.ROLE,
    NodeType.IDENTITY,
    NodeType.MEMBERSHIP,
    NodeType.INVITE,
)
BENCH_QUERY = Bench.descendants(*LOADED_BENCH_NODE_TYPES).select_all()
PACKAGE_QUERY = (
    Package.descendants(*LOADED_PACKAGE_NODE_TYPES)
    .ancestors(Bench, Branch)
    .select_all()
    .exclude(Bench.encryption_key)
)

RUNTIME_CONCURRENCY = 1

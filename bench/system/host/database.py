from dataclasses import dataclass
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language.bench import Bench
from bench.language.block import Block
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.session import Session
from bench.sql.core import Column, Table
from bench.sql.engine import BenchContext
from bench.system.host.core import Commit, HostApi, HostPlugin
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class HostSqlContext(BenchContext):
    """HOst context for SQL operations (with custom databases)."""

    def get_table(self, block: Block) -> Table | None:
        raise NotImplementedError("nocheckin: get_table")

    def get_column(self, prop: Field) -> Column | None:
        raise NotImplementedError("nocheckin: get_column")


class DatabasePlugin(HostPlugin[Block | Field]):
    """Create and maintain custom Postgres tables for DatabaseBlocks."""

    watch_types = bittuple(NodeType.BLOCK, NodeType.FIELD)

    def __init__(self, host: HostApi, bench: "Bench"):
        super().__init__(host, bench)
        self.context = HostSqlContext(bench=bench)

    async def start(self) -> None:
        # synchronize schemas
        # nocheckin
        ...

    async def extend_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # create new databases
        for node in commit.added:
            if isinstance(node, Block) and node.type == BlockType.DATABASE:
                ...

        # nocheckin
        ...

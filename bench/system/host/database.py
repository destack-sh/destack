from dataclasses import dataclass
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language.bench import Bench
from bench.language.block import Block
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.session import Session
from bench.sql.core import Table
from bench.sql.graph import BENCH_RECORD_TABLE_PREFIX, BenchContext
from bench.sql.migration import introspect_sql_schema
from bench.system.graph.postgres import PostgresChannel
from bench.system.host.core import Commit, Host, HostPlugin
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class HostSqlContext(BenchContext):
    """HOst context for SQL operations (with custom databases)."""

    tables_by_block: dict[Block, Table]

    def get_table(self, block: Block) -> Table | None:
        if block.type == BlockType.DATABASE:
            table = self.tables_by_block.get(block)
            assert table is not None, f"no table for {block!r}"
            return table
        else:
            return None


class DatabasePlugin(HostPlugin[Block | Field]):
    """Create and maintain custom Postgres tables for DatabaseBlocks."""

    watch_types = bittuple(NodeType.BLOCK, NodeType.FIELD)

    def __init__(self, host: Host, bench: "Bench"):
        super().__init__(host, bench)
        self._tables_by_block: dict[Block, Table] = {}
        self._context = HostSqlContext(bench=bench, tables_by_block=self._tables_by_block)

    async def start(self) -> None:
        # synchronize schemas
        # nocheckin
        async with self.host.session(readonly=True) as session:
            channel = await session._get_channel_for(
                self.host.scope, NodeType.RECORD, expect=PostgresChannel
            )
            schema = await introspect_sql_schema(
                channel.cur, include_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,)
            )
        ...

    async def extend_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # create new databases
        for node in commit.added:
            if isinstance(node, Block) and node.type == BlockType.DATABASE:
                ...

        # nocheckin
        ...

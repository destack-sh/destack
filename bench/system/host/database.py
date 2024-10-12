from dataclasses import dataclass
from typing import TYPE_CHECKING
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.bench import Bench
from bench.language.block import Block
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.session import Session
from bench.sql.core import Table
from bench.sql.graph import BENCH_RECORD_TABLE_PREFIX, BenchSqlContext
from bench.sql.migration import introspect_sql_schema
from bench.system.graph.postgres import PostgresChannel
from bench.system.host.core import Commit, Host, HostPlugin
from bench.utils.func import bittuple

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class HostSqlContext(BenchSqlContext):
    """Host context for SQL operations (with custom databases)."""

    custom_tables_by_block: dict[Block, Table]
    databases_by_ck: dict[UUID, Block]

    def get_custom_table(self, block: UUID | Block) -> tuple[Table, Block]:
        if isinstance(block, UUID):
            block = self.databases_by_ck[block]
        if block.type == BlockType.DATABASE:
            table = self.custom_tables_by_block.get(block)
            assert table is not None, f"no table for {block!r}"
            return table, block
        else:
            raise RuntimeError(f"not a database block in {self.bench!r}: {block!r}")


class DatabasePlugin(HostPlugin[Block | Field]):
    """Create and maintain custom Postgres tables for DatabaseBlocks."""

    watch_types = bittuple(NodeType.BLOCK, NodeType.FIELD)

    def __init__(self, host: Host, bench: "Bench"):
        super().__init__(host, bench)
        self._tables_by_block: dict[Block, Table] = {}
        self._databases_by_ck: dict[UUID, Block] = {}
        self._context = HostSqlContext(
            bench=bench,
            custom_tables_by_block=self._tables_by_block,
            databases_by_ck=self._databases_by_ck,
        )

    async def start(self) -> None:
        # synchronize schemas
        # nocheckin
        async with self.host.session(readonly=True) as session:
            channel = await session._get_channel_for(
                self.host.scope, NodeType.RECORD, expect=PostgresChannel
            )
            schema = await introspect_sql_schema(
                channel.cur,
                include_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
                include_extensions=False,
            )
        ...

    async def extend_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # create new databases
        for node in commit.added:
            if isinstance(node, Block) and node.type == BlockType.DATABASE:
                ...

        # nocheckin
        ...

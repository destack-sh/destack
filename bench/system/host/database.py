from dataclasses import dataclass
from typing import TYPE_CHECKING
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Package
from bench.language.block import Block
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.session import Session
from bench.sql.core import Schema, Table
from bench.sql.graph import BENCH_RECORD_TABLE_PREFIX, BenchSqlContext, map_database_block_to_table
from bench.sql.migration import (
    MigrationOpType,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    introspect_sql_schema,
)
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
            if block not in self.databases_by_ck:
                raise RuntimeError(f"no database block for {block!r} in {self.bench!r}")
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

    def __init__(self, host: Host, bench: Bench, package: Package):
        super().__init__(host, bench)
        self.package = package
        self._tables_by_block: dict[Block, Table] = {}
        self._databases_by_ck: dict[UUID, Block] = {}
        self._context = HostSqlContext(
            bench=bench,
            custom_tables_by_block=self._tables_by_block,
            databases_by_ck=self._databases_by_ck,
        )

    async def start(self) -> None:
        # synchronize schemas
        # get target schema
        databases = [
            node
            for node in self.package._graph.nodes_of_type(Block)
            if node.type == BlockType.DATABASE
        ]
        for database in databases:
            table = map_database_block_to_table(database)
            self._tables_by_block[database] = table
            self._databases_by_ck[database.ck] = database
        new_schema = Schema(extensions=(), tables=tuple(self._tables_by_block.values()))

        # migrate from current to target schema
        # (usually nothing should happen here, but just in case we change something)
        async with self.host.session(readonly=True) as session:
            channel = await session._get_channel_for(
                self.host.scope, NodeType.RECORD, expect=PostgresChannel
            )
            old_schema = await introspect_sql_schema(
                channel.cur,
                include_table_prefixes=(BENCH_RECORD_TABLE_PREFIX,),
                exclude_table_prefixes=(),
                include_extensions=False,
            )
            migration_ops = generate_sql_migration_ops(
                old_schema=old_schema,
                new_schema=new_schema,
                include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
            )
            if migration_ops:
                await apply_sql_migration_ops(channel.cur, migration_ops)
                logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    async def extend_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # check if any databases were touched
        touched_databases: dict[UUID, Block] = {}
        for node in commit.edited:
            if isinstance(node, Block) and node.type == BlockType.DATABASE:
                touched_databases[node.ck] = node
            elif isinstance(node, Field):
                parent = node.parent
                if isinstance(parent, Block) and parent.type == BlockType.DATABASE:
                    touched_databases[parent.ck] = parent

        # migrate schema for touched databases (and only those)
        if touched_databases:
            old_tables = tuple(
                self._tables_by_block[block]
                for block in touched_databases.values()
                if block in self._tables_by_block
            )
            old_schema = Schema(extensions=(), tables=old_tables)
            new_tables_by_block: dict[Block, Table] = {
                block: map_database_block_to_table(block) for block in touched_databases.values()
            }
            new_schema = Schema(extensions=(), tables=tuple(new_tables_by_block.values()))
            migration_ops = generate_sql_migration_ops(
                old_schema=old_schema,
                new_schema=new_schema,
                include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
            )
            channel = await session._get_channel_for(
                self.host.scope, NodeType.RECORD, expect=PostgresChannel
            )
            await apply_sql_migration_ops(channel.cur, migration_ops)
            self._tables_by_block.update(new_tables_by_block)  # patch current tables
            logger.debug("database.migrate", host=self, migration_ops=migration_ops)

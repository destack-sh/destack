from dataclasses import dataclass
from typing import TYPE_CHECKING, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Package
from bench.language.block import Block
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.session import Session
from bench.sql.core import Schema, Table
from bench.sql.graph import (
    BENCH_RECORD_TABLE_PREFIX,
    BenchSqlContext,
    get_record_table_name,
    map_database_block_to_table,
)
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
    """
    Host context for SQL operations (with custom databases).
    NOTE :Architecture: ideally context like this should be per-branch (and even per-tx)
     (the same applies for other plugins where we need per-version state :IsolatedHostContext)
    """

    custom_tables_by_name: dict[str, Table]  # may include tables for deleted blocks
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
        self.context = HostSqlContext(
            bench=bench,
            custom_tables_by_name={},
            custom_tables_by_block={},
            databases_by_ck={},
        )

    def _init_context_from_blocks(self) -> None:
        databases = [
            node
            for node in self.package._graph.nodes_of_type(Block)
            if node.type == BlockType.DATABASE
        ]
        for database in databases:
            table = map_database_block_to_table(database)
            self.context.custom_tables_by_block[database] = table
            self.context.databases_by_ck[database.ck] = database

    async def start(self) -> None:
        # synchronize schemas
        # get target schema
        self._init_context_from_blocks()
        new_schema = Schema(
            extensions=(), tables=tuple(self.context.custom_tables_by_block.values())
        )

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
            for table in old_schema.tables:
                self.context.custom_tables_by_name[table.name] = table
            migration_ops = generate_sql_migration_ops(
                old_schema=old_schema,
                new_schema=new_schema,
                include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
            )
            if migration_ops:
                await apply_sql_migration_ops(channel.cur, migration_ops)
                logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    @override
    async def on_commit_prepare(self, session: Session, commit: Commit[Block | Field]) -> None:
        # check if any databases were touched
        touched_databases_by_ck: dict[UUID, Block] = {}
        for node in commit.edited:
            if isinstance(node, Block) and node.type == BlockType.DATABASE:
                touched_databases_by_ck[node.ck] = node
            elif isinstance(node, Field):
                parent = node.parent
                if isinstance(parent, Block) and parent.type == BlockType.DATABASE:
                    touched_databases_by_ck[parent.ck] = parent
        if not touched_databases_by_ck:
            return  # nothing to do

        # migrate schema for touched databases (and only those)
        channel = await session._get_channel_for(
            self.host.scope, NodeType.RECORD, expect=PostgresChannel
        )
        old_tables = []
        missing_blocks: list[Block] = []
        for block in touched_databases_by_ck.values():
            if block in self.context.custom_tables_by_block:
                old_tables.append(self.context.custom_tables_by_block[block])
            else:
                missing_blocks.append(block)
        if missing_blocks:
            # load missing blocks' current schema (in case they were restored)
            table_prefixes = tuple(get_record_table_name(block) for block in missing_blocks)
            old_schema = await introspect_sql_schema(
                channel.cur,
                include_table_prefixes=table_prefixes,
                exclude_table_prefixes=(),
                include_extensions=False,
            )
            for table in old_schema.tables:
                old_tables.append(table)
        old_schema = Schema(extensions=(), tables=tuple(old_tables))
        new_tables_by_block: dict[Block, Table] = {
            block: map_database_block_to_table(block) for block in touched_databases_by_ck.values()
        }
        new_schema = Schema(extensions=(), tables=tuple(new_tables_by_block.values()))
        migration_ops = generate_sql_migration_ops(
            old_schema=old_schema,
            new_schema=new_schema,
            include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
        )
        if migration_ops:
            await apply_sql_migration_ops(channel.cur, migration_ops)
        # patch context optimistically
        self.context.custom_tables_by_block.update(new_tables_by_block)
        self.context.databases_by_ck.update(touched_databases_by_ck)
        logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    @override
    async def on_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # actually remove tables for removed databases
        for node in commit.removed:
            if node.ck in self.context.databases_by_ck:
                del self.context.databases_by_ck[node.ck]
                del self.context.custom_tables_by_block[cast(Block, node)]

    @override
    async def on_commit_failed(self, session: Session, error: Exception) -> None:
        self._init_context_from_blocks()  # reset context

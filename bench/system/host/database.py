from dataclasses import dataclass
from typing import TYPE_CHECKING, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Bench, Block, BlockType, Field, NodeType, Package, Session
from bench.language.core import bittuple
from bench.sql import (
    BENCH_RECORD_TABLE_PREFIX,
    BenchSqlContext,
    MigrationOpType,
    Schema,
    Table,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    get_record_table_name,
    introspect_sql_schema,
    map_database_block_to_table,
)
from bench.system.graph import PostgresChannel
from bench.system.host.core import Commit, Host, HostPlugin

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
    custom_tables_by_database: dict[Block, Table]
    databases_by_ck: dict[UUID, Block]

    def get_custom_table(self, block: UUID | Block) -> tuple[Table, Block]:
        if isinstance(block, UUID):
            if block not in self.databases_by_ck:
                raise RuntimeError(f"no database block for {block!r} in {self.bench!r}")
            block = self.databases_by_ck[block]
        if block.type == BlockType.DATABASE:
            table = self.custom_tables_by_database.get(block)
            assert table is not None, f"no table for {block!r}"
            return table, block
        else:
            raise RuntimeError(f"not a database block in {self.bench!r}: {block!r}")


# NOTE :Architecture: automatically cleanup no longer used custom database tables/coumns


class DatabasePlugin(HostPlugin[Block | Field]):
    """Create and maintain custom Postgres tables for DatabaseBlocks."""

    watch_types = bittuple(NodeType.BLOCK, NodeType.FIELD)

    def __init__(self, host: Host, bench: Bench, package: Package):
        super().__init__(host, bench)
        self.package = package
        self.context = HostSqlContext(
            bench=bench,
            custom_tables_by_name={},
            custom_tables_by_database={},
            databases_by_ck={},
        )

    def _init_context_from_blocks(self) -> None:
        """Initialize the SQL context from the current package."""
        # NOTE :Incomplete: handle :ManyToManyRecords
        databases = [
            node
            for node in self.package._graph.nodes_of_type(Block)
            if node.type == BlockType.DATABASE
        ]
        for database in databases:
            old_table = self.context.custom_tables_by_database.get(database)
            new_table = map_database_block_to_table(database, old_table=old_table)
            self.context.custom_tables_by_database[database] = new_table
            self.context.databases_by_ck[database.ck] = database

    async def start(self) -> None:
        # synchronize schemas
        # get target schema
        self._init_context_from_blocks()
        new_schema = Schema(
            extensions=(), tables=tuple(self.context.custom_tables_by_database.values())
        )

        # migrate from current to target schema
        # (usually nothing should happen here, but just in case we change something)
        async with self.host.session(readonly=False) as session:
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
                session._touch_channel(channel)
                await session.commit()
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

        # load missing blocks' current schema (in case they were restored)
        old_tables = []
        restored_databases: list[Block] = []
        for block in touched_databases_by_ck.values():
            if block in self.context.custom_tables_by_database:
                old_tables.append(self.context.custom_tables_by_database[block])
            else:
                restored_databases.append(block)
        if restored_databases:
            table_prefixes = tuple(get_record_table_name(block) for block in restored_databases)
            old_schema = await introspect_sql_schema(
                channel.cur,
                include_table_prefixes=table_prefixes,
                exclude_table_prefixes=(),
                include_extensions=False,
            )
            for table in old_schema.tables:
                old_tables.append(table)
        old_schema = Schema(extensions=(), tables=tuple(old_tables))

        # get new schema and migrate
        new_tables_by_block: dict[Block, Table] = {}
        for block in touched_databases_by_ck.values():
            table_name = get_record_table_name(block)
            old_table = old_schema._tables_by_name.get(table_name)
            new_table = map_database_block_to_table(block, old_table=old_table)
            new_tables_by_block[block] = new_table
        new_schema = Schema(extensions=(), tables=tuple(new_tables_by_block.values()))
        migration_ops = generate_sql_migration_ops(
            old_schema=old_schema,
            new_schema=new_schema,
            include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
        )
        if migration_ops:
            await apply_sql_migration_ops(channel.cur, migration_ops)

        # patch context optimistically
        self.context.custom_tables_by_database.update(new_tables_by_block)
        self.context.databases_by_ck.update(touched_databases_by_ck)
        logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    @override
    async def on_commit(self, session: Session, commit: Commit[Block | Field]) -> None:
        # actually remove tables for removed databases
        for node in commit.removed:
            if node.ck in self.context.databases_by_ck:
                del self.context.databases_by_ck[node.ck]
                del self.context.custom_tables_by_database[cast(Block, node)]

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        # reset context
        self._init_context_from_blocks()

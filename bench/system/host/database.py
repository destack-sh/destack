from dataclasses import dataclass
from typing import TYPE_CHECKING, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Bench, Database, Field, NodeType, Package, Session, bittuple
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
    map_database_to_table,
)
from bench.system.graph import PostgresConnector
from bench.system.host.core import Commit, HostPlugin

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class HostSqlContext(BenchSqlContext):
    """
    Host context for SQL operations (with custom databases).
    NOTE :Architecture: ideally context like this should be per-branch (and even per-tx)
     (the same applies for other plugins where we need per-version state :IsolatedHostContext)
    """

    custom_tables_by_name: dict[str, Table]  # may include tables for deleted databases
    custom_tables_by_database: dict[Database, Table]
    databases_by_ck: dict[UUID, Database]

    def get_custom_table(self, database: UUID | Database) -> tuple[Table, Database]:
        if isinstance(database, UUID):
            if database not in self.databases_by_ck:
                raise RuntimeError(f"no database database for {database!r} in {self.bench!r}")
            database = self.databases_by_ck[database]

        table = self.custom_tables_by_database.get(database)
        assert table is not None, f"no table for {database!r}"
        return table, database


# NOTE :Architecture: automatically cleanup no longer used custom database tables/coumns


class DatabasePlugin(HostPlugin[Database | Field]):
    """Create and maintain custom Postgres tables for Databases."""

    watch_types = bittuple(NodeType.DATABASE, NodeType.FIELD)

    def __init__(self, host: "HostService", bench: Bench, package: Package):
        super().__init__(host, bench)
        self.package = package
        self.context = HostSqlContext(
            bench=bench,
            custom_tables_by_name={},
            custom_tables_by_database={},
            databases_by_ck={},
        )

    def _init_context_from_databases(self) -> None:
        """Initialize the SQL context from the current package."""
        for database in self.package._graph.nodes_of_type(Database):
            old_table = self.context.custom_tables_by_database.get(database)
            new_table = map_database_to_table(database, old_table=old_table)
            self.context.custom_tables_by_database[database] = new_table
            self.context.databases_by_ck[database.ck] = database

    async def start(self) -> None:
        # synchronize schemas
        # get target schema
        self._init_context_from_databases()
        new_schema = Schema(
            extensions=(), tables=tuple(self.context.custom_tables_by_database.values())
        )

        # migrate from current to target schema
        # (usually nothing should happen here, but just in case we change something)
        async with self.host.session(readonly=False) as session:
            connector = await session._get_connector_for(
                self.host.scope, NodeType.RECORD, expect=PostgresConnector
            )
            old_schema = await introspect_sql_schema(
                connector.cur,
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
                await apply_sql_migration_ops(connector.cur, migration_ops)
                session._touch_connector(connector)
                await session.commit()
                logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    @override
    async def on_commit_prepare(self, session: Session, commit: Commit[Database | Field]) -> None:
        # check if any databases were touched
        touched_databases_by_ck: dict[UUID, Database] = {}
        for node in commit.edited:
            if isinstance(node, Database):
                touched_databases_by_ck[node.ck] = node
            elif isinstance(node, Field):
                parent = node.parent
                if isinstance(parent, Database):
                    touched_databases_by_ck[parent.ck] = parent
        if not touched_databases_by_ck:
            return  # nothing to do

        # migrate schema for touched databases (and only those)
        connector = await session._get_connector_for(
            self.host.scope, NodeType.RECORD, expect=PostgresConnector
        )

        # load missing databases' current schema (in case they were restored)
        old_tables = []
        restored_databases: list[Database] = []
        for database in touched_databases_by_ck.values():
            if database in self.context.custom_tables_by_database:
                old_tables.append(self.context.custom_tables_by_database[database])
            else:
                restored_databases.append(database)
        if restored_databases:
            table_prefixes = tuple(
                get_record_table_name(database) for database in restored_databases
            )
            old_schema = await introspect_sql_schema(
                connector.cur,
                include_table_prefixes=table_prefixes,
                exclude_table_prefixes=(),
                include_extensions=False,
            )
            for table in old_schema.tables:
                old_tables.append(table)
        old_schema = Schema(extensions=(), tables=tuple(old_tables))

        # get new schema and migrate
        new_tables_by_database: dict[Database, Table] = {}
        for database in touched_databases_by_ck.values():
            table_name = get_record_table_name(database)
            old_table = old_schema._tables_by_name.get(table_name)
            new_table = map_database_to_table(database, old_table=old_table)
            new_tables_by_database[database] = new_table
        new_schema = Schema(extensions=(), tables=tuple(new_tables_by_database.values()))
        migration_ops = generate_sql_migration_ops(
            old_schema=old_schema,
            new_schema=new_schema,
            include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
        )
        if migration_ops:
            await apply_sql_migration_ops(connector.cur, migration_ops)

        # patch context optimistically
        self.context.custom_tables_by_database.update(new_tables_by_database)
        self.context.databases_by_ck.update(touched_databases_by_ck)
        logger.debug("database.migrate", host=self, migration_ops=migration_ops)

    @override
    async def on_commit(self, session: Session, commit: Commit[Database | Field]) -> None:
        # actually remove tables for removed databases
        for node in commit.removed:
            if node.ck in self.context.databases_by_ck:
                del self.context.databases_by_ck[node.ck]
                del self.context.custom_tables_by_database[cast(Database, node)]

    @override
    async def on_commit_failed(self, session: Session, error: BaseException) -> None:
        # reset context
        self._init_context_from_databases()

from dataclasses import dataclass
from typing import TYPE_CHECKING, cast, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language import Bench, Field, NodeType, Package, Session, Table, bittuple
from bench.sql import (
    BENCH_RECORD_TABLE_PREFIX,
    BenchSqlContext,
    MigrationOpType,
    SqlSchema,
    apply_sql_migration_ops,
    generate_sql_migration_ops,
    get_record_table_name,
    introspect_sql_schema,
)
from bench.sql import SqlTable as SqlTable
from bench.sql.graph import map_table_to_sql_table
from bench.system.graph import PostgresConnector
from bench.system.host import Commit, HostPlugin

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@dataclass(slots=True)
class HostSqlContext(BenchSqlContext):
    """
    Host context for SQL operations (with custom tables).
    NOTE :Architecture: ideally context like this should be per-branch (and even per-tx)
     (the same applies for other plugins where we need per-version state :IsolatedHostContext)
    """

    sql_tables_by_name: dict[str, SqlTable]  # may include tables for deleted tables
    sql_tables_by_table: dict[Table, SqlTable]
    tables_by_id: dict[UUID, Table]

    def get_sql_table(self, table: UUID | Table) -> tuple[SqlTable, Table]:
        if isinstance(table, UUID):
            if table not in self.tables_by_id:
                raise RuntimeError(f"no table for {table!r} in {self.bench!r}")
            table = self.tables_by_id[table]

        sql_table = self.sql_tables_by_table.get(table)
        assert sql_table is not None, f"no sql_table for {table!r}"
        return sql_table, table


# NOTE :Architecture: automatically cleanup no longer used custom table tables/coumns


class TablePlugin(HostPlugin[Table | Field]):
    """Create and maintain custom Postgres Tables."""

    watch_types = bittuple(NodeType.TABLE, NodeType.FIELD)

    def __init__(self, host: "HostService", bench: Bench, package: Package):
        super().__init__(host, bench)
        self.package = package
        self.context = HostSqlContext(
            bench=bench,
            sql_tables_by_name={},
            sql_tables_by_table={},
            tables_by_id={},
        )

    def _init_context_from_tables(self) -> None:
        """Initialize the SQL context from the current package."""
        for table in self.package._graph.nodes_of_type(Table):
            old_sql_table = self.context.sql_tables_by_table.get(table)
            new_sql_table = map_table_to_sql_table(table, prev_sql_table=old_sql_table)
            self.context.sql_tables_by_table[table] = new_sql_table
            self.context.tables_by_id[table.id] = table

    async def start(self) -> None:
        # synchronize schemas
        # get target schema
        self._init_context_from_tables()
        new_schema = SqlSchema(
            extensions=(), tables=tuple(self.context.sql_tables_by_table.values())
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
            for sql_table in old_schema.tables:
                self.context.sql_tables_by_name[sql_table.name] = sql_table
            migration_ops = generate_sql_migration_ops(
                old_schema=old_schema,
                new_schema=new_schema,
                include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
            )
            if migration_ops:
                await apply_sql_migration_ops(connector.cur, migration_ops)
                session._touch_connector(connector)
                await session.commit()
                logger.debug(
                    "table.migrate",
                    host=self,
                    connector=connector,
                    migration_ops=migration_ops,
                )

    @override
    async def pre_commit(self, session: Session, commit: Commit[Table | Field]) -> None:
        # check if any tables were touched
        touched_tables_by_id: dict[UUID, Table] = {}
        for node in commit.edited:
            if isinstance(node, Table):
                touched_tables_by_id[node.id] = node
            elif isinstance(node, Field):
                parent = node.parent
                if isinstance(parent, Table):
                    touched_tables_by_id[parent.id] = parent
        if not touched_tables_by_id:
            return  # nothing to do

        # migrate schema for touched tables (and only those)
        connector = await session._get_connector_for(
            self.host.scope, NodeType.RECORD, expect=PostgresConnector
        )

        # load missing tables' current schema (in case they were restored)
        old_sql_tables = []
        restored_tables: list[Table] = []
        for table in touched_tables_by_id.values():
            if table in self.context.sql_tables_by_table:
                old_sql_tables.append(self.context.sql_tables_by_table[table])
            else:
                restored_tables.append(table)
        if restored_tables:
            table_prefixes = tuple(get_record_table_name(table) for table in restored_tables)
            old_schema = await introspect_sql_schema(
                connector.cur,
                include_table_prefixes=table_prefixes,
                exclude_table_prefixes=(),
                include_extensions=False,
            )
            for sql_table in old_schema.tables:
                old_sql_tables.append(sql_table)
        old_schema = SqlSchema(extensions=(), tables=tuple(old_sql_tables))

        # get new schema and migrate
        new_sql_tables_by_table: dict[Table, SqlTable] = {}
        for table in touched_tables_by_id.values():
            table_name = get_record_table_name(table)
            old_sql_table = old_schema._tables_by_name.get(table_name)
            new_sql_table = map_table_to_sql_table(table, prev_sql_table=old_sql_table)
            new_sql_tables_by_table[table] = new_sql_table
        new_schema = SqlSchema(extensions=(), tables=tuple(new_sql_tables_by_table.values()))
        migration_ops = generate_sql_migration_ops(
            old_schema=old_schema,
            new_schema=new_schema,
            include_types=(MigrationOpType.CREATE, MigrationOpType.UPDATE),
        )
        if migration_ops:
            await apply_sql_migration_ops(connector.cur, migration_ops)
            session._touch_connector(connector)

        # patch context optimistically
        self.context.sql_tables_by_table.update(new_sql_tables_by_table)
        self.context.tables_by_id.update(touched_tables_by_id)
        logger.info("table.migrate", host=self, connector=connector, migration_ops=migration_ops)

    @override
    async def post_commit(self, session: Session, commit: Commit[Table | Field]) -> None:
        # actually remove tables for removed tables
        for node in commit.removed:
            if node.id in self.context.tables_by_id:
                del self.context.tables_by_id[node.id]
                del self.context.sql_tables_by_table[cast(Table, node)]

    @override
    async def post_commit_failed(self, session: Session, error: BaseException) -> None:
        # reset context
        self._init_context_from_tables()

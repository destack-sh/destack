import enum
from dataclasses import dataclass
from uuid import UUID

import psycopg
import structlog
from psycopg import sql

import bench.language as lang
from bench.language import ConditionalOp, HasDatabase, Module
from bench.language.const import MNT, TypeStorageFormat
from bench.language.expression import (
    ComparisonConditional,
    CompoundConditional,
    ExistenceConditional,
    QueryEngine,
    QueryEngineIncapableError,
)
from bench.sql.client import async_pg_cursor
from bench.sql.core import (
    BASE_RECORD_TABLE,
    CONSTRUCT_TABLE,
    INTERNAL_TABLES,
    Column,
    ColumnType,
    Constraint,
    Construct,
    ConstructInfo,
    ConstructKind,
    Index,
    SqlPrimitive,
    Table,
    get_record_table_name,
)

logger = structlog.get_logger(__name__)

COLUMN_TYPE_BY_STORAGE_FORMAT: dict[TypeStorageFormat, ColumnType] = {
    TypeStorageFormat.STRING: ColumnType.STRING,
    TypeStorageFormat.DOUBLE: ColumnType.FLOAT,
    TypeStorageFormat.LONG: ColumnType.BIGINT,
    TypeStorageFormat.VECTOR: ColumnType.VECTOR,
    TypeStorageFormat.BINARY: ColumnType.BINARY,
    TypeStorageFormat.DATE: ColumnType.DATETIME,
    TypeStorageFormat.BOOLEAN: ColumnType.BOOLEAN,
    TypeStorageFormat.KEYWORD: ColumnType.STRING,
    TypeStorageFormat.OBJECT: ColumnType.JSON,
}
assert len(COLUMN_TYPE_BY_STORAGE_FORMAT) == len(TypeStorageFormat), "missing column type"


def map_to_pg_column(field: lang.Field) -> Column:
    """Gets a column from a field. Later, there may be more than one column per field (?)."""
    column_type = COLUMN_TYPE_BY_STORAGE_FORMAT[field._storage_format]
    is_array = (
        field.flags & lang.TypeFlag.IS_ARRAY or field.flags & lang.TypeFlag.IS_ARRAYABLE
    ) and column_type != ColumnType.JSON
    return Column(
        name=field._source_key.replace(".", "_"),
        type=column_type,
        is_array=is_array,
    )


def map_to_pg_table(statement: lang.Statement) -> Table:
    """Gets the full table with all specific fields of a database and general record stuff."""
    columns = [map_to_pg_column(f) for f in statement.resolved_fields]
    indexes = []
    constraints = []

    return Table(
        name=get_record_table_name(statement.ck),
        columns=(*(c.clone() for c in BASE_RECORD_TABLE.columns), *columns),
        indexes=(*(i.clone() for i in BASE_RECORD_TABLE.indexes), *indexes),
        constraints=(*(c.clone() for c in BASE_RECORD_TABLE.constraints), *constraints),
    )


async def update_pg_schema(pg_name: str, module: Module) -> None:
    """Updates Postgres tables (i.e. schema) for a module's databases."""
    log = logger.bind(pg_name=pg_name, module=module)
    databases: list[lang.Statement] = [
        s
        for s in module._nodes
        if s.mnt == MNT.STATEMENT and HasDatabase in s._components and not s.ephemeral
    ]
    tables = (*INTERNAL_TABLES, *(s._table for s in databases if s._table))
    log.info("pg.update_schema", databases=len(databases), tables=len(tables))

    async with async_pg_cursor(pg_name, autocommit=False) as cur:
        # get missing constructs (diff existing and current)
        try:
            existing_constructs = await get_stored_pg_constructs(cur)
        except SqlUnknownConstruct:
            # TODO @Robustness @Architecture: figure out some simple Migration system
            # does not exist yet, will be created below
            await cur.connection.rollback()
            existing_constructs = {}
        current_constructs: dict[UUID, Construct] = {c.id: c for t in tables for c in t.walk()}
        missing_constructs = {
            id: c for id, c in current_constructs.items() if id not in existing_constructs
        }
        if missing_constructs:
            # create missing constructs
            await create_pg_constructs(cur, missing_constructs)
            # and remember the state
            new_constructs = {**existing_constructs}
            new_constructs.update(missing_constructs)  # retain all old constructs (for now)
            await replace_stored_pg_constructs(cur, new_constructs)


class PostgresConditionalOp(enum.StrEnum):
    # logical
    AND = "AND"
    OR = "OR"
    NOT = "NOT"
    # standard
    IS_NULL = "IS NULL"
    IS_NOT_NULL = "IS NOT NULL"
    EQ = "="
    NEQ = "!="
    LT = "<"
    LTE = "<="
    GT = ">"
    GTE = ">="
    IN = "IN"
    NOT_IN = "NOT IN"
    # string
    LIKE = "LIKE"
    NOT_LIKE = "NOT LIKE"
    ILIKE = "ILIKE"
    NOT_ILIKE = "NOT ILIKE"
    # array/json
    CONTAINS = "@>"
    CONTAINED_BY = "<@"
    OVERLAPS = "&&"


PG_CONDITIONAL_OP_BY_BENCH: dict[ConditionalOp, PostgresConditionalOp] = {
    # logical
    ConditionalOp.AND: PostgresConditionalOp.AND,
    ConditionalOp.OR: PostgresConditionalOp.OR,
    ConditionalOp.NOT: PostgresConditionalOp.NOT,
    # standard
    ConditionalOp.EXISTS: PostgresConditionalOp.IS_NOT_NULL,
    ConditionalOp.NOT_EXISTS: PostgresConditionalOp.IS_NULL,
    ConditionalOp.EQUALS: PostgresConditionalOp.EQ,
    ConditionalOp.NOT_EQUALS: PostgresConditionalOp.NEQ,
    ConditionalOp.LESS_THAN: PostgresConditionalOp.LT,
    ConditionalOp.LESS_THAN_OR_EQUALS: PostgresConditionalOp.LTE,
    ConditionalOp.GREATER_THAN: PostgresConditionalOp.GT,
    ConditionalOp.GREATER_THAN_OR_EQUALS: PostgresConditionalOp.GTE,
    # string
    ConditionalOp.MATCHES: PostgresConditionalOp.LIKE,
    ConditionalOp.STARTS_WITH: PostgresConditionalOp.LIKE,
    # containment
    ConditionalOp.CONTAINS: PostgresConditionalOp.CONTAINS,
    ConditionalOp.IN: PostgresConditionalOp.IN,
    ConditionalOp.NOT_IN: PostgresConditionalOp.NOT_IN,
}


class PostgresJoinOp(enum.StrEnum):
    INNER_JOIN = "INNER JOIN"
    LEFT_OUTER_JOIN = "LEFT OUTER JOIN"
    RIGHT_OUTER_JOIN = "RIGHT OUTER JOIN"
    FULL_OUTER_JOIN = "FULL OUTER JOIN"


class SqlException(Exception):
    pass


class SqlUnknownConstruct(SqlException):
    pass


@dataclass(frozen=True)
class SqlExpression:
    def sql(self) -> sql.Composable:
        raise NotImplementedError


SqlNode = SqlExpression | SqlPrimitive | sql.SQL


def sql_node_to_sql(node: SqlNode) -> sql.Composable:
    if isinstance(node, SqlExpression):
        return node.sql()
    elif isinstance(node, SqlPrimitive):
        return sql.Literal(node)
    elif isinstance(node, sql.SQL):
        return node
    else:
        raise TypeError(f"unexpected node type: {node}")


def _compile_field_ref(database: "HasDatabase", field: lang.Field) -> SqlNode:
    # nocheckin: compile :BuiltInFields refs (id, ck, created_at, last_edited_by_id, etc.)
    if database.ephemeral:
        return SqlJsonPath(path=["value", field._typed_key])
    else:
        return sql.Identifier(field._source_key.replace(".", "_"))


def compile_pg_conditional(
    database: "HasDatabase",
    cond: lang.Conditional | None,
) -> SqlNode:
    if isinstance(cond, CompoundConditional) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [compile_pg_conditional(database, c) for c in cond.clauses]
        return SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
    elif isinstance(cond, ComparisonConditional) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_field_ref(database, cond.field)
        right = SqlPrimitive(cond.value)
        return SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)
    elif isinstance(cond, ExistenceConditional):
        return SqlLogical(
            op=PG_CONDITIONAL_OP_BY_BENCH[cond.op],
            operand=compile_pg_conditional(database, cond.condition),
        )
    raise QueryEngineIncapableError(QueryEngine.POSTGRES, cond, "unsupported conditional")


def compile_pg_sort(
    database: "HasDatabase",
    sort: lang.Sort,
) -> SqlNode:
    field_ref = _compile_field_ref(database, sort.field)
    return sql.SQL("{} {}").format(field_ref, sql.SQL(sort.direction.value))


def compile_pg_sorts(
    database: "HasDatabase",
    sorts: list[lang.Sort],
) -> SqlNode:
    return sql.SQL(", ").join(compile_pg_sort(database, sort) for sort in sorts)


@dataclass(frozen=True)
class SqlJsonPath(SqlExpression):
    path: list[str]

    def sql(self) -> sql.Composable:
        return sql.SQL("->").join(sql.Literal(p) for p in self.path)


@dataclass(frozen=True)
class SqlComparison(SqlExpression):
    left: SqlNode
    op: PostgresConditionalOp
    right: SqlNode

    def sql(self) -> sql.Composable:
        return sql.SQL("{} {} {}").format(
            sql_node_to_sql(self.left),
            sql.SQL(self.op),
            sql_node_to_sql(self.right),
        )


@dataclass(frozen=True)
class SqlCompound(SqlExpression):
    op: PostgresConditionalOp
    operands: list[SqlNode]

    def sql(self) -> sql.Composable:
        return (
            sql.SQL(" {} ")
            .format(sql.SQL(self.op))
            .join(*(sql_node_to_sql(o) for o in self.operands))
        )


@dataclass(frozen=True)
class SqlLogical(SqlExpression):
    op: PostgresConditionalOp
    operand: SqlNode

    def sql(self) -> sql.Composable:
        return sql.SQL("{} ({})").format(sql.SQL(self.op), sql_node_to_sql(self.operand))


@dataclass(frozen=True)
class SqlJoin(SqlExpression):
    op: PostgresJoinOp
    foreign_table: Table | SqlNode
    condition: SqlNode

    def sql(self) -> sql.Composable:
        foreign_table = self.foreign_table
        if isinstance(foreign_table, Table):
            foreign_table = sql.Identifier(foreign_table.name)
        return sql.SQL("{} {} ON {}").format(
            sql.SQL(self.op),
            foreign_table,
            sql_node_to_sql(self.condition),
        )


RowIn = dict[str, SqlPrimitive | SqlExpression]
RowOut = dict[str, SqlPrimitive]


async def _do_execute(cur: psycopg.AsyncCursor, query: sql.Composed) -> None:
    try:
        await cur.execute(query)
    except (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn) as e:
        raise SqlUnknownConstruct(str(e)) from e


async def pg_select(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    columns: list[Column] | None = None,
    joins: list[SqlJoin] | None = None,
    where: SqlNode | None = None,
    order_by: SqlNode | None = None,
    first: int | None = None,
    skip: int | None = None,
) -> list[dict[str, any]]:
    """Selects from the given table."""
    columns = columns or table.columns
    query = sql.SQL("SELECT {fields} FROM {table}").format(
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in columns),
        table=sql.Identifier(table.name),
    )
    if joins:
        query += sql.SQL(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        query += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if order_by:
        query += sql.SQL(" ORDER BY {}").format(sql_node_to_sql(order_by))
    if first:
        query += sql.SQL(" LIMIT {}").format(sql.Literal(first))
    if skip:
        query += sql.SQL(" OFFSET {}").format(sql.Literal(skip))
    logger.debug("pg.select_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    return await cur.fetchall()


async def pg_count(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
) -> int:
    """Counts rows matching the given query."""
    query = sql.SQL("SELECT COUNT(*) FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    logger.debug("pg.count_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    return (await cur.fetchone())["count"]


async def pg_insert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Inserts into the given table."""
    values = (
        sql.SQL("(") + sql.SQL(", ").join(sql_node_to_sql(v) for v in row.values()) + sql.SQL(")")
        for row in rows
    )
    query = sql.SQL("INSERT INTO {table} ({fields}) VALUES {values}").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ").join(values),
    )
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.insert_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    if returning:
        return await cur.fetchall()


async def pg_exists(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlNode | None,
) -> bool:
    """Checks if rows matching the given query exist."""
    query = sql.SQL("SELECT EXISTS (SELECT 1 FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    query += sql.SQL(")")
    logger.debug("pg.exists_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    return (await cur.fetchone())["exists"]


async def pg_update(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlNode | None,
    values: RowIn | list[RowIn],
    *,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table."""
    if isinstance(values, dict):
        values = [values]
    query = sql.SQL("UPDATE {table} SET {fields}").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(
            sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
            for row in values
            for k, v in row.items()
        ),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.update_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    if returning:
        return await cur.fetchall()


async def pg_delete(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlNode | None,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Deletes from the given table."""

    query = sql.SQL("DELETE FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.delete_rows", table=table, query=query.as_string(cur))
    await _do_execute(cur, query)
    if returning:
        return await cur.fetchall()


async def pg_truncate(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Truncates the given table."""
    await cur.execute(sql.SQL("TRUNCATE TABLE {}").format(sql.Identifier(table.name)))


async def get_stored_pg_constructs(cur: psycopg.AsyncCursor) -> dict[UUID, ConstructInfo]:
    """Gets the current constructs in the given database (through the CONSTRUCT_TABLE)."""
    rows = await pg_select(cur, CONSTRUCT_TABLE)
    construct_infos = [
        ConstructInfo(
            id=row["id"],
            kind=ConstructKind(row["kind"]),
            name=row["name"],
            hash=row["hash"],
        )
        for row in rows
    ]
    return {c.id: c for c in construct_infos}


async def replace_stored_pg_constructs(cur: psycopg.AsyncCursor, constructs: dict[UUID, Construct]):
    """Replaces the STORED constructs in construct table (data only, no definitions)."""
    await pg_truncate(cur, CONSTRUCT_TABLE)
    rows = [
        {
            "id": construct.id,
            "kind": construct.kind.value,
            "name": construct.name,
            "hash": construct.hash,
        }
        for construct in constructs.values()
    ]
    await pg_insert(cur, CONSTRUCT_TABLE, rows)


async def create_pg_constructs(cur: psycopg.AsyncCursor, constructs: dict[UUID, Construct]):
    """Creates the given constructs in the given database (not an upsert!)."""
    for construct in constructs.values():
        # TODO @Performance: batch pg construct creation where possible
        if isinstance(construct, Table):
            await cur.execute(sql.SQL("CREATE TABLE {} ()").format(sql.Identifier(construct.name)))
        elif isinstance(construct, Column):
            await cur.execute(
                sql.SQL("ALTER TABLE {} ADD COLUMN {}").format(
                    sql.Identifier(construct.table.name), sql.SQL(construct.sql())
                )
            )
        elif isinstance(construct, Index):
            await cur.execute(
                sql.SQL("CREATE INDEX {} on {}").format(
                    sql.SQL(construct.sql()),
                    sql.Identifier(construct.table.name),
                )
            )
        elif isinstance(construct, Constraint):
            await cur.execute(
                sql.SQL("ALTER TABLE {} ADD CONSTRAINT {}").format(
                    sql.Identifier(construct.table.name),
                    sql.SQL(construct.sql()),
                )
            )
        else:
            raise RuntimeError(f"unexpected construct: {construct}")

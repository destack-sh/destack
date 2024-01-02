import base64
import enum
import struct
import typing
from dataclasses import dataclass
from itertools import chain
from typing import Any, Collection, Mapping, Optional, Sequence, cast
from uuid import UUID, uuid4

import cachetools
import msgpack
import psycopg
import structlog
from psycopg import sql
from psycopg.types.json import Jsonb

import bench.language as lang
from bench.language import ConditionalOp, Field, HasDatabase, Module, QueryEngine
from bench.language.const import EditKind, NodeType, TypeFlag, TypeStorageFormat
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    ExpressionOps,
    FieldReference,
    QueryEngineIncapableError,
)
from bench.language.module import UNSET, get_node_id
from bench.proto import wire
from bench.proto.wire import EditData
from bench.sql.client import GLOBAL_RO_PASSWORD, GLOBAL_RO_USERNAME, async_pg_cursor
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
    TableConstruct,
    get_record_table_name,
)
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import DEBUG, LOCAL

if typing.TYPE_CHECKING:
    from bench.proto.wiring import AnyNodeData

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
    TypeStorageFormat.RELATION: ColumnType.UUID,
}
assert len(COLUMN_TYPE_BY_STORAGE_FORMAT) == len(TypeStorageFormat), "missing column type"

CAST_TYPE_BY_STORAGE_FORMAT: dict[TypeStorageFormat, str] = {
    TypeStorageFormat.STRING: "text",
    TypeStorageFormat.DOUBLE: "float",
    TypeStorageFormat.LONG: "bigint",
    TypeStorageFormat.VECTOR: "float[]",
    TypeStorageFormat.BINARY: "bytea",
    TypeStorageFormat.DATE: "timestamptz",
    TypeStorageFormat.BOOLEAN: "boolean",
    TypeStorageFormat.KEYWORD: "text",
    TypeStorageFormat.OBJECT: "jsonb",
    TypeStorageFormat.RELATION: "uuid",
}


def get_value_column_name(typed_key: str, is_array: bool) -> str:
    typed_key = typed_key.replace(".", "_").replace("-", "_").lower()
    return f"value_{typed_key}"


def get_field_column_name(field: lang.Field) -> str:
    return get_value_column_name(field._typed_key, bool(field.flags & TypeFlag.IS_ARRAY))


def map_to_pg_column(field: lang.Field) -> Column:
    """Gets a column from a field. Later, there may be more than one column per field (?)."""
    column_type = COLUMN_TYPE_BY_STORAGE_FORMAT[field._storage_format]
    is_array = (
        field.flags & lang.TypeFlag.IS_ARRAY or field.flags & lang.TypeFlag.IS_ARRAYABLE
    ) and column_type != ColumnType.JSON
    return Column(
        source=str(field.ck),
        name=get_field_column_name(field),
        type=column_type,
        is_array=is_array,
        is_nullable=True,
    )


def map_to_pg_table(statement: lang.Statement) -> Table:
    """Gets the full table with all specific fields of a database and general record stuff."""
    columns = [map_to_pg_column(f) for f in statement.resolved_fields]
    indexes = []
    constraints = []

    return Table(
        source=str(statement.ck),
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
        if s.metatype == NodeType.STATEMENT and HasDatabase in s._components and not s.ephemeral
    ]
    tables = (*INTERNAL_TABLES, *(s._table for s in databases if s._table))
    log.info("pg.update_schema", databases=len(databases), tables=len(tables))

    try:
        async with async_pg_cursor(pg_name, autocommit=False) as cur:
            # get missing constructs (diff existing and current)
            try:
                existing_constructs = await pg_get_stored_constructs(cur)
            except SqlUndefinedConstruct:
                # TODO @Robustness @Architecture: figure out some simple Migration system
                # does not exist yet, will be created below
                await cur.connection.rollback()
                existing_constructs = {}
            current_constructs: dict[UUID, Construct] = {c.id: c for t in tables for c in t.walk()}
            missing_constructs = {
                id: c for id, c in current_constructs.items() if id not in existing_constructs
            }
            if missing_constructs:
                log.info("pg.update_schema.create", missing=len(missing_constructs))
                # create missing constructs
                await pg_create_constructs(cur, missing_constructs)
                # and remember the state
                new_constructs = {**existing_constructs}
                new_constructs.update(missing_constructs)  # retain all old constructs (for now)
                await pg_replace_stored_constructs(cur, new_constructs)
            else:
                log.info("pg.update_schema.skip")
    except Exception as e:
        log.exception("pg.update_schema.failed", e=e)
        raise RuntimeError(f"failed to update {pg_name} schema: {e}") from e


class PostgresConditionalOp(enum.StrEnum):
    # logical
    TRUE = "TRUE"
    FALSE = "FALSE"
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
    ConditionalOp.TRUE: PostgresConditionalOp.TRUE,
    ConditionalOp.FALSE: PostgresConditionalOp.FALSE,
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


class PostgresSortOp(enum.StrEnum):
    ASC = "ASC"
    DESC = "DESC"


POSTGRES_SORT_OP_BY_BENCH: dict[lang.SortOp, PostgresSortOp] = {
    lang.SortOp.ASCENDING: PostgresSortOp.ASC,
    lang.SortOp.DESCENDING: PostgresSortOp.DESC,
}


class SqlError(Exception):
    pass


class SqlUndefinedConstruct(SqlError):
    pass


@dataclass(frozen=True)
class SqlExpression:
    def sql(self) -> sql.Composable:
        raise NotImplementedError


SqlNode = SqlExpression | SqlPrimitive | sql.SQL


def sql_node_to_sql(node: SqlNode) -> sql.Composable:
    if isinstance(node, SqlExpression):
        return node.sql()
    elif isinstance(node, sql.Composable):
        return node
    else:
        return sql.Literal(node)


def _compile_field_ref(database: "HasDatabase", field: lang.Field | FieldReference) -> SqlNode:
    if isinstance(field, lang.Field):
        if field._reflected:
            return sql.Identifier(field.py_ident)
        elif database.ephemeral:
            return SqlJsonPath(sql.Identifier("value"), [field._typed_key])
        else:
            return sql.Identifier(get_field_column_name(field))
    elif isinstance(field, str):
        return sql.Identifier(field)
    else:
        raise TypeError(f"unexpected field ref: {field!r}")


def compile_pg_conditional(
    database: "HasDatabase",
    cond: lang.Expression | None,
) -> SqlNode:
    if cond.op == ConditionalOp.TRUE:
        return sql.SQL("TRUE")
    elif cond.op == ConditionalOp.FALSE:
        return sql.SQL("FALSE")
    elif cond.op in ExpressionOps.COND_LOGICAL and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [compile_pg_conditional(database, c) for c in cond.clauses]
        return SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
    elif (
        cond.op in ExpressionOps.COND_COMPARISON or cond.op in ExpressionOps.COND_STRING
    ) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_field_ref(database, cond.field or cond.field_key)
        if isinstance(cond.field, Field):  # add explicit cast to LHS if possible
            pg_type = CAST_TYPE_BY_STORAGE_FORMAT[cond.field._storage_format]
            left = sql.SQL("({})::{}").format(sql_node_to_sql(left), sql.SQL(pg_type))
        # map IN to ANY() construct (IN/NOT IN doesn't work in psycopg)
        if cond.op in (ConditionalOp.IN, ConditionalOp.NOT_IN):
            right = sql.SQL("ANY({})").format(sql.Literal(cond.value))
            op = (
                PostgresConditionalOp.EQ
                if cond.op == ConditionalOp.IN
                else PostgresConditionalOp.NEQ
            )
            return SqlComparison(left=left, op=op, right=right)

        if cond.op == ConditionalOp.STARTS_WITH:
            right = sql.SQL("{} || '%'").format(sql.Literal(cond.value))
        elif cond.op == ConditionalOp.MATCHES:
            right = sql.SQL("'%' || {} || '%'").format(sql.Literal(cond.value))
        else:
            right = sql.Literal(cond.value)

        return SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)
    elif cond.op in ExpressionOps.COND_EXISTENCE:
        return SqlUnary(
            left=_compile_field_ref(database, cond.field or cond.field_key),
            op=PG_CONDITIONAL_OP_BY_BENCH[cond.op],
        )
    raise QueryEngineIncapableError(QueryEngine.POSTGRES, cond, "unsupported conditional")


def compile_pg_sort(
    database: "HasDatabase",
    sort: lang.Expression,
) -> SqlNode:
    field_ref = _compile_field_ref(database, sort.field or sort.field_key)
    return sql.SQL("{} {}").format(
        sql_node_to_sql(field_ref), sql.SQL(POSTGRES_SORT_OP_BY_BENCH[sort.op])
    )


def compile_pg_sorts(
    database: "HasDatabase",
    sorts: list[lang.Expression],
) -> SqlNode:
    return sql.SQL(", ").join(compile_pg_sort(database, sort) for sort in sorts)


@dataclass(frozen=True)
class SqlJsonPath(SqlExpression):
    field: SqlNode
    path: list[str]

    def sql(self) -> sql.Composable:
        return sql.SQL("{}->{}").format(
            sql_node_to_sql(self.field),
            sql.SQL("->").join(sql.Literal(p) for p in self.path),
        )


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
        return sql.SQL(f" {self.op} ").join(sql_node_to_sql(o) for o in self.operands)


@dataclass(frozen=True)
class SqlUnary(SqlExpression):
    left: SqlNode
    op: PostgresConditionalOp

    def sql(self) -> sql.Composable:
        return sql.SQL("{} {}").format(sql_node_to_sql(self.left), sql.SQL(self.op))


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


def _wrap_pg_error(
    resource: Table | str, query: sql.Composed, e: psycopg.errors.Error
) -> Exception:
    if isinstance(e, (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn)):
        wrapped_t = SqlUndefinedConstruct
    else:
        wrapped_t = SqlError
    e_str = str(e)
    if "\n" in e_str:
        message = f"{e}\nin {resource!r}"
    else:
        message = f"{e} in {resource!r}"
    return wrapped_t(message)


async def _do_execute(
    cur: psycopg.AsyncCursor,
    resource: Table | str,
    query: sql.Composed,
    params: Sequence | Mapping | None = None,
) -> None:
    try:
        await cur.execute(query, params)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(resource, query, e) from e


async def _do_execute_many(
    cur: psycopg.AsyncCursor,
    resource: Table | str,
    query: sql.Composed,
    params: Sequence | Mapping | None = None,
    returning: bool = False,
) -> None:
    try:
        await cur.executemany(query, params, returning=returning)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(resource, query, e) from e


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
    params: Sequence | Mapping | None = None,
) -> list[dict[str, any]]:
    """Selects from the given table."""
    columns = columns or table.columns
    statement = pg_select_statement(
        table=table,
        columns=columns,
        joins=joins,
        where=where,
        order_by=order_by,
        first=first,
        skip=skip,
    )
    logger.debug("pg.select_rows", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, table, statement, params)
    return await cur.fetchall()


def pg_select_statement(
    table: Table,
    columns: list[Column],
    joins: list[SqlJoin] | None = None,
    where: SqlNode | None = None,
    order_by: SqlNode | None = None,
    first: int | None = None,
    skip: int | None = None,
):
    statement = sql.SQL("SELECT {fields} FROM {table}").format(
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in columns),
        table=sql.Identifier(table.name),
    )
    if joins:
        statement += sql.SQL(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if order_by:
        statement += sql.SQL(" ORDER BY {}").format(sql_node_to_sql(order_by))
    if first:
        statement += sql.SQL(" LIMIT {}").format(sql.Literal(first))
    if skip:
        statement += sql.SQL(" OFFSET {}").format(sql.Literal(skip))
    return statement


async def pg_count(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
) -> int:
    """Counts rows matching the given query."""
    statement = sql.SQL("SELECT COUNT(*) FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    logger.debug("pg.count_rows", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, table, statement)
    return (await cur.fetchone())["count"]


async def pg_exists(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    joins: list[SqlJoin] | None = None,
) -> bool:
    """Checks if rows matching the given query exist."""
    statement = sql.SQL("SELECT EXISTS (SELECT 1 FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if joins:
        statement += sql.SQL(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    statement += sql.SQL(")")
    logger.debug("pg.exists_rows", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, table, statement)
    return (await cur.fetchone())["exists"]


async def pg_insert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Inserts into the given table."""
    statement = sql.SQL("INSERT INTO {table} ({fields}) VALUES ({values})").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ".join(["%s"] * len(table.columns))),
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.insert_rows", table=table, query=sql_to_str(cur, statement))
    values = [tuple(row.get(c.name) for c in table.columns) for row in rows]
    await _do_execute_many(cur, table, statement, values, returning=bool(returning))
    if returning:
        return await cur.fetchall()


async def pg_update_static(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    static_value: RowIn,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with static values."""
    statement = sql.SQL("UPDATE {table} SET {values}").format(
        table=sql.Identifier(table.name),
        values=sql.SQL(", ").join(
            sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
            for k, v in static_value.items()
        ),
    )
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.update_rows.fixed", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, table, statement)
    if returning:
        return await cur.fetchall()


async def pg_update_list(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    static_values: RowIn,
    dynamic_columns: list[Column],
    dynamic_values: list[RowIn],
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with a list of values (corresponding to rows)."""
    assert not any(c.is_primary_key for c in dynamic_columns), f"primary key in {dynamic_columns}"
    table_name = sql.Identifier(table.name)
    # join fixed and dynamic values
    values_sql = sql.SQL(", ").join(
        chain(
            (
                sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
                for k, v in static_values.items()
            ),
            (sql.SQL("{} = %s").format(sql.Identifier(c.name)) for c in dynamic_columns),
        ),
    )
    statement = sql.SQL("UPDATE {table} SET {values} WHERE {pk} = %s").format(
        table=table_name, pk=sql.Identifier(table.primary_key.name), values=values_sql
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.SQL("{}").format(sql.Identifier(c.name)) for c in returning)
        )
    logger.debug(
        "pg.update_rows.list",
        table=table,
        query=sql_to_str(cur, statement),
        rows=len(dynamic_values),
    )
    dynamic_values = [
        (*(value.get(c.name) for c in dynamic_columns), value.get(table.primary_key.name))
        for value in dynamic_values
    ]
    await _do_execute_many(cur, table, statement, dynamic_values, returning=bool(returning))
    if returning:
        return await cur.fetchall()


async def pg_delete(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Deletes from the given table."""

    statement = sql.SQL("DELETE FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.delete_rows", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, table, statement)
    if returning:
        return await cur.fetchall()


async def pg_truncate(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Truncates the given table."""
    await cur.execute(sql.SQL("TRUNCATE TABLE {}").format(sql.Identifier(table.name)))


#
# Migrations
#


async def pg_get_stored_constructs(cur: psycopg.AsyncCursor) -> dict[UUID, ConstructInfo]:
    """Gets the current constructs in the given database (through the CONSTRUCT_TABLE)."""
    rows = await pg_select(cur, CONSTRUCT_TABLE)
    construct_infos = [
        ConstructInfo(
            id=row["id"],
            kind=ConstructKind(row["kind"]),
            table_name=row["table_name"],
            name=row["name"],
            hash=row["hash"],
        )
        for row in rows
    ]
    return {c.id: c for c in construct_infos}


async def pg_replace_stored_constructs(cur: psycopg.AsyncCursor, constructs: dict[UUID, Construct]):
    """Replaces the STORED constructs in construct table (data only, no definitions)."""
    await pg_truncate(cur, CONSTRUCT_TABLE)
    rows = [
        {
            "id": construct.id,
            "kind": construct.kind.value,
            "table_name": construct.table.name if isinstance(construct, TableConstruct) else None,
            "name": construct.name,
            "hash": construct.hash,
            "source": str(construct.source) if construct.source is not None else None,
        }
        for construct in constructs.values()
    ]
    await pg_insert(cur, CONSTRUCT_TABLE, rows)


async def pg_create_constructs(cur: psycopg.AsyncCursor, constructs: dict[UUID, Construct]):
    """Creates the given constructs in the given database (not an upsert!)."""
    for construct in constructs.values():
        # TODO @Performance: batch pg construct creation where possible
        if isinstance(construct, Table):
            statement = sql.SQL("CREATE TABLE {} ()").format(sql.Identifier(construct.name))
        elif isinstance(construct, Column):
            statement = sql.SQL("ALTER TABLE {} ADD COLUMN {}").format(
                sql.Identifier(construct.table.name), sql.SQL(construct.sql())
            )
        elif isinstance(construct, Index):
            statement = sql.SQL("CREATE INDEX {}").format(sql.SQL(construct.sql()))
        elif isinstance(construct, Constraint):
            statement = sql.SQL("ALTER TABLE {} ADD CONSTRAINT {}").format(
                sql.Identifier(construct.table.name),
                sql.SQL(construct.sql()),
            )
        else:
            raise RuntimeError(f"unexpected construct: {construct}")
        logger.debug("pg.create_construct", construct=construct, query=sql_to_str(cur, statement))
        await _do_execute(cur, "schema", statement)


#
# Record API
#
MAX_RECORD_TOTAL_VALUE_SIZE = 128 * 1024  # 128 KiB
MAX_RECORD_FIELD_VALUE_SIZE = 32 * 1024  # 32 KiB


def pg_pack_record_row(database: "HasDatabase", record: wire.RecordData) -> RowIn:
    """Packs a record into a row for Postgres (flattened for ephemeral)."""
    # check size
    # TODO @Performance: measure record value size more efficiently (than msgpack, also see below)
    msgpack_size = len(msgpack.packb(record.value))
    if msgpack_size > MAX_RECORD_TOTAL_VALUE_SIZE:
        raise ValueError(
            f"record {record.id} is too large: {msgpack_size} > {MAX_RECORD_TOTAL_VALUE_SIZE} bytes (consider storing large values in a Blob instead)"
        )

    # pack it up
    row = {
        "id": record.id,
        "ck": record.ck,
        "created_at": record.created_at,
        "created_by_id": None,
        "updated_at": record.updated_at,
        "deleted_at": record.deleted_at,
        "last_edited_at": record.last_edited_at,
        "last_edited_by_id": None,
        "revision": record.revision,
        "statement_key": database.key,
    }
    if database.ephemeral:
        row["statement_ck"] = database.ck
        row["statement_id"] = database.id
        row["value"] = Jsonb(record.value)
    else:
        for field in database.resolved_fields:
            column_name = get_field_column_name(field)
            value = record.value.get(field._typed_key)
            row[column_name] = pg_wrap_record_field_value(database, record, field, value)
    assert len(row) == len(
        database._table.columns
    ), f"row mismatch: {row.keys()} for {database._table!r}"
    return row


def pg_wrap_record_field_value(
    database: "HasDatabase", record: Optional[wire.RecordData], field: "Field", value: Any
) -> Any:
    # see https://www.psycopg.org/psycopg3/docs/basic/adapt.html
    if value is None:
        return None
    msgpack_size = len(msgpack.packb(value))
    if msgpack_size > MAX_RECORD_FIELD_VALUE_SIZE:
        record_str = f"record {record.id}" if record else "record"
        value_str = repr(value)
        if len(value_str) > 256:
            value_str = value_str[:196] + "..." + value_str[-56:]
        raise ValueError(
            f"{record_str} field value '{field.py_ident}' is too large: {msgpack_size} > {MAX_RECORD_FIELD_VALUE_SIZE} bytes (consider storing large values in a Blob instead)\nValue (truncated): {value_str}"
        )
    if field._storage_format == TypeStorageFormat.OBJECT:
        return Jsonb(value)
    elif field._storage_format == TypeStorageFormat.VECTOR:
        if isinstance(value, bytes):
            return value
        elif not isinstance(value, list):
            raise ValueError(f"unexpected vector value: {value}")
        else:
            # turn [-128, 127] into bytea
            return bytes(v + 128 for v in value)
    else:
        return value


def pg_unwrap_record_field_value(database: "HasDatabase", field: "Field", value: Any) -> Any:
    # see https://www.psycopg.org/psycopg3/docs/basic/adapt.html
    if value is None:
        return None
    elif field._storage_format == TypeStorageFormat.OBJECT:
        return value
    elif field._storage_format == TypeStorageFormat.VECTOR:
        # turn bytea into [-128, 127]
        return [v - 128 for v in value]
    else:
        return value


def pg_wrap_record_value(database: "HasDatabase", value: dict) -> dict:
    assert isinstance(value, dict), f"record value not a dict: {value}"
    if TYPE_DISCRIMINATOR_KEY in value:  # not stored in database (implicit in statement_key)
        del value[TYPE_DISCRIMINATOR_KEY]
    if database.ephemeral:  # lift into generic 'value' JSONB column
        value = {"value": sql.SQL("value || {}").format(sql.Literal(Jsonb(value)))}
    else:  # remap typed keys to column names
        value_columnized = {}
        for field in database.resolved_fields:
            v = value.get(field._typed_key, UNSET)
            if v is not UNSET:
                column_name = get_field_column_name(field)
                value_columnized[column_name] = pg_wrap_record_field_value(database, None, field, v)
        value = value_columnized
    return value


def pg_unpack_record_row(database: "HasDatabase", row: RowOut) -> wire.RecordData:
    if database.ephemeral:
        value = row["value"]
    else:
        value = {
            f._typed_key: pg_unwrap_record_field_value(database, f, row[get_field_column_name(f)])
            for f in database.resolved_fields
        }
    return wire.RecordData(
        id=row["id"],
        ck=row["ck"],
        created_at=row["created_at"],
        updated_at=row["updated_at"],
        deleted_at=row["deleted_at"],
        last_edited_at=row["last_edited_at"],
        last_changed_at=row["last_edited_at"],
        revision=row["revision"],
        parent_id=database.id,
        value=value,
    )


PgSelectRecordsResult = typing.NamedTuple(
    "PgSelectRecordsResult",
    [("records", list[wire.RecordData]), ("cursors", list[str]), ("start_cursor", str | None)],
)


async def pg_select_records(
    cur: psycopg.AsyncCursor,
    database: "HasDatabase",
    *,
    where: lang.Expression | None = None,
    sort: list[lang.Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> PgSelectRecordsResult:
    """Executes a select query on the given database."""
    if after:
        skip = (skip or 0) + int(decode_pg_cursor(after)) + 1  # 'after' is exclusive
    where = compile_pg_conditional(database, where) if where is not None else None
    sort = compile_pg_sorts(database, sort) if sort is not None else None
    rows = await pg_select(
        cur=cur, table=database._table, where=where, order_by=sort, first=first, skip=skip
    )
    records_data = [pg_unpack_record_row(database, row) for row in rows]
    cursors = [encode_pg_cursor(i) for i in range(skip or 0, (skip or 0) + len(records_data))]
    assert len(records_data) == len(cursors), f"unexpected cursors: {cursors} for {records_data}"
    return PgSelectRecordsResult(records_data, cursors, after)


@cachetools.cached({})
def encode_pg_cursor(i: int, exclusive: bool = True) -> str:
    if not exclusive:
        i -= 1  # include current element
    return base64.b64encode(struct.pack("q", i)).decode("ascii")


@cachetools.cached({})
def decode_pg_cursor(s: str) -> int:
    return struct.unpack("q", base64.b64decode(s))[0]


async def write_local_edits_to_pg(
    cur: psycopg.AsyncCursor,
    module: Module,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
    old_databases_by_id: dict[UUID, "HasDatabase"] | None = None,
) -> list["AnyNodeData"] | None:
    """
    Writes *local* edits to the database. Returns the updated nodes (i.e. records).
    Pass in databases for statements that are no longer in the module (i.e. deleted record parent).
    TODO @Performance: use psycopg3 pipelining to batch local edits
     see https://www.psycopg.org/psycopg3/docs/advanced/pipeline.html
    """
    if not edits:
        return []

    async def _write_record_edit_batch(
        edit_kind: EditKind, database_id: UUID, batch: list[EditData]
    ) -> list["AnyNodeData"] | None:
        database = module.lookup(database_id) or old_databases_by_id[database_id]
        table = database._table
        materialized_value_columns = tuple(c for c in table.columns if c.name.startswith("value_"))
        if edit_kind == EditKind.CREATE:
            records = cast(list[wire.RecordData], [edit.node for edit in batch])
            rows = [pg_pack_record_row(database, record) for record in records]
            _ = await pg_insert(cur=cur, table=table, rows=rows)
            return records
        elif edit_kind in (EditKind.UPDATE, EditKind.MOVE):
            properties = batch[
                0
            ].properties  # not strictly correct (should be union of all in batch)
            # expand value properties for materialized tables
            if not database.ephemeral and "value" in properties:
                properties = (
                    *(p for p in properties if p != "value"),
                    *(c.name for c in materialized_value_columns),
                )
            records = cast(list[wire.RecordData], [edit.node for edit in batch])
            now = utcnow_with_tz()
            row_values = []
            for record in records:
                # keep only properties touched in the edit
                row = {"id": record.id}
                for field in database.resolved_fields:  # all 'value' fields are considered changed
                    column_name = get_field_column_name(field)
                    value = record.value.get(field._typed_key)
                    row[column_name] = pg_wrap_record_field_value(database, record, field, value)
                row_values.append(row)
            # and update cru info :LocalRecordCru
            fixed_values = {
                "revision": sql.SQL("revision + 1"),
                "updated_at": now,
                "last_edited_at": now,
            }
            rows = await pg_update_list(
                cur=cur,
                table=table,
                static_values=fixed_values,
                dynamic_columns=[table.columns_by_name[k] for k in properties],
                dynamic_values=row_values,
                returning=table.columns if return_nodes else None,
            )
            return [pg_unpack_record_row(database, row) for row in rows] if return_nodes else []
        elif edit_kind in (EditKind.SOFT_DELETE, EditKind.RESTORE):
            records_ids = [edit.node.id for edit in batch]
            now = utcnow_with_tz()
            if edit_kind == EditKind.SOFT_DELETE:
                row = {"deleted_at": now}
            else:
                row = {"deleted_at": None}
            where = SqlComparison(
                sql.Identifier("id"),
                PostgresConditionalOp.EQ,
                sql.SQL("ANY({})").format(sql.Literal(records_ids)),
            )
            rows = await pg_update_static(
                cur=cur,
                table=table,
                where=where,
                static_value=row,
                returning=table.columns if return_nodes else None,
            )
            return [pg_unpack_record_row(database, row) for row in rows] if return_nodes else []
        elif edit_kind == EditKind.DELETE:
            records_ids = [edit.node.id for edit in batch]
            where = SqlComparison(
                sql.Identifier("id"),
                PostgresConditionalOp.EQ,
                sql.SQL("ANY({})").format(sql.Literal(records_ids)),
            )
            await pg_delete(cur=cur, table=table, where=where)
            return []
        else:
            raise RuntimeError(f"unexpected edit kind: {edit_kind} for {batch}")

    # batch operations by edit kind and database
    current_op: tuple[EditKind, UUID] = edits[0].kind, edits[0].node.parent_id
    current_batch: list[EditData] = []
    changed_nodes: list[AnyNodeData] = []
    for edit in edits:
        op = (edit.kind, edit.node.parent_id)
        if current_op != op:
            # new op, flush current batch
            edit_kind, database_id = current_op
            batch_nodes = await _write_record_edit_batch(edit_kind, database_id, current_batch)
            changed_nodes.extend(batch_nodes)
            # start new batch
            current_op = op
            current_batch = [edit]
        else:
            current_batch.append(edit)

    # flush last batch
    edit_kind, database_id = current_op
    batch_nodes = await _write_record_edit_batch(edit_kind, database_id, current_batch)
    changed_nodes.extend(batch_nodes)
    return changed_nodes if return_nodes else None


async def duplicate_records_in_pg(
    source_cur: psycopg.AsyncCursor,
    source_database: "HasDatabase",
    target_cur: psycopg.AsyncCursor,
    target_database: "HasDatabase",
    *,
    keep_cks: bool,
    where: lang.Expression,
    copy_revisions: bool,
    return_nodes: bool = False,
) -> list[wire.RecordData] | None:
    """Duplicates records across databases."""
    source_table = source_database._table
    target_table = target_database._table
    if source_database.ephemeral or target_database.ephemeral:
        raise ValueError(f"cannot duplicate ephemeral: {source_database!r}->{target_database!r}")
    if not target_table.columns_include(source_table):
        raise ValueError(f"target {target_table!r} is not superset of source {source_table!r}")
    target_module_id = target_database.module.id

    # TODO @Performance: duplicate records within same database directly in postgres
    where = where & lang.C(
        ConditionalOp.EQUALS, field_key="statement_key", value=source_database.key
    )
    log = logger.bind(source=source_database, target=target_database, where=where)
    log.debug("pg.duplicate_records", copy_revisions=copy_revisions, keep_cks=keep_cks)
    record_rows = await pg_select(
        cur=source_cur, table=source_table, where=compile_pg_conditional(source_database, where)
    )
    if record_rows:
        for record_row in record_rows:
            if not keep_cks:
                record_row["ck"] = uuid4()
            record_row["id"] = get_node_id(target_module_id, ck=record_row["ck"])
            record_row["statement_key"] = target_database.key
            if not copy_revisions:
                record_row["revision"] = 0
        await pg_insert(cur=target_cur, table=target_table, rows=record_rows)
    log.debug("pg.duplicate_records.done", rows=len(record_rows))

    if return_nodes:
        return [pg_unpack_record_row(target_database, row) for row in record_rows]
    else:
        return None


if DEBUG or LOCAL:
    # pretty print sql statements in dev mode
    def sql_to_str(c: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        import sqlparse

        s_str = s.as_string(c)
        return "\n" + sqlparse.format(s_str, reindent=True, keyword_case="upper") + "\n"

else:

    def sql_to_str(c: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        return s.as_string(c)


USER_PRIVILEGES = "SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES"


async def create_local_pg_database(
    *, pg_name: str, pg_username: str, pg_password: str, is_public: bool, upsert: bool
) -> None:
    """
    Creates the local Postgres database and corresponding roles/user for a bench.
    """
    log = logger.bind(pg_name=pg_name, upsert=upsert)
    log.info("pg.create_db")

    # create database from the default one (if not exists)
    async with async_pg_cursor("postgres", autocommit=True) as cur:
        await cur.execute("SELECT 1 FROM pg_database WHERE datname = %s", (pg_name,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_db.create")
            await cur.execute(sql.SQL("CREATE DATABASE {}").format(sql.Identifier(pg_name)))
        else:
            log.info("pg.create_db.already_exists")

    # connect to local database and setup auth
    async with async_pg_cursor(pg_name, autocommit=False) as cur:
        # create 'owner' user (if not exists)
        log.info("pg.create_db.create_owner", username=pg_username)
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (pg_username,))
        exists = bool(await cur.fetchone())
        if not exists:
            log.info("pg.create_db.create_owner.create", username=pg_username)
            await cur.execute(
                sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                    sql.Identifier(pg_username), sql.Literal(pg_password)
                ),
            )
        else:
            log.info("pg.create_db.create_owner.already_exists", username=pg_username)
        # grant full regular CRUD access to 'owner' user (no trigger or such)
        log.info("pg.create_db.create_owner.grant")
        # grant new
        await cur.execute(
            sql.SQL("GRANT {} ON ALL TABLES IN SCHEMA public TO {}").format(
                sql.SQL(USER_PRIVILEGES),
                sql.Identifier(pg_username),
            )
        )
        # alter default privileges (to apply to all new tables)
        await cur.execute(
            sql.SQL("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT {} ON TABLES TO {}").format(
                sql.SQL(USER_PRIVILEGES),
                sql.Identifier(pg_username),
            )
        )

        # if public, add global read only user (if not exists)
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (GLOBAL_RO_USERNAME,))
        exists = bool(await cur.fetchone())
        if is_public:
            if not exists:
                log.info("pg.create_db.create_global_ro.create")
                await cur.execute(
                    sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                        sql.Identifier(GLOBAL_RO_USERNAME), sql.Literal(GLOBAL_RO_PASSWORD)
                    ),
                )
            # grant read only
            log.info("pg.create_db.create_global_ro.grant")
            await cur.execute(
                sql.SQL("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME),
                )
            )
        elif exists:
            log.info("pg.create_db.create_global_ro.remove")
            await cur.execute(
                sql.SQL("REVOKE ALL ON SCHEMA PUBLIC FROM {}").format(
                    sql.Identifier(GLOBAL_RO_USERNAME)
                )
            )

    log.info("pg.create_db.done")


async def delete_local_pg_database(pg_name: str, pg_username: str) -> None:
    """
    Deletes the local Postgres database and corresponding roles/user for a bench.
    """
    log = logger.bind(pg_name=pg_name)
    log.info("pg.delete_db")

    # connect to default database and drop the database
    async with async_pg_cursor("postgres", autocommit=True) as cur:
        log.info("pg.delete_db.drop", username=pg_username)
        await cur.execute(sql.SQL("DROP DATABASE IF EXISTS {}").format(sql.Identifier(pg_name)))

    log.info("pg.delete_db.done")


async def write_host_edits_to_pg(
    cur: psycopg.AsyncCursor,
    module: Module,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
) -> list["AnyNodeData"] | None:
    raise NotImplementedError("nocheckin: write_host_edits_to_pg")

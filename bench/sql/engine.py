import base64
import enum
import struct
from dataclasses import dataclass
from typing import Collection, Mapping, Sequence, cast
from uuid import UUID

import cachetools
import psycopg
import structlog
from psycopg import sql
from psycopg.types.json import Jsonb

import bench.language as lang
from bench.language import ConditionalOp, Field, HasDatabase, Module, QueryEngine, wire
from bench.language.const import MNT, TypeStorageFormat
from bench.language.edit import EditData, EditKind
from bench.language.expression import (
    ComparisonConditional,
    CompoundConditional,
    ExistenceConditional,
    FieldReference,
    QueryEngineIncapableError,
    StaticConditional,
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
from bench.utils.dt import utcnow_with_tz
from bench.utils.utils import DEBUG, LOCAL

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
}


def _to_value_column_name(field: lang.Field) -> str:
    return f"value_{field._typed_key.replace('.', '_')}"


def map_to_pg_column(field: lang.Field) -> Column:
    """Gets a column from a field. Later, there may be more than one column per field (?)."""
    column_type = COLUMN_TYPE_BY_STORAGE_FORMAT[field._storage_format]
    is_array = (
        field.flags & lang.TypeFlag.IS_ARRAY or field.flags & lang.TypeFlag.IS_ARRAYABLE
    ) and column_type != ColumnType.JSON
    return Column(
        name=_to_value_column_name(field),
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
            existing_constructs = await pg_get_stored_constructs(cur)
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
            await pg_create_constructs(cur, missing_constructs)
            # and remember the state
            new_constructs = {**existing_constructs}
            new_constructs.update(missing_constructs)  # retain all old constructs (for now)
            await pg_replace_stored_constructs(cur, new_constructs)


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
    elif isinstance(node, sql.Composable):
        return node
    else:
        return sql.Literal(node)


def _compile_field_ref(database: "HasDatabase", field: lang.Field | FieldReference) -> SqlNode:
    if isinstance(field, lang.Field):
        if field.reflected:
            return sql.Identifier(field.py_ident)
        elif database.ephemeral:
            return SqlJsonPath(sql.Identifier("value"), [field._typed_key])
        else:
            return sql.Identifier(_to_value_column_name(field))
    else:
        return sql.Identifier(field)


def compile_pg_conditional(
    database: "HasDatabase",
    cond: lang.Conditional | None,
) -> SqlNode:
    if isinstance(cond, StaticConditional):
        return sql.SQL("TRUE" if cond.op == ConditionalOp.TRUE else "FALSE")
    elif isinstance(cond, CompoundConditional) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [compile_pg_conditional(database, c) for c in cond.clauses]
        return SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
    elif isinstance(cond, ComparisonConditional) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_field_ref(database, cond.field)
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
        else:
            right = sql.Literal(cond.value)

        return SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)
    elif isinstance(cond, ExistenceConditional):
        return SqlUnary(
            left=_compile_field_ref(database, cond.field), op=PG_CONDITIONAL_OP_BY_BENCH[cond.op]
        )
    raise QueryEngineIncapableError(QueryEngine.POSTGRES, cond, "unsupported conditional")


def compile_pg_sort(
    database: "HasDatabase",
    sort: lang.Sort,
) -> SqlNode:
    field_ref = _compile_field_ref(database, sort.field)
    return sql.SQL("{} {}").format(
        sql_node_to_sql(field_ref), sql.SQL(POSTGRES_SORT_OP_BY_BENCH[sort.op])
    )


def compile_pg_sorts(
    database: "HasDatabase",
    sorts: list[lang.Sort],
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


async def _do_execute(
    cur: psycopg.AsyncCursor, query: sql.Composed, params: Sequence | Mapping | None = None
) -> None:
    try:
        await cur.execute(query, params)
    except (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn) as e:
        raise SqlUnknownConstruct(str(e)) from e


async def _do_execute_many(
    cur: psycopg.AsyncCursor, query: sql.Composed, params: Sequence | Mapping | None = None
) -> None:
    try:
        await cur.executemany(query, params)
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
    await _do_execute(cur, statement, params)
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
    await _do_execute(cur, statement)
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
    await _do_execute(cur, statement)
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
    await _do_execute_many(cur, statement, values)
    if returning:
        return await cur.fetchall()


async def pg_update_static(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    values: RowIn,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with static values."""
    statement = sql.SQL("UPDATE {table} SET {values}").format(
        table=sql.Identifier(table.name),
        values=sql.SQL(", ").join(
            sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
            for k, v in values.items()
        ),
    )
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.update_rows", table=table, query=sql_to_str(cur, statement))
    await _do_execute(cur, statement)
    if returning:
        return await cur.fetchall()


async def pg_update_list(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    columns: list[Column],
    values: list[RowIn],
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with a list of values (corresponding to rows)."""
    assert any(c.is_primary_key for c in columns), f"no primary key in {columns}"
    table_name = sql.Identifier(table.name)
    # basically, just pipeline update single with executemany
    statement = sql.SQL("UPDATE {table} SET {values} WHERE {pk} = %s").format(
        table=table_name,
        pk=sql.Identifier(table.primary_key.name),
        values=sql.SQL(", ".join(["{} = %s"] * len(columns))).format(
            *(sql.Identifier(c.name) for c in columns)
        ),
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(
                sql.SQL("{}.{}").format(table_name, sql.Identifier(c.name)) for c in returning
            )
        )
    logger.debug("pg.update_rows", table=table, query=sql_to_str(cur, statement), rows=len(values))
    values = [
        (*(row.get(c.name) for c in columns), row.get(table.primary_key.name)) for row in values
    ]
    await _do_execute_many(cur, statement, values)
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
    await _do_execute(cur, statement)
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
            "name": construct.name,
            "hash": construct.hash,
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
        await _do_execute(cur, statement)


#
# Record API
#


def pack_record_row(database: "HasDatabase", record: wire.RecordData) -> RowIn:
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
        for field in database.fields:
            row[_to_value_column_name(field)] = record.value.get(field._typed_key)
    assert len(row) == len(database._table.columns), f"unexpected row: {row.keys()} for {database}"
    return row


def unpack_record_row(database: "HasDatabase", row: RowOut) -> wire.RecordData:
    if database.ephemeral:
        value = row["value"]
    else:
        value = {k: row[_to_value_column_name(f)] for k, f in database._fields_by_key.items()}
    return wire.RecordData(
        id=row["id"],
        ck=row["ck"],
        created_at=row["created_at"],
        updated_at=row["updated_at"],
        deleted_at=row["deleted_at"],
        last_edited_at=row["last_edited_at"],
        last_changed_at=row["last_edited_at"],
        revision=row["revision"],
        parent_id=row["statement_id"],
        value=value,
    )


async def pg_select_records(
    cur: psycopg.AsyncCursor,
    database: "HasDatabase",
    *,
    where: lang.Conditional | None = None,
    sort: list[lang.Sort] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> tuple[list[wire.RecordData], list[str]]:
    """Executes a select query on the given database."""
    if after:
        if skip is not None:
            raise ValueError("cannot specify both after and skip")
        skip = int(decode_pg_cursor(after))
    where = compile_pg_conditional(database, where) if where is not None else None
    sort = compile_pg_sorts(database, sort) if sort is not None else None
    rows = await pg_select(
        cur=cur, table=database._table, where=where, order_by=sort, first=first, skip=skip
    )
    records_data = [unpack_record_row(database, row) for row in rows]
    cursors = [encode_pg_cursor(i) for i in range(skip or 0, skip or 0 + len(records_data))]
    return records_data, cursors


@cachetools.cached({})
def encode_pg_cursor(i: int) -> str:
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
) -> list[wire.NodeData] | None:
    """
    Writes *local* edits to the database. Returns the updated nodes (i.e. records).
    Pass in databases for statements that are no longer in the module (i.e. deleted record parent).
    TODO @Performance: use psycopg3 pipelining to batch local edits
     see https://www.psycopg.org/psycopg3/docs/advanced/pipeline.html
    """
    if not edits:
        return []

    async def _write_edit_batch(
        edit_kind: EditKind, database_id: UUID, batch: list[EditData]
    ) -> list[wire.NodeData] | None:
        database = module.lookup(database_id) or old_databases_by_id[database_id]
        table = database._table
        if edit_kind == EditKind.CREATE:
            records = cast(list[wire.RecordData], [edit.node for edit in batch])
            rows = [pack_record_row(database, record) for record in records]
            _ = await pg_insert(cur=cur, table=table, rows=rows)
            return records
        elif edit_kind in (EditKind.UPDATE, EditKind.MOVE):
            properties = batch[0].properties  # not strictly correct (should be set)
            assert properties, f"no properties for {edit_kind} on {batch}"
            now = utcnow_with_tz()
            # update cru info :LocalRecordCru
            records = cast(list[wire.RecordData], [edit.node for edit in batch])
            rows = [pack_record_row(database, record) for record in records]
            # keep only properties touched in the edit
            rows = [{k: row[k] for k in properties} for row in rows]
            for record, row in zip(records, rows):
                row["id"] = record.id
                row["revision"] = sql.SQL("revision + 1")
                row["updated_at"] = now
                row["last_edited_at"] = now
            rows = await pg_update_list(
                cur=cur,
                table=table,
                values=rows,
                columns=[table.primary_key, *(table.columns_by_name[k] for k in properties)],
                returning=table.columns if return_nodes else None,
            )
            return [unpack_record_row(database, row) for row in rows] if return_nodes else []
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
                values=row,
                returning=table.columns if return_nodes else None,
            )
            return [unpack_record_row(database, row) for row in rows] if return_nodes else []
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
    changed_nodes: list[wire.NodeData] = []
    for edit in edits:
        op = (edit.kind, edit.node.parent_id)
        if current_op != op:
            # new op, flush current batch
            edit_kind, database_id = current_op
            batch_nodes = await _write_edit_batch(edit_kind, database_id, current_batch)
            changed_nodes.extend(batch_nodes)
            # start new batch
            current_op = op
            current_batch = [edit]
        else:
            current_batch.append(edit)

    # flush last batch
    edit_kind, database_id = current_op
    batch_nodes = await _write_edit_batch(edit_kind, database_id, current_batch)
    changed_nodes.extend(batch_nodes)
    return changed_nodes if return_nodes else None


if DEBUG or LOCAL:
    # pretty print sql statements in dev mode
    def sql_to_str(c: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        import sqlparse

        s_str = s.as_string(c)
        return "\n" + sqlparse.format(s_str, reindent=True, keyword_case="upper") + "\n"

else:

    def sql_to_str(c: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        return s.as_string(c)

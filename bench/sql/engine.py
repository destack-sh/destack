from dataclasses import dataclass
from functools import wraps
from itertools import chain
from typing import (
    Any,
    Callable,
    Collection,
    Iterable,
    Mapping,
    Sequence,
    cast,
)

import psycopg
import structlog
from fastuuid import UUID
from opentelemetry import trace
from psycopg import OperationalError, sql
from psycopg.types.json import Jsonb

from bench.language import TRACING, BenchError, NodeType, PrimitiveType, Table
from bench.utils.oracle import REAL_ORACLE
from bench.utils.tenacity import RetryOptions, retry

from .core import (
    PostgresConditionalOp,
    PostgresJoinOp,
    SqlColumn,
    SqlPrimitive,
    SqlTable,
)

# NOTE :Performance: check out asyncpg instead of psycopg (up to 5x faster?)
#  see https://github.com/MagicStack/asyncpg

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class SqlContext:
    def get_custom_table(self, table: "UUID | Table") -> tuple[SqlTable, Table]:
        raise NotImplementedError("context does not support custom tables")


@dataclass(slots=True)
class NullContext(SqlContext):
    pass


def _trace_pg_span[F: Callable](func: F) -> F:
    """Instruments a pg function with common parameters as span attributes"""

    if not TRACING:
        return func

    func_name = func.__name__
    if func_name.startswith("_"):
        func_name = func_name[1:]
    assert func_name.startswith("pg_"), f"unexpected pg function: {func.__name__}"

    @wraps(func)
    @tracer.start_as_current_span(f"postgres.{func_name[3:]}")
    async def wrapped(**kwargs):
        # extract out all the interesting attributes for the span
        cur = kwargs.get("cur")
        assert isinstance(cur, psycopg.AsyncCursor), f"bad cur for {func.__name__}: {cur!r}"
        span = trace.get_current_span()
        span.set_attribute("sql_url", get_sanitized_sql_url(cur.connection))
        span.set_attribute("connection_id", id(cur.connection))
        table = kwargs.get("table")
        if isinstance(table, SqlTable):
            span.set_attribute("table", table.name)
        node_type = kwargs.get("node_type")
        if node_type is not None:
            span.set_attribute("node_type", NodeType(node_type).bench_name)

        # forward call
        return await func(**kwargs)  # type: ignore

    return cast(F, wrapped)


def get_sanitized_sql_url(conn: psycopg.AsyncConnection) -> str:
    """Gets the SQL URL like postgresql://user:****@host:port/dbname."""
    pgconn = conn.pgconn
    host = pgconn.host.decode()
    port = pgconn.port.decode()
    user = pgconn.user.decode()
    db = pgconn.db.decode()
    return f"postgresql://{user}:****@{host}:{port}/{db}"


class SqlError(BenchError):
    def __init__(self, message: str, conn: psycopg.AsyncCursor | psycopg.AsyncConnection):
        if conn is not None:
            if isinstance(conn, psycopg.AsyncCursor):
                conn = conn.connection
            sql_url = get_sanitized_sql_url(conn)
            super().__init__(f"{sql_url}: {message} (conn={id(conn)})")
        else:
            super().__init__(message)


class SqlUndefinedObjectError(SqlError):
    pass


class SqlViolationError(SqlError):
    pass


class SqlAlreadyExistsError(SqlError):
    pass


class SqlNotExistsError(SqlError):
    pass


class SqlConnectionError(SqlError):
    pass


@dataclass(frozen=True)
class SqlExpression:
    def sql(self) -> sql.Composable:
        raise NotImplementedError


SqlNode = (
    SqlExpression
    | SqlPrimitive
    | sql.SQL
    | sql.Identifier
    | sql.Literal
    | sql.Composed
    | sql.Composable
    | Jsonb
)


def sql_to_str(cur: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
    s_str = s.as_string(cur)
    return s_str


def sqljoin(sep: str, args: Iterable[SqlNode]) -> sql.Composed:
    return sqlstr(sep).join(sql_node_to_sql(a) for a in args)


def sql_node_to_sql(node: SqlNode) -> sql.Composable:
    if isinstance(node, SqlExpression):
        return node.sql()
    elif isinstance(node, sql.Composable):
        return node
    else:
        return sql.Literal(node)


def sqlstr(str: str) -> sql.SQL:
    # don't care about LiteralString
    return sql.SQL(str)  # type: ignore


def sqlliteral(value: Any) -> sql.Literal:
    return sql.Literal(value)


def sqlident(name: str) -> sql.Identifier:
    return sql.Identifier(name)  # type: ignore


@dataclass(frozen=True)
class SqlJsonPath(SqlExpression):
    field: SqlNode
    path: list[str]

    def sql(self) -> sql.Composable:
        return sqlstr("{}->{}").format(
            sql_node_to_sql(self.field),
            sqlstr("->").join(sql.Literal(p) for p in self.path),
        )


@dataclass(frozen=True)
class SqlComparison(SqlExpression):
    left: SqlNode
    op: PostgresConditionalOp
    right: SqlNode

    def sql(self) -> sql.Composable:
        return sqlstr("{} {} {}").format(
            sql_node_to_sql(self.left),
            sqlstr(self.op),
            sql_node_to_sql(self.right),
        )


@dataclass(frozen=True)
class SqlCompound(SqlExpression):
    op: PostgresConditionalOp
    operands: list[SqlNode]

    def sql(self) -> sql.Composable:
        inner = sqlstr(f" {self.op} ").join(sql_node_to_sql(o) for o in self.operands)
        return sqlstr("({})").format(inner)


@dataclass(frozen=True)
class SqlUnary(SqlExpression):
    left: SqlNode
    op: PostgresConditionalOp

    def sql(self) -> sql.Composable:
        return sqlstr("{} {}").format(sql_node_to_sql(self.left), sqlstr(self.op))


@dataclass(frozen=True)
class SqlJoin(SqlExpression):
    op: PostgresJoinOp
    foreign_table: SqlTable | SqlNode
    condition: SqlNode

    def sql(self) -> sql.Composable:
        foreign_table = self.foreign_table
        if isinstance(foreign_table, SqlTable):
            foreign_table = sqlident(foreign_table.name)
        return sqlstr("{} {} ON {}").format(
            sqlstr(self.op),
            foreign_table,
            sql_node_to_sql(self.condition),
        )


RowIn = Mapping[str, SqlPrimitive | SqlExpression | SqlNode]
RowOut = Mapping[str, SqlPrimitive]


def _pg_wrap_error(
    resource: Any, conn: psycopg.AsyncCursor | psycopg.AsyncConnection, e: psycopg.errors.Error
) -> Exception:
    if isinstance(e, (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn)):
        wrapped_t = SqlUndefinedObjectError
    elif isinstance(e, (psycopg.errors.UniqueViolation,)):
        wrapped_t = SqlAlreadyExistsError
    elif "Violation" in e.__class__.__name__:
        wrapped_t = SqlViolationError
    elif isinstance(e, (psycopg.errors.OperationalError, psycopg.errors.InterfaceError)):
        wrapped_t = SqlConnectionError
    else:
        wrapped_t = SqlError
    message = f"{e}\nin {resource!r}" if "\n" in str(e) else f"{e} in {resource!r}"
    return wrapped_t(message, conn)


# NOTE: we retry only on operational PG errors to handle transient issues (e.g. network)
#  (this does *not* handle actual disconnects like due to Postgres restarts)
RETRY_PG = RetryOptions(max_attempts=2, max_retry_interval=5, retry_on=(OperationalError,))


@retry(RETRY_PG, REAL_ORACLE)
async def _pg_execute(
    cur: psycopg.AsyncCursor,
    statement: sql.Composed,
    params: Mapping | None = None,
):
    await cur.execute(statement, params)


@retry(RETRY_PG, REAL_ORACLE)
async def _pg_executemany(
    cur: psycopg.AsyncCursor,
    statement: sql.Composed,
    params: Iterable[Mapping],
    returning: bool = False,
):
    await cur.executemany(statement, params, returning=returning)


def _pg_wrap_write_column(column: SqlColumn, value: SqlNode) -> SqlNode:
    return value


def _pg_wrap_read_column(ctx: SqlContext, column: SqlColumn, value: SqlNode) -> SqlNode:
    return value


def _pg_adapt_row(table: SqlTable, row: Mapping[str, Any]) -> Mapping[str, Any]:
    """Adapts and wraps Any values"""
    wrapped = {}
    for column in table.columns:
        if column.name not in row:
            continue
        value = row.get(column.name)
        if value is None:
            pass
        elif column.type == PrimitiveType.JSON:
            value = [Jsonb(v) for v in value] if column.is_array else Jsonb(value)
        wrapped[column.name] = value
    return wrapped


def _pg_adapt_rows(
    table: SqlTable, rows: Iterable[Mapping[str, Any]]
) -> tuple[Mapping[str, Any], ...]:
    return tuple(_pg_adapt_row(table, row) for row in rows)


async def _pg_fetchall_from_many(cur: psycopg.AsyncCursor, expected: int) -> list[RowOut]:
    # see https://www.psycopg.org/psycopg3/docs/api/cursors.html#psycopg.Cursor.executemany
    results: list[RowOut] = []
    while True:
        row = cast(RowOut | None, await cur.fetchone())
        assert row, f"expected {expected} results, got {len(results)}: {row!r}"
        results.append(row)
        if not cur.nextset():
            break
    assert len(results) == expected, f"wanted {expected} results, got {len(results)}"
    return results


@_trace_pg_span
async def pg_select_raw(
    *, cur: psycopg.AsyncCursor, query: str | sql.Composed
) -> list[dict[str, Any]]:
    """Executes an arbitrary select without any wrapping."""
    query_str = sql_to_str(cur, query) if not isinstance(query, str) else query
    trace.get_current_span().set_attributes({"sql_query": query_str})
    logger.trace("postgres.select_raw", query=query_str, span="current")
    await _pg_execute(cur, cast(sql.Composed, query))
    return cast(list[dict[str, Any]], await cur.fetchall())


@_trace_pg_span
async def pg_select(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    columns: Collection[SqlColumn] | None = None,
    joins: Collection[SqlJoin] | None = None,
    where: SqlNode | None = None,
    order_by: SqlNode | None = None,
    first: int | None = None,
    skip: int | None = None,
    params: Mapping | None = None,
) -> Sequence[RowOut]:
    """Selects from the given table."""
    columns = columns or table.columns
    statement = sqlstr("SELECT {fields} FROM {table}").format(
        fields=sqljoin(", ", (_pg_wrap_read_column(ctx, c, sqlident(c.name)) for c in columns)),
        table=sqlident(table.name),
    )
    if joins:
        statement += sqlstr(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    if order_by:
        statement += sqlstr(" ORDER BY {}").format(sql_node_to_sql(order_by))
    if first:
        statement += sqlstr(" LIMIT {}").format(sql.Literal(first))
    if skip:
        statement += sqlstr(" OFFSET {}").format(sql.Literal(skip))
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("postgres.select", table=table, cur=cur, query=query_str, span="current")
    try:
        await _pg_execute(cur, statement, params)
        rows = await cur.fetchall()
        return cast(Sequence[RowOut], rows)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_count(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    where: SqlNode | None = None,
) -> int:
    """Counts rows matching the given query."""
    statement = sqlstr("SELECT COUNT(*) FROM {table}").format(
        table=sqlident(table.name),
    )
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    query = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query)
    logger.trace("postgres.count", table=table, cur=cur, query=query)
    try:
        await _pg_execute(cur, statement)
        result = cast(dict, await cur.fetchone())
        if not result:
            raise SqlError(f"no result for {table!r}", cur)
        return result["count"]
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_exists(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    where: SqlNode | None = None,
    joins: list[SqlJoin] | None = None,
) -> bool:
    """Checks if rows matching the given query exist."""
    statement = sqlstr("SELECT EXISTS (SELECT 1 FROM {table}").format(
        table=sqlident(table.name),
    )
    if joins:
        statement += sqlstr(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    statement += sqlstr(")")
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("postgres.exists_rows", table=table, cur=cur, query=query_str, span="current")
    try:
        await _pg_execute(cur, statement)
        result = cast(dict, await cur.fetchone())
        if not result:
            raise SqlError(f"no result for {table!r}", cur)
        return result["exists"]
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_insert(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    rows: Collection[RowIn],
) -> None:
    """Inserts into the given table. Expects rows to be adapted and wrapped."""
    statement = sqlstr("INSERT INTO {table} ({fields}) VALUES ({values})").format(
        table=sqlident(table.name),
        fields=sqljoin(", ", (sqlident(c.name) for c in table.columns)),
        values=sqljoin(
            ", ", (_pg_wrap_write_column(c, sqlstr(f"%({c.name})s")) for c in table.columns)
        ),
    )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("postgres.insert", table=table, cur=cur, query=query_str, span="current")

    columns = table.columns
    templated_values = []
    for row in rows:
        row = {**row}
        for column in columns:
            if column.name not in row:
                row[column.name] = None  # ensure all columns are set
        templated_values.append(row)
    try:
        await _pg_executemany(cur, statement, templated_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_upsert(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    rows: Collection[RowIn],
    conflict_columns: list[SqlColumn] | tuple[SqlColumn, ...] | None = None,
    static_columns: list[SqlColumn] | tuple[SqlColumn, ...] | None = None,
    static_values: RowIn | None = None,
) -> None:
    """Upserts into the given table. Expect rows to be adapted and wrapped."""
    if conflict_columns is None:
        assert table._primary_key, f"no primary key for {table!r}"
        conflict_columns = (table._primary_key,)
    if static_columns is None:
        static_columns = tuple(c for c in table.columns if c not in conflict_columns)
    if static_values:
        static_update_columns = tuple(c for c in static_columns if c.name not in static_values)
    else:
        static_update_columns = static_columns

    static_update = tuple(
        sqlstr("{} = EXCLUDED.{}").format(sqlident(c.name), sqlident(c.name))
        for c in static_update_columns
    )
    if static_values:
        dynamic_update = tuple(
            sqlstr("{} = {}").format(sqlident(k), sql_node_to_sql(v))
            for k, v in static_values.items()
        )
    else:
        dynamic_update = ()

    statement = sqlstr(
        "INSERT INTO {table} ({fields}) VALUES ({values}) ON CONFLICT ({conflict}) DO UPDATE SET {updates}"
    ).format(
        table=sqlident(table.name),
        fields=sqljoin(", ", (sqlident(c.name) for c in table.columns)),
        values=sqljoin(
            ", ", (_pg_wrap_write_column(c, sqlstr(f"%({c.name})s")) for c in table.columns)
        ),
        conflict=sqljoin(", ", (sqlident(c.name) for c in conflict_columns)),
        updates=sqljoin(", ", chain(static_update, dynamic_update)),
    )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("postgres.upsert", table=table, cur=cur, query=query_str, span="current")

    templated_values = rows
    try:
        await _pg_executemany(cur, statement, templated_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_update_static(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    where: SqlNode | None = None,
    static_value: RowIn,
) -> None:
    """Updates the given table with static values. Expects values to be adapted and wrapped."""
    trace.get_current_span().set_attribute("table", table.name)
    statement = sqlstr("UPDATE {table} SET {values}").format(
        table=sqlident(table.name),
        values=sqljoin(
            ", ",
            (
                sqlstr("{} = {}").format(
                    sqlident(k),
                    _pg_wrap_write_column(table._columns_by_name[k], sqlstr(f"%({k})s")),
                )
                for k in static_value
            ),
        ),
    )
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    query = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query)
    logger.trace("postgres.update_constant", table=table, cur=cur, query=query)

    template_values = static_value
    try:
        await _pg_execute(cur, statement, template_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_update_variable(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    table: SqlTable,
    dynamic_columns: Collection[SqlColumn],
    dynamic_values: Collection[RowIn],
    static_values: RowIn | None = None,
) -> None:
    """
    Updates the given table with a list of values (corresponding to rows).
    Expects values to be adapted and wrapped.
    If a column is in dynamic_columns but not in dynamic_values for a row, keep the current value.
    """
    assert table._primary_key, f"no primary key for {table!r}"
    if not any(c.is_primary_key for c in dynamic_columns):
        raise ValueError(
            f"dynamic_columns {dynamic_columns!r} do not contain {table._primary_key!r}"
        )
    table_name = sqlident(table.name)
    static_values = static_values or {}

    # join fixed and dynamic values
    static_values_sql = tuple(
        sqlstr("{} = {}").format(sqlident(k), _pg_wrap_write_column(table._columns_by_name[k], v))
        for k, v in static_values.items()
    )
    # only set dynamic columns if they are in the row (otherwise keep current value)
    dynamic_values_sql = tuple(
        sqlstr(f'{{}} = CASE WHEN %(__{c.name}_set)s THEN {{}} ELSE "{c.name}" END').format(
            sqlident(c.name), _pg_wrap_write_column(c, sqlstr(f"%({c.name})s"))
        )
        for c in dynamic_columns
    )
    values_sql = sqljoin(
        ", ",
        chain(static_values_sql, dynamic_values_sql),
    )
    statement = sqlstr("UPDATE {table} SET {values} WHERE {pk} = %(pk)s").format(
        table=table_name, pk=sqlident(table._primary_key.name), values=values_sql
    )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace(
        "postgres.update_variable",
        table=table,
        cur=cur,
        query=query_str,
        span="current",
        rows=len(dynamic_values),
    )

    templated_values: list[RowIn] = []
    for row in dynamic_values:
        pk = row.get(table._primary_key.name)
        if not pk:
            raise ValueError(f"missing primary key {table._primary_key!r} in row {row!r}")
        templated_value = {**row, "pk": pk}
        for column in dynamic_columns:
            row_has_column = column.name in row
            templated_value[f"__{column.name}_set"] = row_has_column
            if not row_has_column:
                templated_value[column.name] = None
        templated_values.append(templated_value)
    try:
        await _pg_executemany(cur, statement, templated_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_delete(
    *, cur: psycopg.AsyncCursor, ctx: SqlContext, table: SqlTable, where: SqlNode | None = None
) -> None:
    """Deletes from the given table."""
    statement = sqlstr("DELETE FROM {table}").format(
        table=sqlident(table.name),
    )
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("postgres.delete", table=table, cur=cur, query=query_str, span="current")
    try:
        await _pg_execute(cur, statement)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_truncate(cur: psycopg.AsyncCursor, table: SqlTable, ctx: SqlContext) -> None:
    """Truncates the given table."""
    await _pg_execute(cur, sqlstr("TRUNCATE TABLE {}").format(sqlident(table.name)))

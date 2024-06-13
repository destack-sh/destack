import base64
import enum
import struct
from collections import defaultdict
from dataclasses import dataclass
from functools import wraps
from itertools import chain
from typing import (
    Any,
    Collection,
    Iterable,
    Mapping,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
    cast,
)
from uuid import UUID

import cachetools
import psycopg
import pytz
import structlog
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from bitarray import bitarray
from opentelemetry import trace
from psycopg import OperationalError, sql
from psycopg.types.json import Jsonb

from bench.language import Block, ConditionalOp, Field, Property
from bench.language.channel import ChannelIncapableError
from bench.language.const import (
    CASCADING_EDIT_TYPES,
    EMPTY_DICT,
    NODE_TYPES,
    BenchError,
    BlockType,
    EditType,
    EnumType,
    LiteralOp,
    NodeType,
    ReferenceKind,
    SortOp,
)
from bench.language.expression import C, Expression, ExpressionOps, NodeReference
from bench.language.graph import NodeDataGraph
from bench.language.node import NODE_CLASS_BY_TYPE, UNSET, BenchNode, Node
from bench.language.query import FILTER_VISIBLE, SELECT_ALL_PROPERTIES, ReadOptions
from bench.language.setup import (
    DESCENDANT_NODE_TYPES_IN_STORE,
    HAS_CHILD_NODE_TYPES,
    NODE_CLASSES,
    PARENT_NODE_TYPES,
)
from bench.language.transaction import pack_node_delta, unpack_node_delta
from bench.language.value import pack_builtin_object_data, unpack_builtin_object_data
from bench.proto import wire, wiring
from bench.proto.wire import AnyNodeData, EditData, GraphScope, IdEnum, NodeReferenceData
from bench.proto.wiring import PROTO_CLASS_BY_TYPE
from bench.sql import schema
from bench.sql.client import get_pg_crypto_key
from bench.sql.core import (
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    GLOBAL_EXTENSIONS,
    LOCAL_EXTENSIONS,
    RECORD_BASE_TABLE,
    CascadeAction,
    Column,
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    PrimitiveType,
    Schema,
    SqlPrimitive,
    Table,
)
from bench.utils.casing import Casing, to_casing
from bench.utils.env import IS_DEV
from bench.utils.func import bittuple, describe_type, group_by, to_uuid
from bench.utils.tenacity import RetryOptions, retry
from bench.utils.uuidt import UUIDT

# NOTE :Performance: check out asyncpg instead of psycopg (up to 5x faster?)
#  see https://github.com/MagicStack/asyncpg

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def _trace_pg_span(func):
    """Instruments a pg function with common parameters as span attributes"""
    func_name = func.__name__
    if func_name.startswith("_"):
        func_name = func_name[1:]
    assert func_name.startswith("pg_"), f"unexpected pg function: {func.__name__}"

    @wraps(func)
    @tracer.start_as_current_span(f"pg.{func_name[3:]}")
    async def wrapped(**kwargs):
        # extract out all the interesting attributes for the span
        cur = kwargs.get("cur")
        assert isinstance(cur, psycopg.AsyncCursor), f"bad cur for {func.__name__}: {cur!r}"
        span = trace.get_current_span()
        span.set_attribute("connection_uri", get_sanitized_connection_uri(cur.connection))
        span.set_attribute("connection_id", id(cur.connection))
        table = kwargs.get("table")
        if isinstance(table, Table):
            span.set_attribute("table", table.name)
        node_type = kwargs.get("node_type")
        if node_type is not None:
            span.set_attribute("node_type", NodeType(node_type).bench_name)

        # forward call
        try:
            return await func(**kwargs)  # type: ignore
        except Exception as e:
            # debug, not error, because this may not be an actual error at the application level
            logger.debug(f"{func_name}.error", **kwargs, exc_info=e, span="current")
            raise

    return wrapped


def get_sanitized_connection_uri(conn: psycopg.AsyncConnection) -> str:
    """Gets the connection URI like postgresql://user:****@host:port/dbname."""
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
            conn_str = get_sanitized_connection_uri(conn)
            super().__init__(f"{conn_str}: {message}")
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


def sqljoin(sep: str, args: Iterable[SqlNode]) -> sql.Composed:
    return sqlstr(sep).join(sql_node_to_sql(a) for a in args)


PG_CAST_PRIMITIVE_TYPE: dict[PrimitiveType, str] = {
    PrimitiveType.BOOLEAN: "boolean",
    PrimitiveType.INT32: "int",
    PrimitiveType.INT64: "bigint",
    PrimitiveType.FLOAT32: "float",
    PrimitiveType.FLOAT64: "double",
    PrimitiveType.DECIMAL: "decimal",
    PrimitiveType.STRING: "text",
    PrimitiveType.VECTOR: "float[]",
    PrimitiveType.BYTES: "bytea",
    PrimitiveType.DATETIME: "timestamptz",
    PrimitiveType.INTERVAL: "interval",
    PrimitiveType.JSON: "jsonb",
    PrimitiveType.UUID: "uuid",
}


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
    ILIKE = "ILIKE"
    REGEXP = "~"
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
    ConditionalOp.MATCHES_REGEX: PostgresConditionalOp.REGEXP,
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


POSTGRES_SORT_OP_BY_BENCH: dict[SortOp, PostgresSortOp] = {
    SortOp.ASCENDING: PostgresSortOp.ASC,
    SortOp.DESCENDING: PostgresSortOp.DESC,
}


def get_block_table_name(block: Block) -> str:
    """
    Gets the name for a table with the Records of a dynamically created DatabaseBlock.
    NOTE: we rely on a constant :BlockTablePrefix
    """
    if block.type == BlockType.DATABASE:
        return f"bench_record_{block.tk.replace('-', '')}"
    else:
        raise ValueError(f"unexpected block type {block.type!r}")


def get_bench_table_name(node_type: NodeType) -> str:
    """Gets the name for a regular Bench node table."""
    return f"bench_{node_type.name.lower().replace('_', '')}"


def map_node_class_to_pg_table(node: type[Node]) -> Table:
    # NOTE :Robustness: add Bench check constraints in Postgres?
    table_name = get_bench_table_name(node.metatype)
    columns: list[Column] = []
    constraints: list[Constraint] = [*(node.__extra_constraints__ or ())]
    indexes: list[Index] = [*(node.__extra_indexes__ or ())]
    properties = list(node.__properties__.values())
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if not prop.is_stored:
            continue
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        column = Column(
            name=prop.name,
            type=prop.primitive_type,
            is_array=prop.is_list,
            is_nullable=not prop.is_required,
            is_encrypted=prop.is_encrypted,
            is_primary_key=prop.name == "id",
            is_unique=prop.is_unique,
            _source=prop.id,
        )
        # default
        if prop.default_sql is not UNSET:
            column.default = prop.default_sql
        elif prop.default is not UNSET and prop.default is not None:
            if isinstance(prop.default, IdEnum):
                column.default = f"'{prop.default.name}'::character varying"
            elif isinstance(prop.default, bool):
                column.default = "false" if prop.default is False else "true"
            elif isinstance(prop.default, int):
                column.default = str(prop.default)
            elif isinstance(prop.default, str):
                column.default = f"'{prop.default}'::character varying"
            else:
                raise TypeError(f"unexpected default in {prop!r}: {prop.default!r}")
        # FKs
        is_local = node.__is_local__ or any(
            NODE_CLASS_BY_TYPE[n].__is_local__ for n in prop.reference_nodes or ()
        )
        if (
            (prop.reference_kind == ReferenceKind.NODE_PARENT or prop.reference_force_fk)
            and not prop.is_list  # foreign keys must be scalar
            and prop.reference_nodes
            and (not is_local or node.metatype == prop.reference_nodes[0])
        ):
            assert len(prop.reference_nodes) == 1, f"stored prop {prop!r} has multiple references"
            column.is_foreign_key_to = get_bench_table_name(prop.reference_nodes[0])
            assert isinstance(prop.reference_on_delete, CascadeAction)
            column.on_delete = prop.reference_on_delete

        if prop.is_indexed_in_pg or prop.is_unique:
            index = Index(
                f"bench_idx_{prop.name}",
                type=IndexType.BTREE,
                columns=(column.name,),
                is_unique=prop.is_unique,
                _source=prop.id,
            )
            indexes.append(index)
            if prop.is_unique:
                constraint = Constraint(
                    index.inner_name,  # must be the same as the index name (postgres will rename otherwise)
                    type=ConstraintType.UNIQUE,
                    columns=(column.name,),
                    index=index.inner_name,
                    _source=prop.id,
                )
                constraints.append(constraint)
        columns.append(column)

    table = Table(
        _source=node.metatype.id,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def get_field_column_name(field: Field) -> str:
    storage_key = field.storage_key.replace(".", "_").replace("-", "_").lower()
    return f"value_{storage_key}"


def map_block_to_pg_table(block: Block) -> Table:
    """Gets the full table with all specific fields of a block and general record stuff."""
    columns: list[Column] = []
    indexes: list[Index] = []
    constraints: list[Constraint] = []

    for field in block.fields:
        typ = field._to_resolved()
        assert typ.primitive_type is not None, f"no primitive type for {field!r}"
        column = Column(
            _source=str(field.ck),
            name=get_field_column_name(field),
            type=typ.primitive_type,
            is_array=typ.is_list,
            is_nullable=True,
            is_encrypted=typ.is_secret,
        )
        columns.append(column)

    return Table(
        _source=str(block.ck),
        name=get_block_table_name(block),
        columns=(*(c.clone() for c in RECORD_BASE_TABLE.columns), *columns),
        indexes=(*(i.clone() for i in RECORD_BASE_TABLE.indexes), *indexes),
        constraints=(*(c.clone() for c in RECORD_BASE_TABLE.constraints), *constraints),
    )


def _compile_expression_ref(
    node: Union[type[Node], Block],
    expr: Expression,
) -> SqlNode:
    if expr.property is not None:
        return sqlident(expr.property.name)
    elif expr.field is not None:
        assert (
            expr.field._introspected_from is None
        ), f"cannot use introspected: {expr!r}->{expr.field!r}"
        if isinstance(node, Block) and not node.is_materialized:
            return SqlJsonPath(sqlident("value"), [expr.field.storage_key])
        else:
            return sqlident(get_field_column_name(expr.field))
    else:
        raise TypeError(f"unexpected expression ref: {expr!r}")


def _pg_compile_conditional_maybe(
    node: Union[type[Node], Block],
    cond: Optional[Expression],
) -> SqlNode:
    if cond is None:
        return sqlstr("TRUE")
    return _pg_compile_conditional(node, cond)


def _pg_compile_conditional(
    node: Union[type[Node], Block],
    cond: Expression,
) -> SqlNode:
    if cond.op == LiteralOp.TRUE:
        return sqlstr("TRUE")
    elif cond.op == LiteralOp.FALSE:
        return sqlstr("FALSE")
    elif cond.op == LiteralOp.NONE:
        return sqlstr("NULL")
    elif cond.op in ExpressionOps.COND_LOGICAL and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [_pg_compile_conditional(node, c) for c in cond.clauses or ()]
        return SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
    elif (
        cond.op in ExpressionOps.COND_COMPARISON or cond.op in ExpressionOps.COND_STRING
    ) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_expression_ref(node, cond)
        # add explicit cast to LHS if possible
        if isinstance(cond.field, Field):
            assert cond.field.primitive_type is not None, f"no primitive type for {cond.field!r}"
            pg_type = PG_CAST_PRIMITIVE_TYPE[cond.field.primitive_type]
            left = sqlstr("({})::{}").format(sql_node_to_sql(left), sqlstr(pg_type))

        if cond.op in (ConditionalOp.IN, ConditionalOp.NOT_IN):
            # map IN to ANY() construct (IN/NOT IN doesn't work in psycopg)
            # psycopg also can't handle tuples, so list it is
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sqlstr("ANY({})").format(sql.Literal(value))
            op = (
                PostgresConditionalOp.EQ
                if cond.op == ConditionalOp.IN
                else PostgresConditionalOp.NEQ
            )
            return SqlComparison(left=left, op=op, right=right)
        elif cond.op == ConditionalOp.STARTS_WITH:
            right = sqlstr("{} || '%'").format(sql.Literal(cond.value))
        else:
            assert cond.value is not None, f"cannot compare {cond!r} with None"
            right = sql.Literal(cond.value)

        return SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)
    elif cond.op in ExpressionOps.COND_EXISTENCE:
        return SqlUnary(
            left=_compile_expression_ref(node, cond),
            op=PG_CONDITIONAL_OP_BY_BENCH[cond.op],
        )
    raise ChannelIncapableError("postgres", expression=cond, reason="unsupported conditional")


def _pg_compile_sort(node: Union[type[Node], Block], sort: Expression) -> sql.Composed:
    field_ref = _compile_expression_ref(node, sort)
    sort_op = POSTGRES_SORT_OP_BY_BENCH[cast(SortOp, sort.op)]
    return sqlstr("{} {}").format(sql_node_to_sql(field_ref), sqlstr(sort_op))


def _pg_compile_sorts(
    node: Union[type[Node], Block], sorts: Collection[Expression]
) -> sql.Composed:
    return sqljoin(", ", (_pg_compile_sort(node, sort) for sort in sorts))


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
    foreign_table: Table | SqlNode
    condition: SqlNode

    def sql(self) -> sql.Composable:
        foreign_table = self.foreign_table
        if isinstance(foreign_table, Table):
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


@retry(RETRY_PG)
async def _pg_execute(
    cur: psycopg.AsyncCursor,
    statement: sql.Composed,
    params: Mapping | None = None,
):
    await cur.execute(statement, params)


@retry(RETRY_PG)
async def _pg_executemany(
    cur: psycopg.AsyncCursor,
    statement: sql.Composed,
    params: Iterable[Mapping],
    returning: bool = False,
):
    await cur.executemany(statement, params, returning=returning)


def _pg_wrap_write_column(column: Column, value: SqlNode) -> SqlNode:
    if not column.is_encrypted:
        return value

    # convert and decrypt
    assert not column.is_array, f"cannot encrypt array column: {column!r}"
    if not isinstance(value, sql.Composable) and column._unencrypted_type == PrimitiveType.JSON:
        value = Jsonb(value)  # adapt json
    # first to bytea
    if column._unencrypted_type == PrimitiveType.BYTES:
        value = sqlstr("{}::bytea").format(value)
    elif column._unencrypted_type in (PrimitiveType.STRING, PrimitiveType.JSON):
        value = sqlstr("convert_to({}::text, 'UTF8')").format(value)
    else:
        pg_cast = PG_CAST_PRIMITIVE_TYPE[cast(PrimitiveType, column._unencrypted_type)]
        value = sqlstr("{}::{}::text::bytea").format(value, sqlstr(pg_cast))
    # then encrypt
    value = sqlstr("pgp_sym_encrypt_bytea({}, %(PG_CRYPTO_KEY)s::text)").format(
        sql_node_to_sql(value)
    )
    return value


def _pg_wrap_read_column(column: Column, value: SqlNode) -> SqlNode:
    if not column.is_encrypted:
        return value

    # decrypt and convert
    assert not column.is_array, f"cannot encrypt array column: {column!r}"
    original = value
    # first decrypt with
    value = sqlstr("pgp_sym_decrypt_bytea({}, %(PG_CRYPTO_KEY)s::text)").format(
        sql_node_to_sql(value), sql.Literal(get_pg_crypto_key(column))
    )
    # then convert from bytea to the correct type
    if column._unencrypted_type == PrimitiveType.BYTES:
        value = sqlstr("{}::bytea").format(value)
    else:
        pg_cast = PG_CAST_PRIMITIVE_TYPE[cast(PrimitiveType, column._unencrypted_type)]
        value = sqlstr("convert_from({}::bytea, 'UTF8')::text::{}").format(value, sqlstr(pg_cast))
    # and bail if original value is null
    value = sqlstr("(CASE WHEN {} IS NULL THEN NULL ELSE {} END)").format(
        sql_node_to_sql(original), value
    )
    # and label column
    value = sqlstr("{} as {}").format(value, sqlident(column.name))
    return value


def _pg_adapt_row(table: Table, row: Mapping[str, Any]) -> Mapping[str, Any]:
    """Adapts and wraps Any values"""
    wrapped = {}
    for column in table.columns:
        if column.name not in row:
            continue
        value = row.get(column.name)
        if value is None:
            pass
        elif column.underlying_type == PrimitiveType.JSON:
            value = [Jsonb(v) for v in value] if column.is_array else Jsonb(value)
        wrapped[column.name] = value
    return wrapped


def _pg_adapt_rows(
    table: Table, rows: Iterable[Mapping[str, Any]]
) -> tuple[Mapping[str, Any], ...]:
    return tuple(_pg_adapt_row(table, row) for row in rows)


async def _pg_fetchall_from_many(cur: psycopg.AsyncCursor, expected: int) -> list[RowOut]:
    # see https://www.psycopg.org/psycopg3/docs/api/cursors.html#psycopg.Cursor.executemany
    results: list[RowOut] = []
    while True:
        row = await cur.fetchone()
        assert row, f"expected {expected} results, got {len(results)}"
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
    logger.trace("pg.select_raw", query=query_str, span="current")
    await _pg_execute(cur, cast(sql.Composed, query))
    return await cur.fetchall()


@_trace_pg_span
async def pg_select(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    columns: Collection[Column] | None = None,
    joins: Collection[SqlJoin] | None = None,
    where: SqlNode | None = None,
    order_by: SqlNode | None = None,
    first: int | None = None,
    skip: int | None = None,
    params: Mapping | None = None,
) -> list[RowOut]:
    """Selects from the given table."""
    columns = columns or table.columns
    statement = sqlstr("SELECT {fields} FROM {table}").format(
        fields=sqljoin(", ", (_pg_wrap_read_column(c, sqlident(c.name)) for c in columns)),
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
    logger.trace("pg.select", table=table, cur=cur, query=query_str, span="current")
    if any(c.is_encrypted for c in columns):
        params = {**(params or EMPTY_DICT), "PG_CRYPTO_KEY": get_pg_crypto_key(table)}
    try:
        await _pg_execute(cur, statement, params)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    return await cur.fetchall()


@_trace_pg_span
async def pg_count(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
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
    logger.trace("pg.count", table=table, cur=cur, query=query)
    try:
        await _pg_execute(cur, statement)
        result = await cur.fetchone()
        if not result:
            raise SqlError(f"no result for {table!r}", cur)
        return result["count"]
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_exists(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
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
    logger.trace("pg.exists_rows", table=table, cur=cur, query=query_str, span="current")
    try:
        await _pg_execute(cur, statement)
        result = await cur.fetchone()
        if not result:
            raise SqlError(f"no result for {table!r}", cur)
        return result["exists"]
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e


@_trace_pg_span
async def pg_insert(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: Collection[RowIn],
    returning: Collection[Column] | None = None,
) -> tuple[RowOut, ...] | list[RowOut] | None:
    """Inserts into the given table. Expects rows to be adapted and wrapped."""
    statement = sqlstr("INSERT INTO {table} ({fields}) VALUES ({values})").format(
        table=sqlident(table.name),
        fields=sqljoin(", ", (sqlident(c.name) for c in table.columns)),
        values=sqljoin(
            ", ", (_pg_wrap_write_column(c, sqlstr(f"%({c.name})s")) for c in table.columns)
        ),
    )
    if returning:
        statement += sqlstr(" RETURNING {}").format(
            sqljoin(", ", (_pg_wrap_read_column(c, sqlident(c.name)) for c in returning))
        )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("pg.insert", table=table, cur=cur, query=query_str, span="current")

    if any(c.is_encrypted for c in table.columns):
        templated_values = tuple({**row, "PG_CRYPTO_KEY": get_pg_crypto_key(table)} for row in rows)
    else:
        templated_values = rows
    try:
        await _pg_executemany(cur, statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(rows))


@_trace_pg_span
async def pg_upsert(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: Collection[RowIn],
    conflict_columns: list[Column] | tuple[Column, ...] | None = None,
    static_columns: list[Column] | tuple[Column, ...] | None = None,
    static_values: RowIn | None = None,
    returning: Collection[Column] | None = None,
) -> tuple[RowOut, ...] | list[RowOut] | None:
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
    if returning:
        statement += sqlstr(" RETURNING {}").format(
            sqljoin(", ", (_pg_wrap_read_column(c, sqlident(c.name)) for c in returning))
        )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("pg.upsert", table=table, cur=cur, query=query_str, span="current")

    if any(c.is_encrypted for c in table.columns):
        templated_values = tuple({**row, "PG_CRYPTO_KEY": get_pg_crypto_key(table)} for row in rows)
    else:
        templated_values = rows
    try:
        await _pg_executemany(cur, statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(rows))


@_trace_pg_span
async def pg_update_static(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlNode | None = None,
    static_value: RowIn,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
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
    if returning:
        statement += sqlstr(" RETURNING {}").format(
            sqljoin(", ", (_pg_wrap_read_column(c, sqlident(c.name)) for c in returning))
        )
    query = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query)
    logger.trace("pg.update_constant", table=table, cur=cur, query=query)

    if any(c.is_encrypted for c in table.columns):
        template_values = {**static_value, "PG_CRYPTO_KEY": get_pg_crypto_key(table)}
    else:
        template_values = static_value
    try:
        await _pg_execute(cur, statement, template_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    if returning:
        return await cur.fetchall()


@_trace_pg_span
async def pg_update_variable(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    dynamic_columns: Collection[Column],
    dynamic_values: Collection[RowIn],
    static_values: RowIn | None = None,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
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
        sqlstr(f"{{}} = CASE WHEN %(__{c.name}_set)s THEN {{}} ELSE {c.name} END").format(
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
    if returning:
        statement += sqlstr(" RETURNING {}").format(
            sqljoin(
                ", ",
                (sqlstr("{}").format(_pg_wrap_read_column(c, sqlident(c.name))) for c in returning),
            )
        )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace(
        "pg.update_variable",
        table=table,
        cur=cur,
        query=query_str,
        span="current",
        rows=len(dynamic_values),
    )

    is_any_encrypted = any(c.is_encrypted for c in table.columns)
    pg_crypto_key = get_pg_crypto_key(table)
    templated_values: list[RowIn] = []
    for row in dynamic_values:
        pk = row.get(table._primary_key.name)
        if not pk:
            raise ValueError(f"missing primary key {table._primary_key!r} in row {row!r}")
        templated_value = {**row, "pk": pk}
        if is_any_encrypted:
            templated_value["PG_CRYPTO_KEY"] = pg_crypto_key
        for column in dynamic_columns:
            row_has_column = column.name in row
            templated_value[f"__{column.name}_set"] = row_has_column
            if not row_has_column:
                templated_value[column.name] = None
        templated_values.append(templated_value)
    try:
        await _pg_executemany(cur, statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(dynamic_values))


@_trace_pg_span
async def pg_delete(
    *,
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlNode | None = None,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Deletes from the given table."""
    statement = sqlstr("DELETE FROM {table}").format(
        table=sqlident(table.name),
    )
    if where:
        statement += sqlstr(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        statement += sqlstr(" RETURNING {}").format(
            sqljoin(", ", (_pg_wrap_read_column(c, sqlident(c.name)) for c in returning))
        )
    query_str = sql_to_str(cur, statement)
    trace.get_current_span().set_attribute("sql_query", query_str)
    logger.trace("pg.delete", table=table, cur=cur, query=query_str, span="current")
    try:
        await _pg_execute(cur, statement)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, cur, e) from e
    if returning:
        return await cur.fetchall()


@_trace_pg_span
async def pg_truncate(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Truncates the given table."""
    await _pg_execute(cur, sqlstr("TRUNCATE TABLE {}").format(sqlident(table.name)))


#
# Nodes API
# Higher level methods use Session and return Nodes, but handle respective PG cursors.
# Lower level methods use data constructs and expect appropriate PG cursors.
#

NodeT = TypeVar("NodeT", bound=Node)


def _pack_struct_data_prop(prop: Property, value: Any, ignore_array: bool) -> SqlPrimitive:
    """Packs the value of a struct property for storage in Postgres."""
    if value is None:
        return None
    elif prop.is_list and not ignore_array:
        return [_pack_struct_data_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        value = pack_builtin_object_data(value)
        return Jsonb(value)  # type: ignore
    elif prop.is_enum:
        return value.value
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(value.to_dict())  # type: ignore
    else:
        return value


def _unpack_struct_data_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    """Unpacks the value of a struct property from Postgres."""
    if value is None:
        return None
    elif prop.is_list and not ignore_array:
        return [_unpack_struct_data_prop(prop, v, ignore_array=True) for v in value]
    elif prop.reference_struct:
        return unpack_builtin_object_data(value)
    elif prop.is_enum:
        return wiring.pack_enum(prop.py_type_stripped, prop.py_type_stripped(value))
    elif prop.primitive_type == PrimitiveType.DATETIME:
        if value.tzinfo is None:
            return value.replace(tzinfo=pytz.utc)
        else:
            return value.astimezone(pytz.utc)
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return ProtoStruct.from_dict(value)
    else:
        return value


def _pg_pack_node_reference_into_row(
    prop: Property | Any,
    row: dict[str, Any],
    value: NodeReferenceData | Collection[NodeReferenceData] | None,
) -> None:
    """
    'Unravels' a wired pointer into (one or more) stored columns as needed.
    pack/unpacking pointers into rows is a bit gnarly, see :StoredPointers
    """
    assert prop.reference_stored_ids is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_ids_by_type is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_meta is not None, f"no stored extras for {prop!r}"
    if prop.is_list:  # list reference
        # map to references list
        if value is None:
            references = ()
        elif isinstance(value, list):
            references = value
        else:
            raise ValueError(f"expected list of references, got {value!r}")
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            row[stored_prop.name] = []
        for ref in references:
            stored_prop = prop.reference_stored_ids_by_type[cast(NodeType, ref.type)]
            row[stored_prop.name].append(ref.id)
        # additional pointer metadata
        for meta_key, meta_prop in prop.reference_stored_meta.items():
            row[meta_prop.name] = [getattr(v, meta_key) for v in references]
    else:  # single reference
        # map to single reference
        if value is None:
            reference = None
        elif type(value) is NodeReferenceData:
            reference = value
        else:
            raise ValueError(f"expected single reference, got {value!r}")
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            assert stored_prop.reference_nodes is not None, f"no reference nodes: {stored_prop!r}"
            if reference is not None and reference.type in stored_prop.reference_nodes:
                row[stored_prop.name] = reference.id
            else:
                row[stored_prop.name] = None
        # additional pointer metadata
        for meta_key, meta_prop in prop.reference_stored_meta.items():
            row[meta_prop.name] = getattr(value, meta_key) if value is not None else None


def _pg_unpack_node_reference_from_row(prop: Property, row: RowOut, node: AnyNodeData) -> None:
    """
    'Ravels' a wired pointer from (one or more) stored columns.
    See above and :StoredPointers
    """

    # get bench id
    bench_id = row.get("id") if node.metatype == NodeType.BENCH else row.get("bench_id")
    if bench_id is not None:
        bench_id = str(bench_id)

    assert prop.reference_stored_ids is not None, f"no stored ids for {prop!r}"
    assert prop.reference_stored_meta is not None, f"no stored extras for {prop!r}"
    assert prop.reference_wired_ptr is not None, f"no wired ptr for {prop!r}"
    if prop.is_list:  # list reference
        # can only be a a set of id props + a single ck prop (:HomogeneousListCk)
        ptrs = []
        # pointer id/cks
        for stored_prop in prop.reference_stored_ids:
            ids = cast(list[UUID] | None, row.get(stored_prop.name))
            for id in ids or ():
                ptr = NodeReferenceData(
                    metatype=wire.ObjectType.NODE_REFERENCE,
                    id=str(id),
                    type=cast(list[wire.NodeType], stored_prop.reference_nodes)[0],
                )
                ptrs.append(ptr)
        setattr(node, prop.reference_wired_ptr.name, ptrs)
        # additional pointer metadata
        for i, ptr in enumerate(ptrs):
            for meta_key, meta_prop in prop.reference_stored_meta.items():
                extra_value = cast(list, row.get(meta_prop.name))[i]
                if extra_value is None:
                    continue
                elif meta_prop.primitive_type == PrimitiveType.UUID:
                    extra_value = str(extra_value)
                elif meta_prop.enum_type == EnumType.NODE_TYPE:
                    extra_value = NodeType(extra_value)
                else:
                    raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
                setattr(ptr, meta_key, extra_value)
            if prop.reference_is_bench_implicit:
                ptr.bench_id = bench_id
                if ptr.base_ck:
                    ptr.base_bench_id = ptr.bench_id
    else:  # single reference
        # pointer id/ck
        for stored_prop in prop.reference_stored_ids:
            value = cast(UUID | None, row.get(stored_prop.name))
            if value is not None:
                ptr = NodeReferenceData(
                    metatype=wire.ObjectType.NODE_REFERENCE,
                    id=str(value),
                    # if this is a heterogeneous ck pointer, type will be overwritten from extras
                    type=cast(list[wire.NodeType], stored_prop.reference_nodes)[0],
                )
                setattr(node, prop.reference_wired_ptr.name, ptr)
                break
        else:
            ptr = None
        # additional pointer metadata
        if ptr is not None:
            for meta_key, meta_prop in prop.reference_stored_meta.items():
                extra_value = row.get(meta_prop.name)
                if extra_value is None:
                    continue
                elif meta_prop.primitive_type == PrimitiveType.UUID:
                    extra_value = str(extra_value)
                elif meta_prop.enum_type == EnumType.NODE_TYPE:
                    extra_value = NodeType(extra_value)
                else:
                    raise RuntimeError(f"unexpected meta prop type: {meta_prop!r}")
                setattr(ptr, meta_key, extra_value)
            if prop.reference_is_bench_implicit:
                ptr.bench_id = bench_id
                if ptr.base_ck:
                    ptr.base_bench_id = ptr.bench_id


def pg_pack_node_data_row(node: AnyNodeData) -> dict[str, SqlPrimitive]:
    """Packs a node's data into a row for the respective table."""
    node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, node.metatype)]
    try:
        row: dict[str, SqlPrimitive] = {}
        for name, prop in node_cls.__wired_properties__.items():
            if prop.reference_source is None:
                # regular non-ref property
                row[name] = _pack_struct_data_prop(prop, getattr(node, name), ignore_array=False)
            else:
                # unravel stored node reference :StoredPointers
                value: NodeReferenceData | list[NodeReferenceData] | None = getattr(
                    node, cast(Property, prop.reference_source.reference_wired_ptr).name
                )
                _pg_pack_node_reference_into_row(prop.reference_source, row, value)
        return row
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack row {node_cls.metatype.name}: {struct!r}") from e


def pg_unpack_node_data_row(node_cls: type[Node], row: Mapping[str, Any]) -> AnyNodeData:
    """Unpacks a node's data from a row from the respective table."""
    try:
        proto_cls = PROTO_CLASS_BY_TYPE[node_cls.metatype]
        data = cast(AnyNodeData, proto_cls(metatype=wiring.pack_enum(NodeType, node_cls.metatype)))  # type: ignore
        for name, prop in node_cls.__wired_properties__.items():
            if prop.reference_source is None:
                # regular non-ref property
                value = row.get(name)
                if value is not None:
                    value = _unpack_struct_data_prop(prop, value, ignore_array=False)
                    setattr(data, name, value)
            else:
                # ravel stored node reference :StoredPointers
                _pg_unpack_node_reference_from_row(prop.reference_source, row, data)
        return data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        row_str = repr(row) if IS_DEV else describe_type(row)
        raise ValueError(f"could not unpack row {node_cls.metatype.name}: {row_str}") from e


class PgSelectNodesDataResult(NamedTuple):
    nodes: tuple[AnyNodeData, ...]
    cursors: tuple[str, ...]
    start_cursor: str | None


@_trace_pg_span
async def pg_get_nodes(
    cur: psycopg.AsyncCursor,
    node_type: NodeType,
    *,
    properties: Collection[Property],
    filter: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> PgSelectNodesDataResult:
    """Selects regular nodes from the given PG database."""
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    assert node_cls.__table__, f"no table for {node_cls!r}"
    if after:
        skip = (skip or 0) + int(decode_pg_cursor(after)) + 1  # 'after' is exclusive
    columns = [prop.column for prop in properties]
    assert any(c.is_primary_key for c in columns), f"no primary key selected in {columns!r}"
    where = _pg_compile_conditional(node_cls, filter) if filter is not None else None
    order_by = _pg_compile_sorts(node_cls, sort) if sort else None
    rows = await pg_select(
        cur=cur,
        table=node_cls.__table__,
        columns=columns,
        where=where,
        order_by=order_by,
        first=first,
        skip=skip,
    )
    nodes_data = tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
    cursors = tuple(encode_pg_cursor(i) for i in range(skip or 0, (skip or 0) + len(nodes_data)))
    assert len(nodes_data) == len(cursors), f"unexpected cursors: {cursors} for {nodes_data}"
    return PgSelectNodesDataResult(nodes_data, cursors, after)


@_trace_pg_span
async def pg_walk_graph_down(
    *,
    cur: psycopg.AsyncCursor,
    roots: list[NodeReferenceData]
    | list[AnyNodeData]
    | tuple[NodeReferenceData, ...]
    | tuple[AnyNodeData, ...],
    descendant_types: Collection[NodeType],
    extra_filter: Expression | None,
) -> tuple[list[NodeReferenceData], dict[str, list[NodeReferenceData]]]:
    """
    Gets node pointers to all descendants down from the roots matching the filter.
    TODO :Performance!: walk graph down in SQL only (no roundtrip recursion)
     (the result of this is usually cached after initial load, but not for edit cascades)
    """
    if not roots:
        return [], {}
    if not isinstance(roots[0], NodeReferenceData):
        roots_ptrs = [NodeReference.from_node_data(cast(AnyNodeData, node)) for node in roots]
    else:
        roots_ptrs = cast(list[NodeReferenceData], roots)

    # filter to descendant types that have a table with parents
    descendant_types = [
        t
        for t in descendant_types
        if NODE_CLASS_BY_TYPE[t].__parent_property__.reference_stored_ids
        and NODE_CLASS_BY_TYPE[t].__table__ is not None
    ]

    # descend
    all_descendants: list[NodeReferenceData] = []
    root_id_by_node_id = {cast(str, node.id): cast(str, node.id) for node in roots_ptrs}
    all_descendants_by_root_id: dict[str, list[NodeReferenceData]] = defaultdict(list)
    current_parents = roots_ptrs
    while current_parents:
        next_parents: list[NodeReferenceData] = []
        # traverse all direct children of plausible types
        for child_type in descendant_types:
            child_cls = NODE_CLASS_BY_TYPE[child_type]
            assert child_cls.__parent_property__.reference_stored_ids

            # collect possible parents
            parent_ids: list[str] = []
            for parent in current_parents:
                if NodeType(parent.type) in PARENT_NODE_TYPES[child_type]:
                    parent_ids.append(cast(str, parent.id))
            if not parent_ids:
                continue  # nothing to do
            parent_filter = C(
                op=ConditionalOp.IN,
                property=child_cls.__parent_property__.reference_stored_ids[0],
                value=parent_ids,
                value_packed=UNSET,  # don't pack this value
            )
            if extra_filter is not None:
                parent_filter = parent_filter & extra_filter

            # collect children
            child_table = child_cls.__table__
            assert child_table, f"no table for {child_cls!r}"
            assert child_table._primary_key, f"no primary key for {child_cls!r}"
            parent_where = _pg_compile_conditional(child_cls, parent_filter)
            children_rows = await pg_select(
                cur=cur,
                table=child_table,
                columns=(
                    child_table._columns_by_name["id"],
                    child_table._columns_by_name["parent_id"],
                ),
                where=parent_where,
            )
            for child_row in children_rows:
                child_ptr = NodeReferenceData(
                    metatype=wire.ObjectType.NODE_REFERENCE,
                    id=str(child_row["id"]),
                    type=wire.NodeType(child_type),
                )
                next_parents.append(child_ptr)
                all_descendants.append(child_ptr)
                # remember root
                root_id = root_id_by_node_id.get(str(child_row["parent_id"]))
                assert root_id is not None, f"no root id for {child_row!r}"
                root_id_by_node_id[cast(str, child_ptr.id)] = root_id
                all_descendants_by_root_id[root_id].append(child_ptr)

        current_parents = next_parents

    return all_descendants, all_descendants_by_root_id


@_trace_pg_span
async def pg_get_node_graph(
    *,
    cur: psycopg.AsyncCursor,
    root_type: NodeType,
    roots: tuple[UUID, ...] | tuple[AnyNodeData, ...],
    options: ReadOptions,
    visited_graph: NodeDataGraph,
) -> None:
    """
    Reads regular nodes from the given PG database.
    Returns a graph of nodes that *may* contain the requested nodes.
    'Root' here is the base level, we get ancestor/descendants relative to the 'roots'.
    NOTE :Performance: we could read all package contents with package_id=x if we know it's a package query.
    """
    assert roots, "no roots to select"

    # get roots
    if isinstance(roots[0], UUID):
        # select roots
        root_filter = options.filter(root_type, C(ConditionalOp.IN, property=Node.id, value=roots))
        roots_result = await pg_get_nodes(
            cur=cur,
            node_type=root_type,
            filter=root_filter,
            properties=options.select(root_type),
        )
        if not roots_result.nodes:
            return
        root_nodes = roots_result.nodes
    else:  # already got nodes
        root_nodes = cast(tuple[AnyNodeData, ...], roots)
    for node in root_nodes:
        visited_graph.add(node)

    # select ancestors (recursively)
    # (since this is usually a straight, short walk we just select up step by step)
    if options.ancestor_types:
        current_parents = root_nodes
        to_select_by_type: dict[NodeType, list[str]] = defaultdict(list)
        while current_parents:
            to_select_by_type.clear()

            # traverse unseen parents to select next
            for node in current_parents:
                if (
                    node.parent_ptr is not None
                    and node.parent_ptr.id is not None
                    and node.parent_ptr.type in options.ancestor_types
                    and node.parent_ptr.id not in visited_graph
                ):
                    parent_type = NodeType(node.parent_ptr.type)
                    to_select_by_type[parent_type].append(node.parent_ptr.id)

            # select next parents
            next_parents = []
            for node_type, node_ids in to_select_by_type.items():
                layer = await pg_get_nodes(
                    cur=cur,
                    node_type=node_type,
                    filter=options.filter(
                        node_type,
                        C(ConditionalOp.IN, property=Node.id, value=node_ids),
                    ),
                    properties=options.select(node_type),
                )
                next_parents.extend(layer.nodes)
                for node in layer.nodes:
                    visited_graph.add(node)
            current_parents = next_parents

    # select descendants (recursively)
    # (since this may be a long wide search down, we first collect the pointers, then select by type)
    if options.descendant_types:
        descendant_node_ptrs, _ = await pg_walk_graph_down(
            cur=cur,
            roots=root_nodes,
            descendant_types=options.descendant_types,
            extra_filter=FILTER_VISIBLE if not options.include_hidden else None,
        )
        descendant_node_ptrs_by_type = group_by(descendant_node_ptrs, lambda ptr: ptr.type)
        for wire_node_type, node_ptrs in descendant_node_ptrs_by_type.items():
            node_type = NodeType(wire_node_type)
            layer = await pg_get_nodes(
                cur=cur,
                node_type=node_type,
                filter=options.filter(
                    node_type,
                    C(ConditionalOp.IN, property=Node.id, value=[ptr.id for ptr in node_ptrs]),
                ),
                properties=options.select(node_type),
            )
            for node in layer.nodes:
                visited_graph.add(node)


@_trace_pg_span
async def pg_search_node_graph(
    *,
    cur: psycopg.AsyncCursor,
    scope: GraphScope,
    node_type: NodeType,
    options: ReadOptions,
    filter: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> tuple[PgSelectNodesDataResult, NodeDataGraph]:
    """Select root nodes and then read the graph of nodes from the given PG database."""
    node_types = tuple({node_type, *options.ancestor_types, *options.descendant_types})
    if options.ancestor_types or options.descendant_types:
        # split into two passes if we have other nodes to fetch
        roots = await pg_get_nodes(
            cur=cur,
            node_type=node_type,
            filter=options.filter(node_type, filter),
            sort=sort,
            first=first,
            skip=skip,
            after=after,
            properties=options.select(node_type),
        )
        visited_graph = NodeDataGraph(scope, node_types)
        if not roots.nodes:
            return roots, visited_graph
        await pg_get_node_graph(
            cur=cur,
            root_type=node_type,
            roots=roots.nodes,
            options=options,
            visited_graph=visited_graph,
        )
        return roots, visited_graph
    else:
        # otherwise just select in one go
        roots = await pg_get_nodes(
            cur=cur,
            node_type=node_type,
            filter=options.filter(node_type, filter),
            sort=sort,
            first=first,
            skip=skip,
            after=after,
            properties=options.select(node_type),
        )
        graph = NodeDataGraph(scope, node_types, nodes=roots.nodes)
        return roots, graph


# TODO :Performance: use psycopg3/postgres pipelining to batch edits?


@_trace_pg_span
async def pg_edit(
    *,
    cur: psycopg.AsyncCursor,
    edits: list[EditData] | tuple[EditData, ...],
    cascade: bittuple[EditType] = CASCADING_EDIT_TYPES,
) -> tuple[list["int"], list[EditData]]:
    """
    Writes 'regular' edits to nodes (that aren't stored specially like records).
    Returns the new revisions of the edited nodes.
    """
    if not edits:
        return [], []

    # batch operations by edit kind and node type
    batch_node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, edits[0].node_ptr.type)]
    batch_updated_properties: bitarray = bitarray(batch_node_cls.__max_property_ord__ + 1)
    batch: list[EditData] = []
    all_new_revisions: list[int] = []
    all_cascaded_edits: list[EditData] = []

    for i, prev_edit in enumerate(edits):
        next_edit = edits[i + 1] if i + 1 < len(edits) else None
        batch.append(prev_edit)

        # accumulate updated properties
        for prop_id in prev_edit.properties:
            prop_ord = batch_node_cls.__properties_by_id__[prop_id].ord
            batch_updated_properties[prop_ord] = True

        # continue batch if same edit + node types
        if (
            next_edit is not None
            and next_edit.type == prev_edit.type
            and next_edit.node_ptr.type == prev_edit.node_ptr.type
        ):
            continue

        # flush batch (at end or next is different)
        edit_type: EditType = wiring.unpack_enum(EditType, prev_edit.type)
        node_type = wiring.unpack_enum(NodeType, prev_edit.node_ptr.type)
        del prev_edit  # for clarity

        # cascade edits down
        # NOTE :Performance: we probably don't _always_ need to cascade down removes
        #  (for instance in Host we may the edited graph loaded, so we could do this in memory)
        if edit_type in cascade and node_type in HAS_CHILD_NODE_TYPES:
            cascaded_edits = await _pg_edit_cascade(
                cur=cur, edit_type=edit_type, node_type=node_type, batch=batch
            )
            all_cascaded_edits.extend(cascaded_edits)

        # write edits to pg
        updated_properties = batch_node_cls._unmask_properties(batch_updated_properties)
        changed_nodes = await _pg_edit_batch(
            cur=cur,
            edit_type=edit_type,
            node_type=cast(NodeType, node_type),
            batch=batch,
            return_nodes=True,
            updated_properties=updated_properties,
            selected_properties=cast(
                tuple[Property, ...], (batch_node_cls.id, batch_node_cls.revision)
            ),
        )
        assert changed_nodes is not None, f"no nodes returned for {batch!r}"
        assert len(changed_nodes) == len(batch), f"unexpected nodes: {changed_nodes!r}"
        # NOTE: in case of multiple edits to the same node, the returned revision is the latest.
        new_revisions_by_id = {node.id: node.revision for node in changed_nodes}
        for edit in batch:
            node_id = cast(str, edit.node_ptr.id)
            all_new_revisions.append(new_revisions_by_id[node_id])

        # start new batch if needed
        if next_edit is not None:
            batch_node_cls = NODE_CLASS_BY_TYPE[
                wiring.unpack_enum(NodeType, next_edit.node_ptr.type)
            ]
            batch_updated_properties = bitarray(batch_node_cls.__max_property_ord__ + 1)
            batch.clear()

    return all_new_revisions, all_cascaded_edits


@_trace_pg_span
async def _pg_edit_cascade(
    *, cur: psycopg.AsyncCursor, edit_type: EditType, node_type: NodeType, batch: list[EditData]
) -> list[EditData]:
    """Cascades a batch of edits to the relevant descendants of the node."""

    # figure out which nodes to cascade to
    root_nodes = tuple(root_edit.node_ptr for root_edit in batch)
    root_edit_by_node_id = {cast(str, root_edit.node_ptr.id): root_edit for root_edit in batch}
    if edit_type in (EditType.UNARCHIVE, EditType.RESTORE):
        # only cascade to nodes that were removed at the exact same time
        removed_dts = []
        for root_edit in batch:
            assert root_edit.old_node_packed is not None, f"no old node for {root_edit!r}"
            old_node = unpack_node_delta(root_edit.old_node_packed, node_type=node_type)
            if edit_type == EditType.UNARCHIVE:
                removed_at = old_node.archived_at
            elif edit_type == EditType.RESTORE:
                removed_at = old_node.deleted_at
            else:
                raise RuntimeError(f"unexpected edit type: {edit_type!r}")
            assert removed_at is not None, f"no removed at for {root_edit!r}"
            removed_dts.append(removed_at)
        extra_filter = C(
            op=ConditionalOp.IN,
            property=Node.archived_at if edit_type == EditType.UNARCHIVE else Node.deleted_at,
            value=removed_dts,
        )
    elif edit_type == EditType.ERASE:
        # cascade to all
        extra_filter = None
    else:
        # only cascade to visible
        extra_filter = FILTER_VISIBLE

    # select cascaded nodes from graph
    _, cascaded_nodes_by_root_id = await pg_walk_graph_down(
        cur=cur,
        roots=root_nodes,
        # only descend to node types in the same store
        descendant_types=DESCENDANT_NODE_TYPES_IN_STORE[node_type],
        extra_filter=extra_filter,
    )

    # turn into cascaded edits with source root for exact context
    all_cascaded_edits: list[EditData] = []
    for root_id, node_ptrs in cascaded_nodes_by_root_id.items():
        root_edit = root_edit_by_node_id[root_id]
        for node_ptr in node_ptrs:
            cascaded_edit = EditData(
                id=str(UUIDT()),
                type=cast(wire.EditType, edit_type),
                node_ptr=node_ptr,
                edited_at=root_edit.edited_at,
                epoch=root_edit.epoch,
                subject_ptr=root_edit.subject_ptr,
                context=root_edit.context,
            )
            all_cascaded_edits.append(cascaded_edit)

    # batch operations by edit kind and node type
    cascaded_edits_by_type = group_by(all_cascaded_edits, lambda edit: edit.node_ptr.type)
    for descendant_node_type, cascaded_edits in cascaded_edits_by_type.items():
        nodes = await _pg_edit_batch(
            cur=cur,
            edit_type=edit_type,
            node_type=NodeType(descendant_node_type),
            batch=cascaded_edits,
            return_nodes=True,
            updated_properties=(),
            selected_properties=SELECT_ALL_PROPERTIES[cast(NodeType, descendant_node_type)],
        )

        # and assign new/old node to edit that we have the full data
        assert nodes
        assert len(nodes) == len(cascaded_edits)
        for node, cascaded_edit in zip(nodes, cascaded_edits):
            if edit_type in (EditType.UNARCHIVE, EditType.RESTORE):
                cascaded_edit.new_node_packed = pack_node_delta(node)
            elif edit_type in (EditType.ARCHIVE, EditType.DELETE, EditType.ERASE):
                cascaded_edit.old_node_packed = pack_node_delta(node)
            else:
                raise RuntimeError(
                    f"unexpected cascaded edit type{edit_type!r} for {cascaded_edit!r}"
                )

    return all_cascaded_edits


@_trace_pg_span
async def _pg_edit_batch(
    *,
    cur: psycopg.AsyncCursor,
    edit_type: EditType,
    node_type: NodeType,
    batch: list[EditData],
    updated_properties: tuple[Property, ...],  # across batch
    return_nodes: bool,
    selected_properties: tuple[Property, ...],
) -> tuple["AnyNodeData", ...] | list["AnyNodeData"] | None:
    """
    Writes a batch of regular (not custom stored) node edits of the same edit type.
    """
    trace.get_current_span().set_attributes(
        {"edit_type": edit_type.bench_name, "edits": len(batch)}
    )

    node_cls = NODE_CLASS_BY_TYPE[node_type]
    table = node_cls.__table__
    assert table is not None, f"no table for {node_cls!r}"
    assert table._primary_key is not None, f"no primary key for {node_cls!r}: {table!r}"
    selected_columns = tuple(prop.column for prop in selected_properties) if return_nodes else None

    if edit_type in (EditType.CREATE, EditType.UPSERT):
        nodes = []
        rows = []
        for edit in batch:
            assert edit.epoch is not None, f"no epoch for {edit!r}"
            assert edit.new_node_packed, f"no new node for {edit!r}"
            node = unpack_node_delta(edit.new_node_packed)
            nodes.append(node)
            # inline implicit metadata
            row: dict[str, SqlPrimitive] = pg_pack_node_data_row(node)
            row["created_at"] = row["updated_at"] = edit.edited_at
            if "created_epoch" in node_cls.__properties__:
                row["created_epoch"] = row["updated_epoch"] = edit.epoch
            _pg_pack_node_reference_into_row(Node.created_by, row, edit.subject_ptr)
            _pg_pack_node_reference_into_row(Node.updated_by, row, edit.subject_ptr)
            rows.append(row)

        if edit_type == EditType.CREATE:
            _ = await pg_insert(cur=cur, table=table, rows=rows)
            if return_nodes:
                return nodes  # ithe nodes are equivalent to the rows (no need to unpack again)
            else:
                return None
        else:
            rows = await pg_upsert(
                cur=cur,
                table=table,
                rows=rows,
                conflict_columns=(table._primary_key,),
                static_columns=tuple(c for c in table.columns if c != table._primary_key),
                static_values={"revision": sqlstr(f"{table.name}.revision + 1")},
                returning=selected_columns if return_nodes else None,
            )
            if return_nodes:
                assert rows is not None, f"no rows returned for {batch!r}"
                return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
            else:
                return None

    elif edit_type in (
        EditType.UPDATE,
        EditType.MOVE,
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.DELETE,
        EditType.RESTORE,
    ):
        # collect dynamic columns (incl. implicit metadata)
        implicit_properties: list[Property | Any] = [node_cls.updated_at, node_cls.updated_by]
        if issubclass(node_cls, BenchNode):
            implicit_properties.append(node_cls.updated_epoch)
        if edit_type in (EditType.ARCHIVE, EditType.UNARCHIVE):
            implicit_properties.append(node_cls.archived_at)
        elif edit_type in (EditType.DELETE, EditType.RESTORE):
            implicit_properties.append(node_cls.deleted_at)
        dynamic_columns: list[Column] = [table._primary_key]
        for prop in chain(implicit_properties, updated_properties):
            if prop.is_node_reference:
                dynamic_columns.extend(p.column for p in prop.reference_stored_props or ())
            else:
                dynamic_columns.append(prop.column)

        # collect dynamic values
        dynamic_values: list[RowIn] = []
        for edit in batch:
            assert edit.epoch is not None, f"no epoch for {edit!r}"
            if edit_type == EditType.UPDATE or edit_type == EditType.MOVE:
                assert edit.new_node_packed, f"no new node for {edit!r}"
                new_node_data = unpack_node_delta(edit.new_node_packed, node_type=node_type)
            else:
                new_node_data = None
            row = {"id": edit.node_ptr.id}
            # directly edited properties
            for prop_id in edit.properties:
                prop = node_cls.__properties_by_id__.get(prop_id)
                assert prop is not None, f"no property {prop_id!r} in {node_cls!r} for {edit!r}"
                if prop.is_node_reference:
                    value = getattr(new_node_data, cast(Property, prop.reference_wired_ptr).name)
                    _pg_pack_node_reference_into_row(prop, row, value)
                else:
                    value = getattr(new_node_data, prop.name)
                    value = _pack_struct_data_prop(prop, value, ignore_array=False)
                    row[prop.name] = value
            # implicit properties
            row["updated_at"] = edit.edited_at
            if "updated_epoch" in node_cls.__properties__:
                row["updated_epoch"] = edit.epoch
            _pg_pack_node_reference_into_row(Node.updated_by, row, edit.subject_ptr)
            if edit_type == EditType.ARCHIVE:
                row["archived_at"] = edit.edited_at
            elif edit_type == EditType.UNARCHIVE:
                row["archived_at"] = None
            elif edit_type == EditType.DELETE:
                row["deleted_at"] = edit.edited_at
            elif edit_type == EditType.RESTORE:
                row["deleted_at"] = None
            dynamic_values.append(row)

        # actually update
        static_values = {"revision": sqlstr("revision + 1")}
        rows = await pg_update_variable(
            cur=cur,
            table=table,
            static_values=static_values,
            dynamic_columns=dynamic_columns,
            dynamic_values=dynamic_values,
            returning=selected_columns if return_nodes else (table._primary_key,),
        )
        if rows is None or len(rows) != len(batch) or any(r is None for r in rows):
            missing_rows = {edit.node_ptr.id for edit in batch} - {
                cast(str, r["id"]) for r in rows or () if r
            }
            raise SqlNotExistsError(f"missing {node_type.bench_name}: {missing_rows}", cur)
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    elif edit_type == EditType.ERASE:
        nodes_ids = [edit.node_ptr.id for edit in batch]
        where = SqlComparison(
            left=sqlident("id"),
            op=PostgresConditionalOp.EQ,
            right=sqlstr("ANY({})").format(sql.Literal(nodes_ids)),
        )
        rows = await pg_delete(
            cur=cur,
            table=table,
            where=where,
            returning=selected_columns if return_nodes else (table._primary_key,),
        )
        if rows is None or len(rows) != len(batch) or any(r is None for r in rows):
            missing_rows = set(nodes_ids) - {cast(str, row["id"]) for row in rows or () if row}
            raise SqlNotExistsError(f"missing {node_type.bench_name}: {missing_rows}", cur)
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    else:
        raise ValueError(f"unexpected edit kind {edit_type} {node_type} for {batch!r}")


@cachetools.cached({})
def encode_pg_cursor(i: int, exclusive: bool = True) -> str:
    if not exclusive:
        i -= 1  # include current element
    return base64.b64encode(struct.pack("q", i)).decode("ascii")


@cachetools.cached({})
def decode_pg_cursor(s: str) -> int:
    return struct.unpack("q", base64.b64decode(s))[0]


def sql_to_str(cur: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
    s_str = s.as_string(cur)
    return s_str


#
# General table registry
# (this needs to run after setup)
#

TABLE_BY_NODE_TYPE: dict[NodeType, Table] = {
    # read previously generated tables in schema.py
    node_type: getattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
    for node_type in NODE_TYPES
    if hasattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
}
NODE_TYPE_BY_TABLE_NAME: dict[str, NodeType] = {
    table.name: node_type for node_type, table in TABLE_BY_NODE_TYPE.items()
}
NODE_TABLES: tuple[Table, ...] = tuple(TABLE_BY_NODE_TYPE.values())
GLOBAL_TABLES: tuple[Table, ...] = DEFAULT_GLOBAL_TABLES + tuple(
    TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if not node.__is_local__
    and node.__is_stored__
    and not node.__is_stored_custom__
    and node.metatype in TABLE_BY_NODE_TYPE
)
LOCAL_TABLES: tuple[Table, ...] = DEFAULT_LOCAL_TABLES + tuple(
    TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__is_local__
    and node.__is_stored__
    and not node.__is_stored_custom__
    and node.metatype in TABLE_BY_NODE_TYPE
)
ALL_TABLES: tuple[Table, ...] = GLOBAL_TABLES + LOCAL_TABLES
GLOBAL_SCHEMA = Schema(GLOBAL_EXTENSIONS, GLOBAL_TABLES)
LOCAL_SCHEMA = Schema(LOCAL_EXTENSIONS, LOCAL_TABLES)

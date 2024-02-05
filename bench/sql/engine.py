import base64
import enum
import struct
from collections import defaultdict
from dataclasses import dataclass
from itertools import chain
from typing import (
    Any,
    Collection,
    Mapping,
    NamedTuple,
    Optional,
    TypeVar,
    Union,
    cast,
)
from uuid import UUID, uuid4

import cachetools
import psycopg
import pytz
import structlog
from psycopg import sql
from psycopg.types.json import Jsonb

from bench.language import Block, ConditionalOp, Field, Package, QueryEngine, Session, TypeInfo
from bench.language.access import ReadOptions
from bench.language.const import NODE_TYPES, BenchError, EditType, NodeType, SortOp
from bench.language.database import HasDatabase, Record
from bench.language.expression import (
    TYPE_DISCRIMINATOR_KEY,
    C,
    Expression,
    ExpressionOps,
    QueryEngineIncapableError,
)
from bench.language.node import (
    NODE_CLASS_BY_TYPE,
    NODE_CLASSES,
    PARENT_NODE_TYPES,
    UNSET,
    Node,
    Property,
    get_node_id,
)
from bench.language.tree import NodeDataTree
from bench.proto import wire, wiring
from bench.proto.wire import AnyNodeData, EditData, NodeReferenceData, IdEnum
from bench.proto.wiring import PROTO_CLASS_BY_TYPE
from bench.sql import schema
from bench.sql.client import (
    GLOBAL_PG_CRYPTO_KEY,
    UNIVERSAL_RO_PASSWORD,
    UNIVERSAL_RO_USERNAME,
    async_pg_cursor,
)
from bench.sql.core import (
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    RECORD_BASE_TABLE,
    CascadeAction,
    Column,
    Constraint,
    ConstraintType,
    Index,
    IndexType,
    PrimitiveType,
    SqlPrimitive,
    Table,
)
from bench.utils.casing import Casing, to_casing
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import describe_type, to_uuid
from bench.utils.utils import IS_DEBUG, IS_LOCAL, IS_TEST

logger = structlog.get_logger(__name__)

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
    ConditionalOp.REGEX: PostgresConditionalOp.REGEXP,
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


def get_database_table_name(block_ck: UUID) -> str:
    """
    Gets the name for a table with the Records of a dynamically created DatabaseBlock.
    NOTE: we rely on this table prefix to remain constant
    """
    return f"bench_record_{str(block_ck).replace('-', '')}"


def get_bench_table_name(node_type: NodeType) -> str:
    """Gets the name for a regular Bench node table."""
    return f"bench_{node_type.name.lower().replace('_', '')}"


def map_node_class_to_pg_table(node: type[Node]) -> Table:
    # TODO @Robustness: add Bench check constraints in Postgres
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
        column = Column(
            name=prop.name,
            type=prop.primitive_type,
            is_array=prop.is_array,
            is_nullable=not prop.is_required,
            is_encrypted=prop.is_encrypted,
            is_primary_key=prop.name == "id",
            is_unique=prop.is_unique,
            _source=prop.id,
        )
        # default
        if prop.default is not UNSET and prop.default is not None:
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
        # references
        if (
            prop.reference_types
            and prop.name.endswith("_id")
            and node.__is_local__ == NODE_CLASS_BY_TYPE[prop.reference_types[0]].__is_local__
        ):
            assert len(prop.reference_types) == 1, f"stored prop {prop!r} has multiple references"
            column.is_foreign_key_to = get_bench_table_name(prop.reference_types[0])
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

    # one of the parent_<type>_id columns must be non-null
    parent_columns: tuple[str, ...] = tuple(c.name for c in columns if c.name.startswith("parent_"))
    if parent_columns:
        constraint = Constraint(
            "bench_check_one_parent",
            type=ConstraintType.CHECK,
            condition=f"({') OR ('.join(f'{c} IS NOT NULL' for c in parent_columns)})",
            _source=node.metatype.id,
        )
        constraints.append(constraint)

    # index [package] + deleted_at/archived_at if applicable
    for prop_name in ("deleted_at", "archived_at"):
        prop = node.__properties__.get(prop_name)
        if prop is None:
            continue
        if node.__is_in_package__ and "package_id" in node.__properties__:
            index = Index(
                f"bench_idx_package_{prop_name}",
                type=IndexType.BTREE,
                columns=("package_id", prop_name),
                _source=prop.id,
            )
        else:
            index = Index(
                f"bench_idx_{prop_name}",
                type=IndexType.BTREE,
                columns=(prop_name,),
                _source=prop.id,
            )
        indexes.append(index)

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


def map_database_to_pg_table(database: Block) -> Table:
    """Gets the full table with all specific fields of a database and general record stuff."""
    columns: list[Column] = []
    indexes: list[Index] = []
    constraints: list[Constraint] = []

    for field in database.fields:
        type: TypeInfo = field.resolved_type
        column = Column(
            _source=str(field.ck),
            name=get_field_column_name(field),
            type=type.primitive_type,
            is_array=type.is_array,
            is_nullable=True,
            is_encrypted=type.is_secret,
        )
        columns.append(column)

    return Table(
        _source=str(database.ck),
        name=get_database_table_name(database.ck),
        columns=tuple(*(c.clone() for c in RECORD_BASE_TABLE.columns), *columns),
        indexes=tuple(*(i.clone() for i in RECORD_BASE_TABLE.indexes), *indexes),
        constraints=tuple(*(c.clone() for c in RECORD_BASE_TABLE.constraints), *constraints),
    )


async def update_dynamic_local_pg_schema(pg_name: str, package: Package) -> None:
    """Updates the dynamic local record Postgres tables for a package's databases."""
    from bench.sql.migration import (
        MigrationOpKind,
        apply_migration_ops,
        generate_migration_ops,
        introspect_tables_from_pg,
    )

    log = logger.bind(pg_name=pg_name, package=package)
    databases: list[Block] = [
        cast(Block, s)
        for s in package._nodes
        if s.metatype == NodeType.BLOCK and HasDatabase in s._components and not s.ephemeral
    ]
    tables: list[Table] = [d._table for d in databases]
    log.info("pg.update_schema", databases=len(databases), tables=len(tables))

    try:
        async with async_pg_cursor(pg_name, autocommit=False) as cur:
            # introspect and update schema
            old_tables = await introspect_tables_from_pg(cur, table_prefix="bench_record_")
            new_tables = [map_database_to_pg_table(d) for d in databases]
            migration_ops = generate_migration_ops(old_tables, new_tables)
            # we don't do deletes here
            migration_ops = [
                m
                for m in migration_ops
                if m.kind
                in (MigrationOpKind.CREATE, MigrationOpKind.UPDATE, MigrationOpKind.RENAME)
            ]
            await apply_migration_ops(cur, migration_ops)
    except Exception as e:
        log.exception("pg.update_schema.failed", e=e)
        raise RuntimeError(f"failed to update {pg_name} schema: {e}") from e


class SqlError(BenchError):
    pass


class SqlUndefinedObjectError(SqlError):
    pass


class SqlViolation(SqlError):
    pass


class SqlAlreadyExistsError(SqlError):
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


def _compile_expression_ref(
    node: Union[type[Node], Block],
    expr: Expression,
) -> SqlNode:
    if expr.property is not None:
        return sql.Identifier(expr.property.name)
    elif expr.field is not None:
        assert (
            expr.field._introspected_from is None
        ), f"cannot use introspected: {expr!r}->{expr.field!r}"
        if isinstance(node, Block) and node.ephemeral:
            return SqlJsonPath(sql.Identifier("value"), [expr.field.storage_key])
        else:
            return sql.Identifier(get_field_column_name(expr.field))
    else:
        raise TypeError(f"unexpected expression ref: {expr!r}")


def compile_pg_conditional(
    node: Union[type[Node], Block],
    cond: Expression | None,
) -> SqlNode:
    if cond.op == ConditionalOp.TRUE:
        return sql.SQL("TRUE")
    elif cond.op == ConditionalOp.FALSE:
        return sql.SQL("FALSE")
    elif cond.op in ExpressionOps.COND_LOGICAL and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        clauses = [compile_pg_conditional(node, c) for c in cond.clauses]
        return SqlCompound(op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], operands=clauses)
    elif (
        cond.op in ExpressionOps.COND_COMPARISON or cond.op in ExpressionOps.COND_STRING
    ) and cond.op in PG_CONDITIONAL_OP_BY_BENCH:
        left = _compile_expression_ref(node, cond)
        if isinstance(cond.field, Field):  # add explicit cast to LHS if possible
            pg_type = PG_CAST_PRIMITIVE_TYPE[cond.field._storage_format]
            left = sql.SQL("({})::{}").format(sql_node_to_sql(left), sql.SQL(pg_type))
        # map IN to ANY() construct (IN/NOT IN doesn't work in psycopg)
        if cond.op in (ConditionalOp.IN, ConditionalOp.NOT_IN):
            # psycopg also can't handle tuples, so list it is
            value = list(cond.value) if not isinstance(cond.value, list) else cond.value
            right = sql.SQL("ANY({})").format(sql.Literal(value))
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
            assert cond.value is not None, f"cannot compare {cond!r} to None"
            right = sql.Literal(cond.value)

        return SqlComparison(left=left, op=PG_CONDITIONAL_OP_BY_BENCH[cond.op], right=right)
    elif cond.op in ExpressionOps.COND_EXISTENCE:
        return SqlUnary(
            left=_compile_expression_ref(node, cond),
            op=PG_CONDITIONAL_OP_BY_BENCH[cond.op],
        )
    raise QueryEngineIncapableError(QueryEngine.LOCAL_POSTGRES, cond, "unsupported conditional")


def compile_pg_sort(
    database: Block,
    sort: Expression,
) -> SqlNode:
    field_ref = _compile_expression_ref(database, sort)
    return sql.SQL("{} {}").format(
        sql_node_to_sql(field_ref), sql.SQL(POSTGRES_SORT_OP_BY_BENCH[sort.op])
    )


def compile_pg_sorts(
    database: Block,
    sorts: list[Expression],
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


def _pg_wrap_error(resource: Any, e: psycopg.errors.Error) -> Exception:
    if isinstance(e, (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn)):
        wrapped_t = SqlUndefinedObjectError
    elif isinstance(e, (psycopg.errors.UniqueViolation,)):
        wrapped_t = SqlAlreadyExistsError
    elif "Violation" in e.__class__.__name__:
        wrapped_t = SqlViolation
    else:
        wrapped_t = SqlError
    e_str = str(e)
    if "\n" in e_str:
        message = f"{e}\nin {resource!r}"
    else:
        message = f"{e} in {resource!r}"
    return wrapped_t(message)


async def pg_select_raw(cur: psycopg.AsyncCursor, query: sql.Composable) -> list[dict[str, any]]:
    """Executes an arbitrary select without any wrapping."""
    logger.debug("pg.select_raw", query=sql_to_str(cur, query))
    await cur.execute(query)
    return await cur.fetchall()


# TODO @Cleanup @Security: parameterize pg crypto key per database & pass more selectively


def _pg_wrap_write_column(column: Column, value: SqlNode) -> SqlNode:
    if column.is_encrypted:
        assert not column.is_array, f"cannot encrypt array column: {column!r}"
        if not isinstance(value, sql.Composable) and column._unencrypted_type == PrimitiveType.JSON:
            value = Jsonb(value)  # adapt json
        original_value = value
        # first to bytea
        if column._unencrypted_type == PrimitiveType.BYTES:
            value = sql.SQL("{}::bytea").format(value)
        elif column._unencrypted_type in (PrimitiveType.STRING, PrimitiveType.JSON):
            value = sql.SQL("convert_to({}::text, 'UTF8')").format(value)
        else:
            cast = PG_CAST_PRIMITIVE_TYPE[column._unencrypted_type]
            value = sql.SQL("{}::{}::text::bytea").format(value, sql.SQL(cast))
        # then encrypt with pgp_sym_encrypt_bytea
        value = sql.SQL("pgp_sym_encrypt_bytea({}, %(PG_CRYPTO_KEY)s::text)").format(
            sql_node_to_sql(value)
        )
        # bail if original value is null
        # value = sql.SQL("(CASE WHEN {} IS NULL THEN NULL ELSE {} END)").format(
        #     sql_node_to_sql(original_value), value
        # )
        return value
    else:
        return value


def _pg_wrap_read_column(column: Column, value: SqlNode) -> SqlNode:
    if column.is_encrypted:
        assert not column.is_array, f"cannot encrypt array column: {column!r}"
        original = value
        # first decrypt with pgp_sym_decrypt_bytea
        value = sql.SQL("pgp_sym_decrypt_bytea({}, %(PG_CRYPTO_KEY)s::text)").format(
            sql_node_to_sql(value), sql.Literal(GLOBAL_PG_CRYPTO_KEY)
        )
        # then convert from bytea to the correct type
        if column._unencrypted_type == PrimitiveType.BYTES:
            value = sql.SQL("{}::bytea").format(value)
        else:
            cast = PG_CAST_PRIMITIVE_TYPE[column._unencrypted_type]
            value = sql.SQL("convert_from({}::bytea, 'UTF8')::text::{}").format(
                value, sql.SQL(cast)
            )
        # and bail if original value is null
        value = sql.SQL("(CASE WHEN {} IS NULL THEN NULL ELSE {} END)").format(
            sql_node_to_sql(original), value
        )
        # and label column
        value = sql.SQL("{} as {}").format(value, sql.Identifier(column.name))
        return value
    else:
        return value


def _pg_adapt_row(table: Table, row: Mapping[str, any]) -> Mapping[str, any]:
    """Adapts and wraps any values"""
    wrapped = {}
    for column in table.columns:
        if column.name not in row:
            continue
        value = row.get(column.name)
        if value is None:
            pass
        elif column.underlying_type == PrimitiveType.JSON:
            if column.is_array:
                value = [Jsonb(v) for v in value]
            else:
                value = Jsonb(value)
        wrapped[column.name] = value
    return wrapped


def _pg_adapt_rows(
    table: Table, rows: tuple[Mapping[str, any], ...]
) -> tuple[Mapping[str, any], ...]:
    return tuple(_pg_adapt_row(table, row) for row in rows)


async def _pg_fetchall_from_many(cur: psycopg.AsyncCursor, expected: int) -> list[RowOut]:
    # see https://www.psycopg.org/psycopg3/docs/api/cursors.html#psycopg.Cursor.executemany
    results: list[RowOut] = []
    while True:
        row = await cur.fetchone()
        results.append(row)
        if not cur.nextset():
            break
    assert len(results) == expected, f"wanted {expected} results, got {len(results)}"
    return results


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
    params: Mapping | None = None,
) -> list[RowOut]:
    """Selects from the given table."""
    columns = columns or table.columns
    statement = sql.SQL("SELECT {fields} FROM {table}").format(
        fields=sql.SQL(", ").join(_pg_wrap_read_column(c, sql.Identifier(c.name)) for c in columns),
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
    logger.debug("pg.select", table=table, query=sql_to_str(cur, statement))
    if any(c.is_encrypted for c in columns):
        params = {**(params or {}), "PG_CRYPTO_KEY": GLOBAL_PG_CRYPTO_KEY}
    try:
        await cur.execute(statement, params)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    return await cur.fetchall()


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
    logger.debug("pg.count", table=table, query=sql_to_str(cur, statement))
    try:
        await cur.execute(statement)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
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
    try:
        await cur.execute(statement)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    return (await cur.fetchone())["exists"]


async def pg_insert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: tuple[RowIn, ...] | list[RowIn],
    *,
    returning: Collection[Column] | None = None,
) -> tuple[RowOut, ...] | list[RowOut] | None:
    """Inserts into the given table. Expects rows to be adapted and wrapped."""
    statement = sql.SQL("INSERT INTO {table} ({fields}) VALUES ({values})").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ").join(
            _pg_wrap_write_column(c, sql.SQL(f"%({c.name})s")) for c in table.columns
        ),
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(_pg_wrap_read_column(c, sql.Identifier(c.name)) for c in returning)
        )
    logger.debug("pg.insert", table=table, query=sql_to_str(cur, statement))

    if any(c.is_encrypted for c in table.columns):
        templated_values = tuple({**row, "PG_CRYPTO_KEY": GLOBAL_PG_CRYPTO_KEY} for row in rows)
    else:
        templated_values = rows
    try:
        await cur.executemany(statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(rows))


async def pg_upsert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: tuple[RowIn, ...] | list[RowIn],
    *,
    conflict_columns: list[Column] | tuple[Column, ...] | None = None,
    update_columns: list[Column] | tuple[Column, ...] | None = None,
    update_values: RowIn | None = None,
    returning: Collection[Column] | None = None,
) -> tuple[RowOut, ...] | list[RowOut] | None:
    """Upserts into the given table. Expect rows to be adapted and wrapped."""
    if conflict_columns is None:
        conflict_columns = (table._primary_key,)
    if update_columns is None:
        update_columns = tuple(c for c in table.columns if c not in conflict_columns)
    if update_values:
        static_update_columns = tuple(c for c in update_columns if c.name not in update_values)
    else:
        static_update_columns = update_columns

    static_update = tuple(
        sql.SQL("{} = EXCLUDED.{}").format(sql.Identifier(c.name), sql.Identifier(c.name))
        for c in static_update_columns
    )
    if update_values:
        dynamic_update = tuple(
            sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
            for k, v in update_values.items()
        )
    else:
        dynamic_update = ()

    statement = sql.SQL(
        "INSERT INTO {table} ({fields}) VALUES ({values}) ON CONFLICT ({conflict}) DO UPDATE SET {updates}"
    ).format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ").join(
            _pg_wrap_write_column(c, sql.SQL(f"%({c.name})s")) for c in table.columns
        ),
        conflict=sql.SQL(", ").join(sql.Identifier(c.name) for c in conflict_columns),
        updates=sql.SQL(", ").join(chain(static_update, dynamic_update)),
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(_pg_wrap_read_column(c, sql.Identifier(c.name)) for c in returning)
        )
    logger.debug("pg.upsert", table=table, query=sql_to_str(cur, statement))

    if any(c.is_encrypted for c in table.columns):
        templated_values = tuple({**row, "PG_CRYPTO_KEY": GLOBAL_PG_CRYPTO_KEY} for row in rows)
    else:
        templated_values = rows
    try:
        await cur.executemany(statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(rows))


async def pg_update_static(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    static_value: RowIn,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with static values. Expects values to be adapted and wrapped."""
    statement = sql.SQL("UPDATE {table} SET {values}").format(
        table=sql.Identifier(table.name),
        values=sql.SQL(", ").join(
            sql.SQL("{} = {}").format(
                sql.Identifier(k),
                _pg_wrap_write_column(table._columns_by_name[k], sql.SQL(f"%({k})s")),
            )
            for k in static_value.keys()
        ),
    )
    if where:
        statement += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(_pg_wrap_read_column(c, sql.Identifier(c.name)) for c in returning)
        )
    logger.debug("pg.update_static", table=table, query=sql_to_str(cur, statement))

    if any(c.is_encrypted for c in table.columns):
        template_values = {**static_value, "PG_CRYPTO_KEY": GLOBAL_PG_CRYPTO_KEY}
    else:
        template_values = static_value
    try:
        await cur.execute(statement, template_values)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    if returning:
        return await cur.fetchall()


async def pg_update_dynamic(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    dynamic_columns: Collection[Column],
    dynamic_values: Collection[RowIn],
    static_values: RowIn = None,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table with a list of values (corresponding to rows). Expects values to be adapted and wrapped."""
    if not any(c.is_primary_key for c in dynamic_columns):
        raise ValueError(
            f"dynamic_columns {dynamic_columns!r} do not contain {table._primary_key!r}"
        )
    table_name = sql.Identifier(table.name)
    static_values = static_values or {}

    # join fixed and dynamic values
    static_values_sql = (
        sql.SQL("{} = {}").format(
            sql.Identifier(k), _pg_wrap_write_column(table._columns_by_name[k], v)
        )
        for k, v in static_values.items()
    )
    dynamic_values_sql = (
        sql.SQL("{} = {}").format(
            sql.Identifier(c.name), _pg_wrap_write_column(c, sql.SQL(f"%({c.name})s"))
        )
        for c in dynamic_columns
    )
    values_sql = sql.SQL(", ").join(
        chain(static_values_sql, dynamic_values_sql),
    )
    statement = sql.SQL("UPDATE {table} SET {values} WHERE {pk} = %(pk)s").format(
        table=table_name, pk=sql.Identifier(table._primary_key.name), values=values_sql
    )
    if returning:
        statement += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(
                sql.SQL("{}").format(_pg_wrap_read_column(c, sql.Identifier(c.name)))
                for c in returning
            )
        )
    logger.debug(
        "pg.update_dynamic",
        table=table,
        query=sql_to_str(cur, statement),
        rows=len(dynamic_values),
    )

    if any(c.is_encrypted for c in table.columns):
        templated_values = tuple(
            {**row, "pk": row.get(table._primary_key.name), "PG_CRYPTO_KEY": GLOBAL_PG_CRYPTO_KEY}
            for row in dynamic_values
        )
    else:
        templated_values = tuple(
            {**row, "pk": row.get(table._primary_key.name)} for row in dynamic_values
        )
    try:
        await cur.executemany(statement, templated_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    if returning:
        return await _pg_fetchall_from_many(cur, len(dynamic_values))


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
            sql.SQL(", ").join(_pg_wrap_read_column(c, sql.Identifier(c.name)) for c in returning)
        )
    logger.debug("pg.delete", table=table, query=sql_to_str(cur, statement))
    try:
        await cur.execute(statement)
    except psycopg.errors.Error as e:
        raise _pg_wrap_error(table, e) from e
    if returning:
        return await cur.fetchall()


async def pg_truncate(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Truncates the given table."""
    await cur.execute(sql.SQL("TRUNCATE TABLE {}").format(sql.Identifier(table.name)))


#
# Nodes API
# Higher level methods use Session and return Nodes, but handle respective PG cursors.
# Lower level methods use data constructs and expect appropriate PG cursors.
#

NodeT = TypeVar("NodeT", bound=Node)

DEFAULT_GLOBAL_FILTER: Expression = Node.filter(archived_at=None, deleted_at=None)._filter
DEFAULT_SELECTED_PROPERTIES: Mapping[NodeType, tuple[Property, ...]] = {
    node.metatype: tuple(
        prop for prop in node.__stored_properties__.values() if not prop.is_deferred
    )
    for node in NODE_CLASSES
}


def _pack_struct_data_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_data_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        value = value.to_robust_dict(prop.struct_type)
        return wiring.pack_json_value(value)
    elif prop.is_enum:
        return value.value
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(wiring.unpack_json_value(value))
    else:
        return value


def _unpack_struct_data_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_unpack_struct_data_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        proto_cls = PROTO_CLASS_BY_TYPE[prop.struct_type]
        value = wiring.unpack_json_value(value)
        return proto_cls().from_robust_dict(value, prop.struct_type)
    elif prop.is_enum:
        return wiring.pack_enum(prop.py_type_stripped, prop.py_type_stripped(value))
    elif prop.primitive_type == PrimitiveType.DATETIME:
        if value.tzinfo is None:
            return value.replace(tzinfo=pytz.utc)
        else:
            return value
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return wiring.pack_json_value(value)
    else:
        return value


def pg_pack_node_data_row(node: AnyNodeData) -> dict[str, any]:
    """Packs a node's data into a row for the respective table."""
    node_cls = NODE_CLASS_BY_TYPE[wiring.unpack_enum(NodeType, node.metatype)]
    try:
        row: dict[str, any] = {}
        for prop in node_cls.__stored_properties__.values():
            if prop.reference_source is None:  # regular non-ref property
                value = getattr(node, prop.name)
                value = _pack_struct_data_prop(prop, value, ignore_array=False)
                row[prop.name] = value
            else:  # unravel reference into per-type columns
                assert prop.is_array is False, f"array property not supported (yet) {prop!r}"
                ptr: NodeReferenceData | None = getattr(
                    node, prop.reference_source.reference_wired_ptr.name
                )
                if ptr is not None and prop.reference_types[0].id == ptr.type:
                    if prop.name.endswith("_ck"):
                        row[prop.name] = ptr.ck
                    else:
                        row[prop.name] = ptr.id
                else:
                    row[prop.name] = None
        return row
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        raise ValueError(f"could not pack row {node_cls.metatype.name}: {struct!r}") from e


def pg_unpack_node_data_row(node_cls: type[Node], row: dict[str, any]) -> AnyNodeData:
    """Unpacks a node's data from a row from the respective table."""
    try:
        proto_cls = PROTO_CLASS_BY_TYPE[node_cls.metatype]
        data = proto_cls(metatype=wiring.pack_enum(NodeType, node_cls.metatype))
        for prop in node_cls.__stored_properties__.values():
            value = row.get(prop.name)
            if value is None:
                continue
            elif prop.reference_source is None:  # regular non-ref property
                value = _unpack_struct_data_prop(prop, value, ignore_array=False)
                setattr(data, prop.name, value)
            else:  # ravel reference from per-type columns
                assert prop.is_array is False, f"array property not supported (yet) {prop!r}"
                if prop.name.endswith("_ck"):
                    ptr = NodeReferenceData(
                        metatype=wire.StructType.NODE_REFERENCE,
                        type=prop.reference_types[0],
                        ck=str(value),
                    )
                else:
                    ptr = NodeReferenceData(
                        metatype=wire.StructType.NODE_REFERENCE,
                        type=prop.reference_types[0],
                        id=str(value),
                    )
                assert prop.reference_source is not None, f"no reference source for {prop!r}"
                setattr(data, prop.reference_source.reference_wired_ptr.name, ptr)
        return data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        row_str = repr(row) if IS_DEBUG else describe_type(row)
        raise ValueError(f"could not unpack row {node_cls.metatype.name}: {row_str}") from e


PgSelectNodesDataResult = NamedTuple(
    "PgSelectNodesDataResult",
    [("nodes", list[wire.AnyNodeData]), ("cursors", list[str]), ("start_cursor", str | None)],
)


async def pg_select_nodes_data(
    cur: psycopg.AsyncCursor,
    node_type: NodeType,
    *,
    properties: Collection[Property] | None = None,
    filter: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> PgSelectNodesDataResult:
    """Selects regular nodes from the given PG database."""
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    if after:
        skip = (skip or 0) + int(decode_pg_cursor(after)) + 1  # 'after' is exclusive
    columns = [prop.column for prop in properties] if properties is not None else None
    filter = compile_pg_conditional(node_cls, filter) if filter is not None else None
    sort = compile_pg_sorts(node_cls, sort) if sort is not None else None
    rows = await pg_select(
        cur=cur,
        table=node_cls.__table__,
        columns=columns,
        where=filter,
        order_by=sort,
        first=first,
        skip=skip,
    )
    nodes_data = [pg_unpack_node_data_row(node_cls, row) for row in rows]
    cursors = [encode_pg_cursor(i) for i in range(skip or 0, (skip or 0) + len(nodes_data))]
    assert len(nodes_data) == len(cursors), f"unexpected cursors: {cursors} for {nodes_data}"
    return PgSelectNodesDataResult(nodes_data, cursors, after)


async def pg_read_node_data_tree(
    cur: psycopg.AsyncCursor,
    root_type: NodeType,
    root_ids: tuple[UUID, ...],
    options: ReadOptions,
    _tree: NodeDataTree | None = None,
) -> NodeDataTree | None:
    """Reads regular nodes from the given PG database. Returns a tree of nodes."""

    visited_tree = _tree if _tree is not None else NodeDataTree()

    # select "roots"
    root_filter = options.filter(root_type, C(ConditionalOp.IN, property=Node.id, value=root_ids))
    roots = await pg_select_nodes_data(
        cur=cur,
        node_type=root_type,
        filter=root_filter,
        properties=options.select(root_type),
    )
    if not roots.nodes:
        return None
    for node in roots.nodes:
        visited_tree.add(node)

    # select ancestors (recursively)
    # (basically, walk parent pointer if type is in ancestor_types)
    if options.ancestor_types:
        current_parents: list[wire.AnyNodeData] = roots.nodes
        to_select_by_type: dict[NodeType, list[str]] = defaultdict(list)
        while current_parents:
            to_select_by_type.clear()

            # traverse unseen parents to select next
            for node in current_parents:
                if (
                    node.parent_ptr is not None
                    and node.parent_ptr.type in options.ancestor_types
                    and node.parent_ptr.id not in visited_tree
                ):
                    to_select_by_type[node.parent_ptr.type].append(node.parent_ptr.id)

            # select next parents
            next_parents = []
            for node_type, node_ids in to_select_by_type.items():
                layer = await pg_select_nodes_data(
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
                    visited_tree.add(node)
            current_parents = next_parents

    # select descendants (recursively)
    # nocheckin @Performance!: recurse read nodes up?/down in SQL
    #  (take advantage of the ancestry graph to optimize this)
    if options.descendant_types:
        current_parents: list[wire.AnyNodeData] = roots.nodes
        while current_parents:
            next_parents: list[wire.AnyNodeData] = []
            # traverse all direct children of plausible types
            for child_type in options.descendant_types:
                # collect possible parents
                parents_by_type: dict[NodeType, list[str]] = defaultdict(list)
                for parent in current_parents:
                    if parent.metatype in PARENT_NODE_TYPES[child_type]:
                        parents_by_type[parent.metatype].append(parent.id)
                if not parents_by_type:
                    continue

                # build initial filter
                parents_filters: list[Expression] = []
                parent_property = NODE_CLASS_BY_TYPE[child_type].__parent_property__
                for parent_property in parent_property.reference_stored_ptrs:
                    filter = C(
                        op=ConditionalOp.IN,
                        property=parent_property,
                        value=parents_by_type[parent_property.reference_types[0]],
                    )
                    parents_filters.append(filter)
                parent_filter = C(op=ConditionalOp.OR, clauses=parents_filters)

                # collect children
                children = await pg_select_nodes_data(
                    cur=cur,
                    node_type=child_type,
                    filter=options.filter(child_type, parent_filter),
                    properties=options.select(child_type),
                )
                next_parents.extend(n for n in children.nodes if n.id not in visited_tree)
                for child in children.nodes:
                    visited_tree.add(child)

            current_parents = next_parents

    return visited_tree


async def pg_search_nodes_data_tree(
    cur: psycopg.AsyncCursor,
    node_type: NodeType,
    *,
    options: ReadOptions,
    filter: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> tuple[PgSelectNodesDataResult, NodeDataTree]:
    """Select root nodes and then read the tree of nodes from the given PG database."""

    if options.ancestor_types or options.descendant_types:
        # split into two passes if we have other nodes to fetch
        node_cls = NODE_CLASS_BY_TYPE[node_type]
        roots = await pg_select_nodes_data(
            cur=cur,
            node_type=node_type,
            filter=options.filter(node_type, filter),
            sort=sort,
            first=first,
            skip=skip,
            after=after,
            properties=(node_cls.__properties__["id"],),
        )
        if not roots.nodes:
            return roots, NodeDataTree()
        tree = await pg_read_node_data_tree(
            cur=cur,
            root_type=node_type,
            root_ids=tuple(to_uuid(node.id) for node in roots.nodes),
            options=options,
        )
        return roots, tree
    else:
        # otherwise just select in one go
        roots = await pg_select_nodes_data(
            cur=cur,
            node_type=node_type,
            filter=options.filter(node_type, filter),
            sort=sort,
            first=first,
            skip=skip,
            after=after,
            properties=options.select(node_type),
        )
        tree = NodeDataTree(nodes=roots.nodes)
        return roots, tree


async def pg_read_nodes(
    session: Session,
    root_type: NodeType,
    root_ids: tuple[UUID, ...],
    options: ReadOptions,
    parent: Node | None = None,
) -> tuple[NodeT, ...]:
    """Reads 'regular' nodes from the given PG database and unpacks them into the session. Returns the roots."""
    root_cls = NODE_CLASS_BY_TYPE[root_type]
    cur = session.local_pg_cursor if root_cls.__is_local__ else session.global_pg_cursor
    source_tree = await pg_read_node_data_tree(cur, root_type, root_ids, options)
    if source_tree is None:
        raise ValueError(f"could not find nodes {root_type.name}:{root_ids} (in {session!r})")
    roots = tuple(source_tree.get(str(id)) for id in root_ids)
    return wiring.unpack_nodes_inline(source_tree, parent=parent, session=session, roots=roots)


async def pg_read_node(
    session: Session,
    root_type: NodeType,
    root_id: UUID,
    options: ReadOptions,
    parent: Node | None = None,
) -> NodeT:
    """Reads a 'regular' node from the given PG database and unpacks it into the session."""
    roots = await pg_read_nodes(session, root_type, (root_id,), options, parent)
    if len(roots) != 1:
        raise ValueError(f"could not find root {root_type.name}:{root_id} (in {session!r})")
    return roots[0]


async def pg_search_nodes(
    session: Session,
    node_type: NodeType,
    *,
    options: ReadOptions,
    filter: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
    parent: Node | None = None,
) -> tuple[tuple[NodeT, ...], list[str] | tuple[str, ...], str | None]:
    """Searches 'regular' nodes from the given PG database and unpacks them into the session."""
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    cur = session.local_pg_cursor if node_cls.__is_local__ else session.global_pg_cursor
    roots, tree = await pg_search_nodes_data_tree(
        cur=cur,
        node_type=node_type,
        options=options,
        filter=filter,
        sort=sort,
        first=first,
        skip=skip,
        after=after,
    )
    if not tree:
        return (), (), None
    nodes = wiring.unpack_nodes_inline(tree, parent=parent, session=session, roots=roots.nodes)
    return nodes, roots.cursors, roots.start_cursor


# TODO @Performance: use psycopg3 pipelining to batch edits?


async def pg_write_regular_edits(
    cur: psycopg.AsyncCursor,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
    select_properties_by_type: dict[NodeType, tuple[Property, ...]]
    | None = DEFAULT_SELECTED_PROPERTIES,
) -> list["AnyNodeData"] | None:
    """Writes 'regular' edits to nodes (that aren't stored specially like records)."""
    if not edits:
        return None

    # batch operations by edit kind and node type
    current_updated_properties: list[int] = list(edits[0].properties or ())
    current_batch: list[EditData] = []
    all_returned_nodes: list[AnyNodeData] = [] if return_nodes else None
    for i in range(len(edits)):
        edit = edits[i]
        next_edit = edits[i + 1] if i + 1 < len(edits) else None
        current_batch.append(edit)

        # collect edited properties
        if edit.properties is not None:
            for prop in edit.properties:
                if prop not in current_updated_properties:
                    current_updated_properties.append(prop)

        # new op or end, flush batch
        if (
            next_edit is None
            or edit.type != next_edit.type
            or edit.node_type != next_edit.node_type
        ):
            edit_kind: EditType = wiring.unpack_enum(EditType, edit.type)
            node_type: NodeType = wiring.unpack_enum(NodeType, edit.node_type)
            batch_changed_nodes = await _pg_write_regular_edit_batch(
                cur=cur,
                edit_kind=edit_kind,
                node_type=node_type,
                batch=current_batch,
                return_nodes=return_nodes,
                updated_properties=current_updated_properties,
                selected_properties=select_properties_by_type[node_type],
            )
            if batch_changed_nodes:
                all_returned_nodes.extend(batch_changed_nodes)
            # start new batch
            current_updated_properties.clear()
            current_batch.clear()

    if return_nodes:
        return all_returned_nodes
    else:
        return None


async def _pg_write_regular_edit_batch(
    *,
    cur: psycopg.AsyncCursor,
    edit_kind: EditType,
    node_type: NodeType,
    batch: list[EditData],
    updated_properties: list[int] | tuple[int, ...] | None,  # across all edits
    return_nodes: bool,
    selected_properties: tuple[Property, ...],
) -> tuple["AnyNodeData", ...] | list["AnyNodeData"] | None:
    """Writes a batch of regular (not specially stored) node edits of the same kind."""

    node_cls = NODE_CLASS_BY_TYPE[node_type]
    table = node_cls.__table__
    if return_nodes:
        selected_columns = tuple(prop.column for prop in selected_properties)
    else:
        selected_columns = None

    if edit_kind == EditType.CREATE:
        nodes = tuple(wiring.unwrap_some_node(edit.node) for edit in batch)
        rows = tuple(pg_pack_node_data_row(node) for node in nodes)
        _ = await pg_insert(cur=cur, table=table, rows=rows)
        if return_nodes:
            return nodes  # if we get here, the nodes are equivalent to the rows
        else:
            return None

    elif edit_kind == EditType.UPSERT:
        nodes = tuple(wiring.unwrap_some_node(edit.node) for edit in batch)
        rows = tuple(pg_pack_node_data_row(node) for node in nodes)
        rows = await pg_upsert(
            cur=cur,
            table=table,
            rows=rows,
            conflict_columns=(table._primary_key,),
            update_columns=tuple(c for c in table.columns if c != table._primary_key),
            update_values={"revision": sql.SQL(f"{table.name}.revision + 1")},
            returning=selected_columns if return_nodes else None,
        )
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    elif edit_kind in (EditType.UPDATE, EditType.MOVE):
        assert updated_properties, f"no updated properties for {edit_kind} {node_type} ({batch!r})"
        now = utcnow_with_tz()
        dynamic_values = []
        for edit in batch:
            node = wiring.unwrap_some_node(edit.node)
            row = {"id": node.id}
            for prop_id in updated_properties:
                prop = node_cls.__properties_by_id__[prop_id]
                if prop_id in edit.properties:  # this is pretty inefficient
                    value = getattr(node, prop.name)
                    value = _pack_struct_data_prop(prop, value, ignore_array=False)
                else:
                    value = sql.Identifier(prop.column.name)  # keep old value
                row[prop.name] = value
            dynamic_values.append(row)
        # and update cru
        static_values = {
            "revision": sql.SQL("revision + 1"),
            "updated_at": now,
            "last_edited_at": now,
        }
        rows = await pg_update_dynamic(
            cur=cur,
            table=table,
            static_values=static_values,
            dynamic_columns=tuple(prop.column for prop in node_cls.__stored_properties__.values()),
            dynamic_values=dynamic_values,
            returning=selected_columns if return_nodes else None,
        )
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    elif edit_kind in (
        EditType.SOFT_DELETE,
        EditType.RESTORE,
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
    ):
        nodes_ids = tuple(edit.node.id for edit in batch)
        now = utcnow_with_tz()
        if edit_kind == EditType.SOFT_DELETE:
            row = {"deleted_at": now}
        elif edit_kind == EditType.RESTORE:
            row = {"deleted_at": None}
        elif edit_kind == EditType.ARCHIVE:
            row = {"archived_at": now}
        elif edit_kind == EditType.UNARCHIVE:
            row = {"archived_at": None}
        where = SqlComparison(
            left=sql.Identifier("id"),
            op=PostgresConditionalOp.EQ,
            right=sql.SQL("ANY({})").format(sql.Literal(nodes_ids)),
        )
        rows = await pg_update_static(
            cur=cur,
            table=table,
            where=where,
            static_value=row,
            returning=selected_columns,
        )
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    elif edit_kind == EditType.DELETE:
        nodes_ids = tuple(edit.node.id for edit in batch)
        where = SqlComparison(
            left=sql.Identifier("id"),
            op=PostgresConditionalOp.EQ,
            right=sql.SQL("ANY({})").format(sql.Literal(nodes_ids)),
        )
        rows = await pg_delete(
            cur=cur,
            table=table,
            where=where,
            returning=selected_columns,
        )
        if return_nodes:
            return tuple(pg_unpack_node_data_row(node_cls, row) for row in rows)
        else:
            return None

    else:
        raise ValueError(f"unexpected edit kind {edit_kind} {node_type} for {batch!r}")


#
# Record API
#

MAX_RECORD_TOTAL_VALUE_SIZE = 256 * 1024  # 256 KiB
MAX_RECORD_FIELD_VALUE_SIZE = 32 * 1024  # 32 KiB


async def pg_write_record_edits(
    cur: psycopg.AsyncCursor,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
    all_databases_by_id: dict[UUID, Block] | None = None,
) -> list["AnyNodeData"] | None:
    """
    Writes record edits to the given PG database. Unlike regular edits, records are in materialized tables.
    Pass in databases for all blocks (including deleted ones).
    """
    if not edits:
        return None

    # batch operations by edit kind and database
    current_updated_properties: list[int] = list(edits[0].properties or ())
    current_batch: list[EditData] = []
    all_returned_nodes: list[AnyNodeData] = [] if return_nodes else None
    for i in range(len(edits)):
        edit = edits[i]
        next_edit = edits[i + 1] if i + 1 < len(edits) else None
        current_batch.append(edit)

        # collect edited properties
        if edit.properties is not None:
            for prop in edit.properties:
                if prop not in current_updated_properties:
                    current_updated_properties.append(prop)

        # new op or end, flush current batch
        if (
            next_edit is None
            or edit.type != next_edit.type
            or edit.node.parent_id != next_edit.node.parent_id
        ):
            database = all_databases_by_id[edit.node.parent_id]
            batch_changed_nodes = await _pg_write_record_edit_batch(
                cur=cur,
                edit_kind=wiring.unpack_enum(EditType, edit.type),
                database=database,
                batch=current_batch,
                return_nodes=return_nodes,
            )
            if batch_changed_nodes:
                all_returned_nodes.extend(batch_changed_nodes)
            # start new batch
            current_updated_properties.clear()
            current_batch.clear()

    if return_nodes:
        return all_returned_nodes
    else:
        return None


async def _pg_write_record_edit_batch(
    *,
    cur: psycopg.AsyncCursor,
    database: Block,
    edit_kind: EditType,
    batch: list[EditData],
    updated_properties: list[int] | tuple[int, ...] | None,  # across all edits
    return_nodes: bool,
) -> tuple["AnyNodeData", ...] | list["AnyNodeData"] | None:
    """Writes a batch of record edits of the same kind."""

    table = database._table
    if edit_kind == EditType.CREATE:
        records = cast(tuple[wire.RecordData, ...], tuple(edit.node for edit in batch))
        rows = tuple(pg_pack_record_data_row(database, record) for record in records)
        _ = await pg_insert(cur=cur, table=table, rows=rows)
        if return_nodes:
            return records  # if we get here, the records are equivalent to the rows
        else:
            return None

    elif edit_kind == EditType.UPSERT:
        records = cast(tuple[wire.RecordData, ...], tuple(edit.node for edit in batch))
        rows = tuple(pg_pack_record_data_row(database, record) for record in records)
        rows = await pg_upsert(
            cur=cur,
            table=table,
            rows=rows,
            conflict_columns=(table._primary_key,),
            update_columns=tuple(c for c in table.columns if c != table._primary_key),
            update_values={"revision": sql.SQL(f"{table.name}.revision + 1")},
            returning=table.columns if return_nodes else None,
        )
        if return_nodes:
            return tuple(pg_unpack_record_data_row(database, row) for row in rows)
        else:
            return None

    elif edit_kind in (EditType.UPDATE, EditType.MOVE):
        updated_non_value_columns: tuple[str, ...] = tuple(
            Record.__properties_name_by_id__[p] for p in updated_properties if p != Record.value.id
        )
        # expand value properties for materialized tables
        if database.is_materialized and Record.value.id in updated_properties:
            all_value_columns = tuple(c.name for c in table.columns if c.name.startswith("value_"))
            updated_columns: tuple[str, ...] = updated_non_value_columns + all_value_columns
        else:
            updated_columns: tuple[str, ...] = tuple(
                Record.__properties_name_by_id__[p] for p in updated_properties
            )
        now = utcnow_with_tz()
        dynamic_values = []
        for edit in batch:
            record = cast(wire.RecordData, edit.node)
            row = {"id": record.id}
            for column_name in updated_non_value_columns:
                row[column_name] = getattr(record, column_name)
            # all 'value' fields are considered changed
            for field in database.fields:
                column_name = get_field_column_name(field)
                value = record.value.get(field.storage_key)
                row[column_name] = pg_wrap_record_field_value(record, field, value)
            dynamic_values.append(row)
        # and update cru
        static_values = {
            "revision": sql.SQL("revision + 1"),
            "updated_at": now,
            "last_edited_at": now,
        }
        rows = await pg_update_dynamic(
            cur=cur,
            table=table,
            static_values=static_values,
            dynamic_columns=tuple(table._columns_by_name[k] for k in updated_columns),
            dynamic_values=dynamic_values,
            returning=table.columns if return_nodes else None,
        )
        if return_nodes:
            return tuple(pg_unpack_record_data_row(database, row) for row in rows)
        else:
            return None

    elif edit_kind in (
        EditType.SOFT_DELETE,
        EditType.RESTORE,
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
    ):
        records_ids = tuple(edit.node.id for edit in batch)
        now = utcnow_with_tz()
        if edit_kind == EditType.SOFT_DELETE:
            row = {"deleted_at": now}
        elif edit_kind == EditType.RESTORE:
            row = {"deleted_at": None}
        elif edit_kind == EditType.ARCHIVE:
            row = {"archived_at": now}
        elif edit_kind == EditType.UNARCHIVE:
            row = {"archived_at": None}
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
        if return_nodes:
            return tuple(pg_unpack_record_data_row(database, row) for row in rows)
        else:
            return None

    elif edit_kind == EditType.DELETE:
        records_ids = tuple(edit.node.id for edit in batch)
        where = SqlComparison(
            sql.Identifier("id"),
            PostgresConditionalOp.EQ,
            sql.SQL("ANY({})").format(sql.Literal(records_ids)),
        )
        rows = await pg_delete(cur=cur, table=table, where=where, returning=table.columns)
        if return_nodes:
            return tuple(pg_unpack_record_data_row(database, row) for row in rows)
        else:
            return None

    else:
        raise RuntimeError(f"unexpected edit kind: {edit_kind} for {batch!r}")


def pg_pack_record_data_row(database: Block, record: wire.RecordData) -> RowIn:
    """Packs a record into a row for Postgres (flattened for ephemeral)."""
    # check size
    record_len_bytes = len(record)
    if record_len_bytes > MAX_RECORD_TOTAL_VALUE_SIZE:
        raise ValueError(
            f"record {record.id} is too large: {record_len_bytes} > {MAX_RECORD_TOTAL_VALUE_SIZE} bytes (consider storing large values in a File instead)"
        )

    # pack it up
    row = {
        "id": record.id,
        "ck": record.ck,
        "created_at": record.created_at,
        "updated_at": record.updated_at,
        "deleted_at": record.deleted_at,
        "last_edited_at": record.last_edited_at,
        "revision": record.revision,
        "block_key": database.dynamic_key,
    }
    if database.ephemeral:
        row["block_ck"] = database.ck
        row["block_id"] = database.id
        row["value"] = Jsonb(record.value)
    else:
        for field in database.fields:
            column_name = get_field_column_name(field)
            value = record.value.get(field.storage_key)
            row[column_name] = pg_wrap_record_field_value(record, field, value)
    assert len(row) == len(
        database._table.columns
    ), f"row mismatch: {row.keys()} for {database._table!r}"
    return row


def pg_wrap_record_field_value(
    record: Optional[wire.RecordData], field: "Field", value: Any
) -> Any:
    # see https://www.psycopg.org/psycopg3/docs/basic/adapt.html
    if value is None:
        return None
    record_len_bytes = len(record)
    if record_len_bytes > MAX_RECORD_FIELD_VALUE_SIZE:
        record_str = f"record {record.id}" if record else "record"
        value_str = repr(value)
        if len(value_str) > 256:
            value_str = value_str[:196] + "..." + value_str[-56:]
        raise ValueError(
            f"{record_str} field value '{field.py_ident}' is too large: {record_len_bytes} > {MAX_RECORD_FIELD_VALUE_SIZE} bytes (consider storing large values in a File instead)\nValue (truncated): {value_str}"
        )
    if field._storage_format == PrimitiveType.JSON:
        return Jsonb(value)
    elif field._storage_format == PrimitiveType.VECTOR:
        if isinstance(value, bytes):
            return value
        elif not isinstance(value, list):
            raise ValueError(f"unexpected vector value: {value}")
        else:
            # turn [-128, 127] into bytea
            return bytes(v + 128 for v in value)
    else:
        return value


def pg_unwrap_record_field_value(field: "Field", value: Any) -> Any:
    # see https://www.psycopg.org/psycopg3/docs/basic/adapt.html
    if value is None:
        return None
    elif field._storage_format == PrimitiveType.JSON:
        return value
    elif field._storage_format == PrimitiveType.VECTOR:
        # turn bytea into [-128, 127]
        return [v - 128 for v in value]
    else:
        return value


def pg_wrap_record_value(database: Block, value_packed: dict) -> dict:
    assert isinstance(value_packed, dict), f"record value not a dict: {value_packed}"
    if TYPE_DISCRIMINATOR_KEY in value_packed:  # not stored in database (implicit in block_key)
        del value_packed[TYPE_DISCRIMINATOR_KEY]
    if database.ephemeral:  # lift into generic 'value_packed' JSONB column
        value_packed = {
            "value_packed": sql.SQL("value_packed || {}").format(sql.Literal(Jsonb(value_packed)))
        }
    else:  # remap typed keys to column names
        value_columnized = {}
        for field in database.fields:
            v = value_packed.get(field.storage_key, UNSET)
            if v is not UNSET:
                column_name = get_field_column_name(field)
                value_columnized[column_name] = pg_wrap_record_field_value(None, field, v)
        value_packed = value_columnized
    return value_packed


def pg_unpack_record_data_row(database: Block, row: RowOut) -> wire.RecordData:
    if database.ephemeral:
        value_packed = row["value_packed"]
    else:
        value_packed = {
            f.storage_key: pg_unwrap_record_field_value(f, row[get_field_column_name(f)])
            for f in database.fields
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
        value=value_packed,
    )


PgSelectRecordsDataResult = NamedTuple(
    "PgSelectRecordsResult",
    [("records", list[wire.RecordData]), ("cursors", list[str]), ("start_cursor", str | None)],
)


async def pg_select_records_data(
    cur: psycopg.AsyncCursor,
    database: Block,
    *,
    where: Expression | None = None,
    sort: list[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> PgSelectRecordsDataResult:
    """Executes a select query on the given database."""
    if after:
        skip = (skip or 0) + int(decode_pg_cursor(after)) + 1  # 'after' is exclusive
    where = compile_pg_conditional(database, where) if where is not None else None
    sort = compile_pg_sorts(database, sort) if sort is not None else None
    rows = await pg_select(
        cur=cur, table=database._table, where=where, order_by=sort, first=first, skip=skip
    )
    records_data = [pg_unpack_record_data_row(database, row) for row in rows]
    cursors = [encode_pg_cursor(i) for i in range(skip or 0, (skip or 0) + len(records_data))]
    assert len(records_data) == len(cursors), f"unexpected cursors: {cursors} for {records_data}"
    return PgSelectRecordsDataResult(records_data, cursors, after)


@cachetools.cached({})
def encode_pg_cursor(i: int, exclusive: bool = True) -> str:
    if not exclusive:
        i -= 1  # include current element
    return base64.b64encode(struct.pack("q", i)).decode("ascii")


@cachetools.cached({})
def decode_pg_cursor(s: str) -> int:
    return struct.unpack("q", base64.b64decode(s))[0]


async def pg_duplicate_records(
    source_cur: psycopg.AsyncCursor,
    source_database: Block,
    target_cur: psycopg.AsyncCursor,
    target_database: Block,
    *,
    keep_cks: bool,
    where: Expression,
    copy_revisions: bool,
    return_nodes: bool = False,
) -> list[wire.RecordData] | None:
    """Duplicates records across databases."""
    source_table = source_database._table
    target_table = target_database._table
    if source_database.ephemeral or target_database.ephemeral:
        raise ValueError(f"cannot duplicate ephemeral: {source_database!r}->{target_database!r}")
    if not target_table.columns_include(source_table):
        raise ValueError(f"target {target_table!r} is not a superset of source {source_table!r}")
    target_package_id = target_database.package.id

    # TODO @Performance: duplicate records within same database directly in postgres
    where = where & C(
        ConditionalOp.EQUALS, field_key="block_key", value=source_database.dynamic_key
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
            record_row["id"] = get_node_id(target_package_id, ck=record_row["ck"])
            record_row["block_key"] = target_database.dynamic_key
            if not copy_revisions:
                record_row["revision"] = 0
        await pg_insert(cur=target_cur, table=target_table, rows=record_rows)
    log.debug("pg.duplicate_records.done", rows=len(record_rows))

    if return_nodes:
        return [pg_unpack_record_data_row(target_database, row) for row in record_rows]
    else:
        return None


if IS_DEBUG or IS_LOCAL or IS_TEST:
    # pretty print sql blocks in dev mode
    def sql_to_str(cur: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        import sqlparse

        s_str = s.as_string(cur)
        return "\n" + sqlparse.format(s_str, reindent=True, keyword_case="upper") + "\n"

else:

    def sql_to_str(cur: psycopg.Cursor | psycopg.AsyncCursor, s: sql.Composable) -> str:
        return s.as_string(cur)


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
        await cur.execute("SELECT 1 FROM pg_roles WHERE rolname = %s", (UNIVERSAL_RO_USERNAME,))
        exists = bool(await cur.fetchone())
        if is_public:
            if not exists:
                log.info("pg.create_db.create_global_ro.create")
                await cur.execute(
                    sql.SQL("CREATE USER {} WITH PASSWORD {}").format(
                        sql.Identifier(UNIVERSAL_RO_USERNAME), sql.Literal(UNIVERSAL_RO_PASSWORD)
                    ),
                )
            # grant read only
            log.info("pg.create_db.create_global_ro.grant")
            await cur.execute(
                sql.SQL("GRANT SELECT ON ALL TABLES IN SCHEMA public TO {}").format(
                    sql.Identifier(UNIVERSAL_RO_USERNAME),
                )
            )
        elif exists:
            log.info("pg.create_db.create_global_ro.remove")
            await cur.execute(
                sql.SQL("REVOKE ALL ON SCHEMA PUBLIC FROM {}").format(
                    sql.Identifier(UNIVERSAL_RO_USERNAME)
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


TABLE_BY_NODE_TYPE: dict[NodeType, Table] = {
    # read previously generated tables in schema.py
    node_type: getattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
    for node_type in NODE_TYPES
    if hasattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
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

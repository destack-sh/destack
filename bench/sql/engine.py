import base64
import enum
import struct
import typing
from collections import defaultdict
from dataclasses import dataclass
from itertools import chain
from typing import Any, Collection, Mapping, Optional, Sequence, cast
from uuid import UUID, uuid4

import cachetools
import psycopg
import structlog
from psycopg import sql
from psycopg.types.json import Jsonb

from bench.language import Block, ConditionalOp, Field, Package, QueryEngine, Session, TypeInfo
from bench.language.const import NODE_TYPES, EditKind, NodeType, SortOp, to_bench_metatype
from bench.language.database import HasDatabase
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
from bench.proto.wire import AnyNodeData, EditData, NodeReferenceData
from bench.proto.wiring import PROTO_CLASS_BY_TYPE
from bench.sql import schema
from bench.sql.client import UNIVERSAL_RO_PASSWORD, UNIVERSAL_RO_USERNAME, async_pg_cursor
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
from bench.utils.utils import DEBUG, LOCAL_ENV

logger = structlog.get_logger(__name__)

CAST_TYPE_BY_STORAGE_FORMAT: dict[PrimitiveType, str] = {
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
            _source=prop.id,
            name=prop.name,
            type=prop.primitive_type,
            is_array=prop.is_array,
            is_nullable=not prop.is_required,
            is_encrypted=prop.is_encrypted,
            is_primary_key=prop.name == "id",
            is_unique=prop.is_unique,
        )
        # default
        if prop.default is not UNSET and prop.default is not None:
            if isinstance(prop.default, enum.Enum) and not isinstance(
                prop.default, (enum.IntEnum, enum.IntFlag)
            ):
                column.default = f"'{prop.default.value}'::character varying"
            elif isinstance(prop.default, bool):
                column.default = "false" if prop.default is False else "true"
            elif isinstance(prop.default, int):
                column.default = str(prop.default)
            elif isinstance(prop.default, str):
                column.default = f"'{prop.default}'::character varying"
            else:
                raise TypeError(f"unexpected default in {prop!r}: {prop.default!r}")
        # is_encrypted
        if prop.is_encrypted:
            column.type = PrimitiveType.BYTES  # all encrypted columns are bytes
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
        )
        if type.is_secret:
            column.type = PrimitiveType.BYTES  # all encrypted columns are bytes
            column.is_encrypted = True

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


class SqlError(Exception):
    pass


class SqlUndefinedObject(SqlError):
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
    node: typing.Union[type[Node], "HasDatabase"],
    expr: Expression,
) -> SqlNode:
    if expr.property_ptr is not None:
        return sql.Identifier(expr._stored_property_resolved.name)
    elif expr.field is not None:
        assert expr.field._reflected_from is None, f"cannot use reflected: {expr!r}->{expr.field!r}"
        if isinstance(node, Block) and node.ephemeral:
            return SqlJsonPath(sql.Identifier("value"), [expr.field.storage_key])
        else:
            return sql.Identifier(get_field_column_name(expr.field))
    else:
        raise TypeError(f"unexpected expression ref: {expr!r}")


def compile_pg_conditional(
    node: typing.Union[type[Node], "HasDatabase"],
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
            pg_type = CAST_TYPE_BY_STORAGE_FORMAT[cond.field._storage_format]
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
    database: "HasDatabase",
    sort: Expression,
) -> SqlNode:
    field_ref = _compile_expression_ref(database, sort)
    return sql.SQL("{} {}").format(
        sql_node_to_sql(field_ref), sql.SQL(POSTGRES_SORT_OP_BY_BENCH[sort.op])
    )


def compile_pg_sorts(
    database: "HasDatabase",
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


def _wrap_pg_error(resource: Any, e: psycopg.errors.Error) -> Exception:
    if isinstance(e, (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn)):
        wrapped_t = SqlUndefinedObject
    else:
        wrapped_t = SqlError
    e_str = str(e)
    if "\n" in e_str:
        message = f"{e}\nin {resource!r}"
    else:
        message = f"{e} in {resource!r}"
    return wrapped_t(message)


async def pg_select_raw(cur: psycopg.AsyncCursor, query: sql.Composable) -> list[dict[str, any]]:
    logger.debug("pg.select_raw", query=sql_to_str(cur, query))
    await cur.execute(query)
    return await cur.fetchall()


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
    block = pg_select_sql(
        table=table,
        columns=columns,
        joins=joins,
        where=where,
        order_by=order_by,
        first=first,
        skip=skip,
    )
    logger.debug("pg.select_rows", table=table, query=sql_to_str(cur, block))
    try:
        await cur.execute(block, params)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
    return await cur.fetchall()


def pg_select_sql(
    table: Table,
    columns: list[Column],
    joins: list[SqlJoin] | None = None,
    where: SqlNode | None = None,
    order_by: SqlNode | None = None,
    first: int | None = None,
    skip: int | None = None,
):
    block = sql.SQL("SELECT {fields} FROM {table}").format(
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in columns),
        table=sql.Identifier(table.name),
    )
    if joins:
        block += sql.SQL(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        block += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if order_by:
        block += sql.SQL(" ORDER BY {}").format(sql_node_to_sql(order_by))
    if first:
        block += sql.SQL(" LIMIT {}").format(sql.Literal(first))
    if skip:
        block += sql.SQL(" OFFSET {}").format(sql.Literal(skip))
    return block


async def pg_count(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
) -> int:
    """Counts rows matching the given query."""
    block = sql.SQL("SELECT COUNT(*) FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        block += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    logger.debug("pg.count_rows", table=table, query=sql_to_str(cur, block))
    try:
        await cur.execute(block)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
    return (await cur.fetchone())["count"]


async def pg_exists(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    where: SqlNode | None = None,
    joins: list[SqlJoin] | None = None,
) -> bool:
    """Checks if rows matching the given query exist."""
    block = sql.SQL("SELECT EXISTS (SELECT 1 FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if joins:
        block += sql.SQL(" ").join(sql_node_to_sql(j) for j in joins)
    if where:
        block += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    block += sql.SQL(")")
    logger.debug("pg.exists_rows", table=table, query=sql_to_str(cur, block))
    try:
        await cur.execute(block)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
    return (await cur.fetchone())["exists"]


async def pg_insert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Inserts into the given table."""
    block = sql.SQL("INSERT INTO {table} ({fields}) VALUES ({values})").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ".join(["%s"] * len(table.columns))),
    )
    if returning:
        block += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.insert_rows", table=table, query=sql_to_str(cur, block))
    values = [tuple(row.get(c.name) for c in table.columns) for row in rows]
    try:
        await cur.executemany(block, values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
    if returning:
        return await cur.fetchall()


async def pg_upsert(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    conflict_columns: list[Column] | None = None,
    update_columns: list[Column] | None = None,
    returning: Collection[Column] | None = None,
) -> list[RowOut] | None:
    """Upserts into the given table."""
    if conflict_columns is None:
        conflict_columns = [table._primary_key]
    if update_columns is None:
        update_columns = [c for c in table.columns if c not in conflict_columns]
    block = sql.SQL(
        "INSERT INTO {table} ({fields}) VALUES ({values}) ON CONFLICT ({conflict}) DO UPDATE SET {updates}"
    ).format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ".join(["%s"] * len(table.columns))),
        conflict=sql.SQL(", ").join(sql.Identifier(c.name) for c in conflict_columns),
        updates=sql.SQL(", ").join(
            sql.SQL("{} = EXCLUDED.{}").format(sql.Identifier(c.name), sql.Identifier(c.name))
            for c in update_columns
        ),
    )
    if returning:
        block += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.insert_rows", table=table, query=sql_to_str(cur, block))
    values = [tuple(row.get(c.name) for c in table.columns) for row in rows]
    try:
        await cur.executemany(block, values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
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
    block = sql.SQL("UPDATE {table} SET {values}").format(
        table=sql.Identifier(table.name),
        values=sql.SQL(", ").join(
            sql.SQL("{} = {}").format(sql.Identifier(k), sql_node_to_sql(v))
            for k, v in static_value.items()
        ),
    )
    if where:
        block += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        block += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.update_rows.fixed", table=table, query=sql_to_str(cur, block))
    try:
        await cur.execute(block)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
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
    block = sql.SQL("UPDATE {table} SET {values} WHERE {pk} = %s").format(
        table=table_name, pk=sql.Identifier(table._primary_key.name), values=values_sql
    )
    if returning:
        block += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.SQL("{}").format(sql.Identifier(c.name)) for c in returning)
        )
    logger.debug(
        "pg.update_rows.list",
        table=table,
        query=sql_to_str(cur, block),
        rows=len(dynamic_values),
    )
    dynamic_values = [
        (*(value.get(c.name) for c in dynamic_columns), value.get(table._primary_key.name))
        for value in dynamic_values
    ]
    try:
        await cur.executemany(block, dynamic_values, returning=bool(returning))
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
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

    block = sql.SQL("DELETE FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        block += sql.SQL(" WHERE {}").format(sql_node_to_sql(where))
    if returning:
        block += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.delete_rows", table=table, query=sql_to_str(cur, block))
    try:
        await cur.execute(block)
    except psycopg.errors.Error as e:
        raise _wrap_pg_error(table, e) from e
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

NodeT = typing.TypeVar("NodeT", bound=Node)


def _pack_struct_data_prop(prop: Property, value: Any, ignore_array: bool) -> Any:
    if value is None:
        return None
    elif prop.is_array and not ignore_array:
        return [_pack_struct_data_prop(prop, v, ignore_array=True) for v in value]
    elif prop.is_struct:
        value = value.to_robust_dict(prop.struct_type)
        return wiring.pack_json_value(value)
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return wiring.unpack_json_value(value)
    elif prop.is_enum:
        return value.value
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
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return wiring.pack_json_value(value)
    elif prop.is_enum:
        return prop.py_type_stripped(value)
    else:
        return value


def pg_pack_node_data_row(node: AnyNodeData) -> dict[str, any]:
    """Packs a node's data into a row for the respective table."""
    node_cls = NODE_CLASS_BY_TYPE[to_bench_metatype(node.metatype)]
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
                    prop.reference_source.reference_wired_ptr.name
                )
                if ptr is not None and prop.reference_types[0] == ptr.type:
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
        data = proto_cls(metatype=node_cls.metatype)
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
                        ck=value,
                    )
                else:
                    ptr = NodeReferenceData(
                        metatype=wire.StructType.NODE_REFERENCE,
                        type=prop.reference_types[0],
                        id=value,
                    )
                assert prop.reference_source is not None, f"no reference source for {prop!r}"
                setattr(data, prop.reference_source.reference_wired_ptr.name, ptr)
        return data
    except (AttributeError, TypeError, ValueError, KeyError) as e:
        row_str = repr(row) if DEBUG else describe_type(row)
        raise ValueError(f"could not unpack row {node_cls.metatype.name}: {row_str}") from e


PgSelectNodesDataResult = typing.NamedTuple(
    "PgSelectNodesDataResult",
    [("nodes", list[wire.AnyNodeData]), ("cursors", list[str]), ("start_cursor", str | None)],
)


# nocheckin: handle colum encrypt/decrypt


async def pg_select_nodes_data(
    cur: psycopg.AsyncCursor,
    node_type: NodeType,
    *,
    properties: Collection[Property] | None = None,
    where: Expression | None = None,
    sort: Collection[Expression] | None = None,
    first: int | None = None,
    skip: int | None = None,
    after: str | None = None,
) -> PgSelectNodesDataResult:
    node_cls = NODE_CLASS_BY_TYPE[node_type]
    if after:
        skip = (skip or 0) + int(decode_pg_cursor(after)) + 1  # 'after' is exclusive
    columns = [prop.column for prop in properties] if properties is not None else None
    where = compile_pg_conditional(node_cls, where) if where is not None else None
    sort = compile_pg_sorts(node_cls, sort) if sort is not None else None
    rows = await pg_select(
        cur=cur,
        table=node_cls.__table__,
        columns=columns,
        where=where,
        order_by=sort,
        first=first,
        skip=skip,
    )
    nodes_data = [pg_unpack_node_data_row(node_cls, row) for row in rows]
    cursors = [encode_pg_cursor(i) for i in range(skip or 0, (skip or 0) + len(nodes_data))]
    assert len(nodes_data) == len(cursors), f"unexpected cursors: {cursors} for {nodes_data}"
    return PgSelectNodesDataResult(nodes_data, cursors, after)


DEFAULT_GLOBAL_FILTER: Expression = Node.filter(archived_at=None, deleted_at=None)._filter
DEFAULT_SELECTED_PROPERTIES: Mapping[NodeType, tuple[Property, ...]] = {
    node.metatype: tuple(
        prop for prop in node.__stored_properties__.values() if not prop.is_deferred
    )
    for node in NODE_CLASSES
}
ALL_SELECTED_PROPERTIES: Mapping[NodeType, tuple[Property, ...]] = {
    node.metatype: tuple(prop for prop in node.__stored_properties__.values())
    for node in NODE_CLASSES
}


async def pg_read_node_data_tree(
    cur: psycopg.AsyncCursor,
    root_type: NodeType,
    root_ids: tuple[UUID, ...],
    *,
    ancestor_types: tuple[NodeType, ...] | None = None,
    descendant_types: tuple[NodeType, ...] | None = None,
    global_filter: Expression = DEFAULT_GLOBAL_FILTER,
    select_properties_by_type: dict[NodeType, tuple[Property, ...]] = DEFAULT_SELECTED_PROPERTIES,
) -> NodeDataTree | None:
    """Reads 'regular' nodes from the given PG database. Returns an unordered list of all nodes."""
    visited_tree = NodeDataTree()

    # select "roots"
    roots = await pg_select_nodes_data(
        cur=cur,
        node_type=root_type,
        where=global_filter & Node.filter(id__in=root_ids)._filter,
        properties=select_properties_by_type[root_type],
    )
    if not roots.nodes:
        return None
    for node in roots.nodes:
        visited_tree.add(node)

    # select ancestors (recursively)
    # (basically, walk parent pointer if type is in ancestor_types)
    if ancestor_types:
        current_parents: list[wire.AnyNodeData] = roots.nodes
        to_select_by_type: dict[NodeType, list[str]] = defaultdict(list)
        while current_parents:
            to_select_by_type.clear()

            # traverse unseen parents to select next
            for node in current_parents:
                if (
                    node.parent_ptr is not None
                    and node.parent_ptr.type in ancestor_types
                    and node.parent_ptr.id not in visited_tree
                ):
                    to_select_by_type[node.parent_ptr.type].append(node.parent_ptr.id)

            # select next parents
            next_parents = []
            for node_type, node_ids in to_select_by_type.items():
                layer = await pg_select_nodes_data(
                    cur=cur,
                    node_type=node_type,
                    where=global_filter & Node.filter(id__in=node_ids)._filter,
                    select_properties_by_type=select_properties_by_type,
                )
                next_parents.extend(layer.nodes)
                for node in layer.nodes:
                    visited_tree.add(node)
            current_parents = next_parents

    # select descendants (recursively)
    if descendant_types:
        current_parents: list[wire.AnyNodeData] = roots.nodes
        while current_parents:
            next_parents: list[wire.AnyNodeData] = []
            # traverse all direct children of plausible types
            for child_type in descendant_types:
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
                        property_ptr=parent_property.ptr,
                        value=parents_by_type[parent_property.reference_types[0]],
                    )
                    parents_filters.append(filter)
                parent_filter = C(op=ConditionalOp.OR, clauses=parents_filters)

                # collect children
                # TODO @Performance!: recurse read node in SQL if child is parent of itself
                #  (also: we could likely take advantage of the ancestry graph to optimize this more)
                #  (maybe also for ancestors (same problem in reverse), but that's used much less)
                children = await pg_select_nodes_data(
                    cur=cur,
                    node_type=child_type,
                    where=global_filter & parent_filter,
                    properties=select_properties_by_type[child_type],
                )
                next_parents.extend(n for n in children.nodes if n.id not in visited_tree)
                for child in children.nodes:
                    visited_tree.add(child)

            current_parents = next_parents

    return visited_tree


async def pg_read_nodes(
    session: Session,
    root_type: NodeType,
    root_ids: tuple[UUID, ...],
    ancestor_types: tuple[NodeType, ...] | None = None,
    descendant_types: tuple[NodeType, ...] | None = None,
    global_filter: Expression = DEFAULT_GLOBAL_FILTER,
    select_properties_by_type: dict[NodeType, tuple[Property, ...]] = DEFAULT_SELECTED_PROPERTIES,
    parent: Node | None = None,
) -> tuple[NodeT, ...]:
    """Reads 'regular' nodes from the given PG database and unpacks them into the session. Returns the roots."""
    root_cls = NODE_CLASS_BY_TYPE[root_type]
    cur = session.local_pg_cursor if root_cls.__is_local__ else session.global_pg_cursor
    source_tree = await pg_read_node_data_tree(
        cur=cur,
        root_type=root_type,
        root_ids=root_ids,
        ancestor_types=ancestor_types,
        descendant_types=descendant_types,
        global_filter=global_filter,
        select_properties_by_type=select_properties_by_type,
    )
    if source_tree is None:
        raise ValueError(f"could not find nodes {root_type.name}:{root_ids} (in {session!r})")
    root = wiring.unpack_node_inline(source_tree, parent=parent, session=session)
    return tuple(root.lookup(id) for id in root_ids)


async def pg_read_node(
    session: Session,
    root_type: NodeType,
    root_id: UUID,
    ancestor_types: tuple[NodeType, ...] | None = None,
    descendant_types: tuple[NodeType, ...] | None = None,
    parent: Node | None = None,
) -> NodeT:
    """Reads a 'regular' node from the given PG database and unpacks it into the session."""
    roots = await pg_read_nodes(
        session,
        root_type=root_type,
        root_ids=(root_id,),
        ancestor_types=ancestor_types,
        descendant_types=descendant_types,
        parent=parent,
    )
    if len(roots) != 1:
        raise ValueError(f"could not find root {root_type.name}:{root_id} (in {session!r})")
    return roots[0]


async def pg_write_regular_edits(
    cur: psycopg.AsyncCursor,
    package: Package,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
    select_properties_by_type: dict[NodeType, tuple[Property, ...]] | None = None,
) -> list["AnyNodeData"] | None:
    """Writes 'regular' edits to nodes (that aren't stored specially like records)."""
    raise NotImplementedError("nocheckin: write_regular_edits_to_pg")


async def pg_write_record_edits(
    cur: psycopg.AsyncCursor,
    module: Package,
    edits: list[EditData],
    *,
    return_nodes: bool = False,
    old_databases_by_id: dict[UUID, "HasDatabase"] | None = None,
) -> list["AnyNodeData"] | None:
    """
    Writes record edits to the given PG database (unlike regular edits, these can act on materialized tables).
    Pass in databases for blocks that are no longer in the module (i.e. deleted record parent).
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
            rows = [pg_pack_record_data_row(database, record) for record in records]
            _ = await pg_insert(cur=cur, table=table, rows=rows)
            return records
        elif edit_kind in (EditKind.UPDATE, EditKind.MOVE):
            # not strictly correct (should be union of all in batch)
            properties = batch[0].properties
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
                for field in database.fields:  # all 'value' fields are considered changed
                    column_name = get_field_column_name(field)
                    value = record.value.get(field.storage_key)
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
                dynamic_columns=[table._columns_by_name[k] for k in properties],
                dynamic_values=row_values,
                returning=table.columns if return_nodes else None,
            )
            return (
                [pg_unpack_record_data_row(database, row) for row in rows] if return_nodes else []
            )
        elif edit_kind in (
            EditKind.SOFT_DELETE,
            EditKind.RESTORE,
            EditKind.ARCHIVE,
            EditKind.UNARCHIVE,
        ):
            records_ids = [edit.node.id for edit in batch]
            now = utcnow_with_tz()
            if edit_kind == EditKind.SOFT_DELETE:
                row = {"deleted_at": now}
            elif edit_kind == EditKind.RESTORE:
                row = {"deleted_at": None}
            elif edit_kind == EditKind.ARCHIVE:
                row = {"archived_at": now}
            elif edit_kind == EditKind.UNARCHIVE:
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
            return (
                [pg_unpack_record_data_row(database, row) for row in rows] if return_nodes else []
            )
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


#
# Record API
#

MAX_RECORD_TOTAL_VALUE_SIZE = 128 * 1024  # 128 KiB
MAX_RECORD_FIELD_VALUE_SIZE = 32 * 1024  # 32 KiB


def pg_pack_record_data_row(database: "HasDatabase", record: wire.RecordData) -> RowIn:
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
        "created_by_id": None,
        "updated_at": record.updated_at,
        "deleted_at": record.deleted_at,
        "last_edited_at": record.last_edited_at,
        "last_edited_by_id": None,
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


def pg_unwrap_record_field_value(database: "HasDatabase", field: "Field", value: Any) -> Any:
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


def pg_wrap_record_value(database: "HasDatabase", value_packed: dict) -> dict:
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
                value_columnized[column_name] = pg_wrap_record_field_value(database, None, field, v)
        value_packed = value_columnized
    return value_packed


def pg_unpack_record_data_row(database: "HasDatabase", row: RowOut) -> wire.RecordData:
    if database.ephemeral:
        value_packed = row["value_packed"]
    else:
        value_packed = {
            f.storage_key: pg_unwrap_record_field_value(database, f, row[get_field_column_name(f)])
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


PgSelectRecordsDataResult = typing.NamedTuple(
    "PgSelectRecordsResult",
    [("records", list[wire.RecordData]), ("cursors", list[str]), ("start_cursor", str | None)],
)


async def pg_select_records_data(
    cur: psycopg.AsyncCursor,
    database: "HasDatabase",
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
    source_database: "HasDatabase",
    target_cur: psycopg.AsyncCursor,
    target_database: "HasDatabase",
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
    target_module_id = target_database.module.id

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
            record_row["id"] = get_node_id(target_module_id, ck=record_row["ck"])
            record_row["block_key"] = target_database.dynamic_key
            if not copy_revisions:
                record_row["revision"] = 0
        await pg_insert(cur=target_cur, table=target_table, rows=record_rows)
    log.debug("pg.duplicate_records.done", rows=len(record_rows))

    if return_nodes:
        return [pg_unpack_record_data_row(target_database, row) for row in record_rows]
    else:
        return None


if DEBUG or LOCAL_ENV:
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

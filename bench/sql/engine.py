import enum
from dataclasses import dataclass
from uuid import UUID

import psycopg
import structlog
from psycopg import sql

from bench.language import ConditionalOp
from bench.sql.core import (
    CONSTRUCT_TABLE,
    Column,
    Constraint,
    Construct,
    ConstructInfo,
    ConstructKind,
    Index,
    SqlPrimitive,
    Table,
)

logger = structlog.get_logger(__name__)


class SqlException(Exception):
    pass


class UnknownConstruct(SqlException):
    pass


@dataclass(frozen=True)
class SqlExpression:
    def sql(self) -> sql.Composable:
        raise NotImplementedError


SqlNode = SqlExpression | SqlPrimitive | sql.SQL


# where SqlPrimitive is Union[str, int, float, bool, None, etc.]
def sql_node_to_sql(node: SqlNode) -> sql.Composable:
    if isinstance(node, SqlExpression):
        return node.sql()
    elif isinstance(node, SqlPrimitive):
        return sql.Literal(node)
    elif isinstance(node, sql.SQL):
        return node
    else:
        raise TypeError(f"unexpected node type: {node}")


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


POSTGRES_COMPARISON_OP_BY_BENCH_OP: dict[ConditionalOp, PostgresConditionalOp] = {
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
    # nocheckin: map in/contains properly (expression language doesn't differentiate)
}


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
        return sql.SQL("{} ({})").format(
            sql.SQL(self.op),
            sql_node_to_sql(self.operand),
        )


RowIn = dict[str, SqlPrimitive | SqlExpression]
RowOut = dict[str, SqlPrimitive]


async def _execute(cur: psycopg.AsyncCursor, query: sql.Composed) -> None:
    try:
        await cur.execute(query)
    except (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn) as e:
        raise UnknownConstruct(str(e)) from e


async def select_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    *,
    columns: list[Column] | None = None,
    where: SqlExpression | None = None,
    order_by: SqlExpression | None = None,
    first: int | None = None,
    skip: int | None = None,
) -> list[dict[str, any]]:
    """Selects from the given table."""
    columns = columns or table.columns
    query = sql.SQL("SELECT {fields} FROM {table}").format(
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in columns),
        table=sql.Identifier(table.name),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(where.sql())
    if order_by:
        query += sql.SQL(" ORDER BY {}").format(order_by.sql())
    if first:
        query += sql.SQL(" LIMIT {}").format(sql.Literal(first))
    if skip:
        query += sql.SQL(" OFFSET {}").format(sql.Literal(skip))
    logger.debug("pg.select_rows", table=table, query=query.as_string(cur))
    await _execute(cur, query)
    return await cur.fetchall()


async def insert_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Inserts into the given table."""
    query = sql.SQL("INSERT INTO {table} ({fields}) VALUES {values}").format(
        table=sql.Identifier(table.name),
        fields=sql.SQL(", ").join(sql.Identifier(c.name) for c in table.columns),
        values=sql.SQL(", ").join(
            sql.SQL("(")
            + sql.SQL(", ").join(sql_node_to_sql(v) for v in row.values())
            + sql.SQL(")")
            for row in rows
        ),
    )
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.insert_rows", table=table, query=query.as_string(cur))
    await _execute(cur, query)
    if returning:
        return await cur.fetchall()


async def update_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlExpression | None,
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
        query += sql.SQL(" WHERE {}").format(where.sql())
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.update_rows", table=table, query=query.as_string(cur))
    await _execute(cur, query)
    if returning:
        return await cur.fetchall()


async def delete_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlExpression | None,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Deletes from the given table."""

    query = sql.SQL("DELETE FROM {table}").format(
        table=sql.Identifier(table.name),
    )
    if where:
        query += sql.SQL(" WHERE {}").format(where.sql())
    if returning:
        query += sql.SQL(" RETURNING {}").format(
            sql.SQL(", ").join(sql.Identifier(c.name) for c in returning)
        )
    logger.debug("pg.delete_rows", table=table, query=query.as_string(cur))
    await _execute(cur, query)
    if returning:
        return await cur.fetchall()


async def truncate_rows(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Truncates the given table."""
    await cur.execute(sql.SQL("TRUNCATE TABLE {}").format(sql.Identifier(table.name)))


async def get_stored_pg_constructs(cur: psycopg.AsyncCursor) -> dict[UUID, ConstructInfo]:
    """Gets the current constructs in the given database (through the CONSTRUCT_TABLE)."""
    rows = await select_rows(cur, CONSTRUCT_TABLE)
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
    await truncate_rows(cur, CONSTRUCT_TABLE)
    rows = [
        {
            "id": construct.id,
            "kind": construct.kind.value,
            "name": construct.name,
            "hash": construct.hash,
        }
        for construct in constructs.values()
    ]
    await insert_rows(cur, CONSTRUCT_TABLE, rows)


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

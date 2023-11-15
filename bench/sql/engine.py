from dataclasses import dataclass
from typing import Any
from uuid import UUID

import psycopg
from psycopg import sql

from bench.sql.core import (
    CONSTRUCT_TABLE,
    Column,
    Constraint,
    Construct,
    ConstructInfo,
    Index,
    Table,
)


class SqlException(Exception):
    pass


class UnknownConstruct(SqlException):
    pass


@dataclass(frozen=True)
class SqlExpression:
    def sql(self) -> sql.SQL:
        raise NotImplementedError


@dataclass(frozen=True)
class Join(SqlExpression):
    pass


RowIn = dict[str, Any | SqlExpression]
RowOut = dict[str, Any]


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
    try:
        await cur.execute(query)
        return await cur.fetchall()
    except (psycopg.errors.UndefinedTable, psycopg.errors.UndefinedColumn) as e:
        raise UnknownConstruct(str(e)) from e


async def insert_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    rows: list[RowIn],
    *,
    returning: list[Column] | None = None,
) -> None:
    """Inserts into the given table."""
    raise NotImplementedError


async def update_rows(
    cur: psycopg.AsyncCursor,
    table: Table,
    where: SqlExpression | None,
    values: RowIn | list[RowIn],
    *,
    returning: list[Column] | None = None,
) -> list[RowOut] | None:
    """Updates the given table."""
    raise NotImplementedError


async def delete_rows(cur: psycopg.AsyncCursor, table: Table, where: SqlExpression | None) -> int:
    """Deletes from the given table."""
    raise NotImplementedError


async def get_pg_constructs(cur: psycopg.AsyncCursor) -> dict[UUID, ConstructInfo]:
    """Gets the current constructs in the given database (through the CONSTRUCT_TABLE)."""
    try:
        await select_rows(cur, CONSTRUCT_TABLE)
    except UnknownConstruct:
        pass  # first time
    raise NotImplementedError  # nocheckin fetch from CONSTRUCT_TABLE for current migration


async def create_pg_constructs(cur: psycopg.AsyncCursor, constructs: dict[UUID, Construct]):
    """Creates the given constructs in the given database (not an upsert!)."""
    for construct in constructs.values():
        if isinstance(construct, Table):
            await cur.execute(sql.SQL("CREATE TABLE {} ()").format(sql.Identifier(construct.name)))
        elif isinstance(construct, Column):
            await cur.execute(
                sql.SQL("ALTER TABLE {} ADD COLUMN {} {}").format(
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

from itertools import chain
from uuid import UUID

from psycopg import sql

import bench.language as lang
from bench.language import HasDatabase, Module
from bench.language.const import MNT, TypeStorageFormat
from bench.storage.client import async_pg_cursor
from bench.storage.core import (
    BASE_RECORD_TABLE,
    COMMON_TABLES,
    Column,
    ColumnType,
    Construct,
    ConstructInfo,
    ConstructKind,
    Table,
    get_record_table_name,
)

COLUMN_TYPE_BY_STORAGE_FORMAT: dict[TypeStorageFormat, ColumnType] = {
    TypeStorageFormat.STRING: ColumnType.STRING,
    TypeStorageFormat.DOUBLE: ColumnType.FLOAT,
    TypeStorageFormat.LONG: ColumnType.BIGINT,
    TypeStorageFormat.VECTOR: ColumnType.VECTOR,
    TypeStorageFormat.BINARY: ColumnType.BINARY,
    TypeStorageFormat.DATE: ColumnType.DATETIME,
    TypeStorageFormat.KEYWORD: ColumnType.STRING,
    TypeStorageFormat.OBJECT: ColumnType.JSON,
}


def map_to_column(field: lang.Field) -> Column:
    column_type = COLUMN_TYPE_BY_STORAGE_FORMAT[field.type.storage_format]
    is_array = (
        field.flags & lang.TypeFlag.IS_ARRAY or field.flags & lang.TypeFlag.IS_ARRAYABLE
    ) and column_type != ColumnType.JSON
    return Column(
        name=field._typed_key,
        type=column_type,
        is_array=is_array,
    )


def map_to_table(statement: lang.Statement) -> Table:
    columns = [map_to_column(f) for f in statement.resolved_fields]
    indexes = []
    constraints = []

    return Table(
        name=get_record_table_name(statement.ck),
        columns=(*(c.clone() for c in BASE_RECORD_TABLE.columns), *columns),
        indexes=(*(i.clone() for i in BASE_RECORD_TABLE.indexes), *indexes),
        constraints=(*(c.clone() for c in BASE_RECORD_TABLE.constraints), *constraints),
    )


async def get_pg_constructs(pg_name: str) -> dict[UUID, ConstructInfo]:
    raise NotImplementedError  # nocheckin fetch from CONSTRUCT_TABLE for current migration


async def update_pg_schema(pg_name: str, module: Module) -> None:
    """Updates Postgres tables (i.e. schema) for a module's databases."""
    databases: list[lang.Statement] = [
        s
        for s in module._nodes
        if s.mnt == MNT.STATEMENT and HasDatabase in s._components and not s.ephemeral
    ]
    tables = (*COMMON_TABLES, *(map_to_table(s) for s in databases))

    # get missing constructs (diff existing and current)
    existing_constructs = await get_pg_constructs(pg_name)
    current_constructs: dict[UUID, Construct] = {c.id: c for c in chain(t.walk() for t in tables)}
    missing_constructs = {
        id: c for id, c in current_constructs.items() if id not in existing_constructs
    }

    # create missing constructs
    async with async_pg_cursor(pg_name, autocommit=False) as cur:
        for construct in missing_constructs.values():
            if construct.kind == ConstructKind.TABLE:
                await cur.execute(
                    sql.SQL("CREATE TABLE {} ()").format(sql.Identifier(construct.name))
                )
            elif construct.kind == ConstructKind.COLUMN:
                await cur.execute(
                    sql.SQL("ALTER TABLE {} ADD COLUMN {} {}").format(
                        sql.Identifier(construct.table_name), sql.SQL(construct.sql())
                    )
                )
            elif construct.kind == ConstructKind.INDEX:
                await cur.execute(
                    sql.SQL("CREATE INDEX {} on {}").format(
                        sql.SQL(construct.sql()),
                        sql.Identifier(construct.table_name),
                    )
                )
            elif construct.kind == ConstructKind.CONSTRAINT:
                await cur.execute(
                    sql.SQL("ALTER TABLE {} ADD CONSTRAINT {}").format(
                        sql.Identifier(construct.table_name),
                        sql.SQL(construct.sql()),
                    )
                )
            else:
                raise RuntimeError(f"unexpected construct: {construct}")

from itertools import chain
from uuid import UUID

import bench.language as lang
from bench.language import HasDatabase, Module
from bench.language.const import MNT, TypeStorageFormat
from bench.sql.client import async_pg_cursor
from bench.sql.core import (
    BASE_RECORD_TABLE,
    INTERNAL_TABLES,
    Column,
    ColumnType,
    Construct,
    Table,
    get_record_table_name,
)
from bench.sql.engine import create_pg_constructs, get_pg_constructs

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
    """Gets a column from a field. Later, there may be more than one column per field (?)."""
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
    """Gets the full table with all specific fields of a database and general record stuff."""
    columns = [map_to_column(f) for f in statement.resolved_fields]
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
    databases: list[lang.Statement] = [
        s
        for s in module._nodes
        if s.mnt == MNT.STATEMENT and HasDatabase in s._components and not s.ephemeral
    ]
    tables = (*INTERNAL_TABLES, *(map_to_table(s) for s in databases))

    async with async_pg_cursor(pg_name, autocommit=False) as cur:
        # get missing constructs (diff existing and current)
        existing_constructs = await get_pg_constructs(cur)
        current_constructs: dict[UUID, Construct] = {
            c.id: c for c in chain(t.walk() for t in tables)
        }
        missing_constructs = {
            id: c for id, c in current_constructs.items() if id not in existing_constructs
        }
        # create missing constructs
        await create_pg_constructs(cur, missing_constructs)

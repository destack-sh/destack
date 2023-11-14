import psycopg

import bench.language as lang
from bench.language import HasDatabase, Module
from bench.language.const import MNT, TypeStorageFormat
from bench.storage.core import BASE_RECORD_TABLE, Column, ColumnType, Table

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
        name=f"record_{statement.key.replace('-', '')}",
        columns=(*BASE_RECORD_TABLE.columns, *columns),
        indexes=(*BASE_RECORD_TABLE.indexes, *indexes),
        constraints=(*BASE_RECORD_TABLE.constraints, *constraints),
    )


async def upsert_pg_table(cur: psycopg.AsyncCursor, table: Table) -> None:
    """Upserts a table schema into Postgres."""
    # create table
    await cur.execute(
        f"""
        CREATE TABLE IF NOT EXISTS {table.name} (
            {', '.join(c.sql() for c in table.columns)}
        )
        """
    )

    # create indexes
    for index in table.indexes:
        await cur.execute(
            f"""
            CREATE INDEX IF NOT EXISTS {index.name} ON {table.name} ({', '.join(index.columns)})
            """
        )

    # create constraints
    for constraint in table.constraints:
        await cur.execute(
            f"""
            ALTER TABLE {table.name} ADD CONSTRAINT {constraint.name} {constraint.sql()}
            """
        )


async def update_pg_schema(pg_name: str, module: Module) -> None:
    """Updates Postgres tables (i.e. schema) for a module's databases."""
    databases: list[lang.Statement] = [
        s for s in module._nodes if s.mnt == MNT.STATEMENT and HasDatabase in s._components
    ]
    [map_to_table(s) for s in databases]
    raise NotImplementedError  # nocheckin do it

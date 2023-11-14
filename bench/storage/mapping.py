import bench.language as lang
from bench.language import Module
from bench.language.const import TypeStorageFormat
from bench.storage.core import Column, ColumnType

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


def map_to_column_type(field: lang.Field) -> ColumnType:
    return COLUMN_TYPE_BY_STORAGE_FORMAT[field._storage_format]


def map_to_column(field: lang.Field) -> Column:
    raise NotImplementedError("DB storage is incomplete")


def update_pg_schema(pg_name: str, module: Module) -> None:
    """Updates Postgres tables (i.e. schema) for a module's databases."""
    raise NotImplementedError  # nocheckin do it

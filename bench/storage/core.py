import enum
from dataclasses import dataclass


class ColumnType(enum.StrEnum):
    """
    Generic SQL column types (akin to Prisma/SQLAlchemy).
    """

    STRING = "String"
    BOOLEAN = "Boolean"
    INT = "Int"  # range: -2147483648 to 2147483647
    BIGINT = "BigInt"  # range: -9223372036854775808 to 9223372036854775807
    FLOAT = "Float"
    DECIMAL = "Decimal"
    DATETIME = "DateTime"
    JSON = "Json"
    BINARY = "Binary"
    VECTOR = "Vector"
    UUID = "UUID"
    BYTES = "Bytes"


@dataclass
class Column:
    """
    A high-level SQL column definition.
    """

    name: str
    type: ColumnType
    is_array: bool = False
    is_primary_key: bool = False
    is_unique: bool = False
    is_nullable: bool = False
    default: str | None = None


class ConstraintType(enum.StrEnum):
    """
    A high-level SQL constraint type (we don't need real foreign keys).
    """

    PRIMARY_KEY = "PRIMARY KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


@dataclass
class Constraint:
    """
    A high-level SQL constraint.
    """

    type: ConstraintType
    columns: list[str]
    name: str
    condition: str | None = None


class IndexType(enum.StrEnum):
    """
    A high-level SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"


@dataclass
class Index:
    """
    A high-level SQL index.
    """

    type: IndexType
    columns: list[str]
    name: str
    condition: str | None = None


@dataclass
class Table:
    """
    A high-level SQL table.
    """

    name: str
    columns: list[Column]
    constraints: list[Constraint] = ()
    indexes: list[str] = ()


# template for actual record tables
BASE_RECORD_TABLE = Table(
    "base_record",
    columns=[
        Column("id", ColumnType.UUID, is_primary_key=True),
        Column("ck", ColumnType.UUID),
        Column("created_at", ColumnType.DATETIME),
        Column("updated_at", ColumnType.DATETIME),
        Column("deleted_at", ColumnType.DATETIME, is_nullable=True),
        Column("created_by_id", ColumnType.UUID, is_nullable=True),
        Column("last_edited_at", ColumnType.DATETIME),
        Column("last_edited_by_id", ColumnType.UUID, is_nullable=True),
        Column("revision", ColumnType.INT),
        Column("statement_key", ColumnType.UUID),
    ],
    constraints=[
        # ck + statement_key must be unique
        Constraint(
            ConstraintType.UNIQUE,
            columns=["ck", "statement_key"],
            name="unique_statement_key_ck",
        )
    ],
)

# 'hufflepuff' table for ephemeral 'tables' without actual tables
EPHEMERAL_RECORD_TABLE = Table(
    "ephemeral_record",
    columns=[
        *BASE_RECORD_TABLE.columns,
        Column("statement_id", ColumnType.UUID),
        Column("statement_ck", ColumnType.UUID),
        Column("value", ColumnType.JSON, is_nullable=True),
    ],
)


#
# Postgres engine
#


class PostgresColumnType(enum.StrEnum):
    """
    A PostgreSQL column type.
    """

    BIGINT = "bigint"
    BIGSERIAL = "bigserial"
    BIT = "bit"
    BIT_VARYING = "bit_varying"
    BOOLEAN = "boolean"
    BOX = "box"
    BYTEA = "bytea"
    CHARACTER = "character"
    CHARACTER_VARYING = "character_varying"
    CIDR = "cidr"
    CIRCLE = "circle"
    DATE = "date"
    DOUBLE_PRECISION = "double_precision"
    INET = "inet"
    INTEGER = "integer"
    INTERVAL = "interval"
    JSON = "json"
    JSONB = "jsonb"
    LINE = "line"
    LSEG = "lseg"
    MACADDR = "macaddr"
    MONEY = "money"
    NUMERIC = "numeric"
    PATH = "path"
    POINT = "point"
    POLYGON = "polygon"
    REAL = "real"
    SMALLINT = "smallint"
    SMALLSERIAL = "smallserial"
    SERIAL = "serial"
    TEXT = "text"
    TIME = "time"
    TIMESTAMP = "timestamp"
    UUID = "uuid"
    VECTOR = "vector"
    XML = "xml"


POSTGRES_TYPE_BY_GENERIC_TYPE = {
    ColumnType.STRING: PostgresColumnType.TEXT,
    ColumnType.BOOLEAN: PostgresColumnType.BOOLEAN,
    ColumnType.INT: PostgresColumnType.INTEGER,
    ColumnType.BIGINT: PostgresColumnType.BIGINT,
    ColumnType.FLOAT: PostgresColumnType.REAL,
    ColumnType.DECIMAL: PostgresColumnType.NUMERIC,
    ColumnType.DATETIME: PostgresColumnType.TIMESTAMP,
    ColumnType.JSON: PostgresColumnType.JSONB,
    ColumnType.BINARY: PostgresColumnType.BYTEA,
    ColumnType.VECTOR: PostgresColumnType.VECTOR,
    ColumnType.UUID: PostgresColumnType.UUID,
}

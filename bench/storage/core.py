import enum
from dataclasses import dataclass
from datetime import datetime


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

    def __str__(self):
        args_str = ", ".join(
            f"{name}={self.__dict__[name]}"
            for name in ("is_array", "is_primary_key", "is_unique", "is_nullable", "default")
        )
        return f"{self.name} ({self.type}) [{args_str}])"

    def __repr__(self):
        return f"<Column {self}>"


class ConstraintType(enum.StrEnum):
    """
    A high-level SQL constraint type (we don't need real foreign keys).
    """

    PRIMARY_KEY = "PRIMARY KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


dataclass(ceq=True, frozen=True)


class Constraint:
    """
    A high-level SQL constraint.
    """

    name: str
    type: ConstraintType
    columns: list[str]
    condition: str | None = None

    def __str__(self):
        return f"{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Constraint {self}>"


class IndexType(enum.StrEnum):
    """
    A high-level SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"


dataclass(ceq=True, frozen=True)


class Index:
    """
    A high-level SQL index.
    """

    name: str
    type: IndexType
    columns: list[str]
    condition: str | None = None

    def __str__(self):
        return f"{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"


dataclass(ceq=True, frozen=True)


class Table:
    """
    A high-level SQL table.
    """

    name: str
    columns: list[Column]
    constraints: list[Constraint] = ()
    indexes: list[str] = ()

    def __str__(self):
        return (
            f"{self.name} ({self.columns}, constraints={self.constraints}, indexes={self.indexes})"
        )

    def __repr__(self):
        return f"<Table {self}>"


# template for actual record tables
BASE_RECORD_TABLE = Table(
    "record_base",
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
    "record_ephemeral",
    columns=[
        *BASE_RECORD_TABLE.columns,
        Column("statement_id", ColumnType.UUID),
        Column("statement_ck", ColumnType.UUID),
        Column("value", ColumnType.JSON, is_nullable=True),
    ],
)

# for internal use only
MIGRATION_TABLE = Table(
    "_migration",
    columns=[
        Column("id", ColumnType.INT, is_primary_key=True),
        Column("applied_at", ColumnType.DATETIME),
        Column("runtime_version", ColumnType.STRING),
        Column("module_version", ColumnType.STRING),
        Column("hash", ColumnType.STRING),
    ],
)


@dataclass
class Migration:
    """
    A recorded SQL migration.
    """

    id: int
    version: str
    hash: str
    sql: str
    applied_at: datetime


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

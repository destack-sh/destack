import enum


class ColumnType(enum.StrEnum):
    """
    Generic SQL column types (akin to Prisma/SQLAlchemy).
    """

    STRING = "String"
    BOOLEAN = "Boolean"
    INT = "Int"
    BIGINT = "BigInt"
    FLOAT = "Float"
    DECIMAL = "Decimal"
    DATETIME = "DateTime"
    JSON = "Json"
    BINARY = "Binary"
    VECTOR = "Vector"
    UUID = "UUID"
    BYTES = "Bytes"


class ConstraintType(enum.StrEnum):
    """
    A SQL constraint type.
    """

    PRIMARY_KEY = "PRIMARY KEY"
    UNIQUE = "UNIQUE"
    FOREIGN_KEY = "FOREIGN KEY"
    CHECK = "CHECK"
    EXCLUDE = "EXCLUDE"


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

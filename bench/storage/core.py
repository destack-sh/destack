import enum
import hashlib
from dataclasses import dataclass, is_dataclass, replace
from datetime import datetime
from itertools import chain
from typing import ClassVar, Union
from uuid import UUID, uuid5

from bench.language.const import BENCH_UUID_NAMESPACE


def stable_hash(*args) -> int:
    """
    Hashes a tuple of arguments deterministically.
    """
    hasher = hashlib.sha256()

    def update_hash(value):
        if is_dataclass(value):
            hasher.update(str(value.__hash__()).encode())
        elif isinstance(value, list):
            for item in value:
                update_hash(item)
        else:
            hasher.update(str(value).encode())

    for arg in args:
        update_hash(arg)

    return int(hasher.hexdigest(), 16)


class ConstructKind(enum.StrEnum):
    TABLE = "TABLE"
    COLUMN = "COLUMN"
    CONSTRAINT = "CONSTRAINT"
    INDEX = "INDEX"


@dataclass
class Construct:
    kind: ClassVar[ConstructKind]
    name: str

    def sql(self) -> str:
        raise NotImplementedError

    def walk(self) -> tuple["Construct", ...]:
        return (self,)

    def __hash__(self):
        """Computes a stable hash of this construct and any child constructs."""
        raise NotImplementedError

    @property
    def id(self):
        return uuid5(BENCH_UUID_NAMESPACE, f"{self.kind.value}:{self.name}")


@dataclass
class TableConstruct(Construct):
    @property
    def _table(self) -> Union["Table", None]:
        raise NotImplementedError

    @property
    def id(self) -> UUID:
        return uuid5(BENCH_UUID_NAMESPACE, f"{self.kind.value}:{self._table.name}.{self.name}")

    def clone(self):
        """Deep copy this table construct without the table reference."""
        return replace(self, _table=None)


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
class Column(TableConstruct):
    """
    A high-level SQL column definition.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.COLUMN

    name: str
    type: ColumnType
    is_array: bool = False
    is_primary_key: bool = False
    is_unique: bool = False
    is_nullable: bool = False
    default: str | None = None
    _table: Union["Table", None] = None

    def __str__(self):
        args_str = ", ".join(
            f"{name}={self.__dict__[name]}"
            for name in ("is_array", "is_primary_key", "is_unique", "is_nullable", "default")
        )
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{args_str}])"

    def __repr__(self):
        return f"<Column {self}>"

    def __hash__(self):
        return stable_hash(
            self.kind,
            self.name,
            self.type,
            self.is_array,
            self.is_primary_key,
            self.is_unique,
            self.is_nullable,
            self.default,
        )

    def __eq__(self, other):
        return hash(self) == hash(other)

    def sql(self) -> str:
        pg_type = POSTGRES_TYPE_BY_GENERIC_TYPE[self.type]
        if self.is_array:
            pg_type += "[]"
        parts = [self.name, pg_type]
        if self.is_primary_key:
            parts.append("PRIMARY KEY")
        if self.is_unique:
            parts.append("UNIQUE")
        if not self.is_nullable:
            parts.append("NOT NULL")
        if self.default is not None:
            parts.append(f"DEFAULT {self.default}")
        return " ".join(parts)


class ConstraintType(enum.StrEnum):
    """
    A high-level SQL constraint type (we don't need real foreign keys).
    """

    PRIMARY_KEY = "PRIMARY KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


@dataclass
class Constraint(Construct):
    """
    A high-level SQL constraint.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.CONSTRAINT

    name: str
    type: ConstraintType
    columns: list[str] | None = None
    condition: str | None = None
    _table: Union["Table", None] = None

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Constraint {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name, self.type, self.columns, self.condition)

    def __eq__(self, other):
        return hash(self) == hash(other)

    def sql(self) -> str:
        parts = [self.name, self.type]
        if self.type == ConstraintType.CHECK:
            parts.append(f"({self.condition})")
        elif self.type == ConstraintType.UNIQUE:
            parts.append(f"({', '.join(self.columns)})")
        return " ".join(parts)


class IndexType(enum.StrEnum):
    """
    A high-level SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"


@dataclass
class Index(Construct):
    """
    A high-level SQL index.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.INDEX

    name: str
    type: IndexType
    columns: list[str]
    condition: str | None = None
    _table: Union["Table", None] = None

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name, self.type, self.columns, self.condition)

    def __eq__(self, other):
        return hash(self) == hash(other)

    def sql(self) -> str:
        parts = [self.name, self.type, f"({', '.join(self.columns)})"]
        if self.condition is not None:
            parts.append(f"WHERE {self.condition}")
        return " ".join(parts)


@dataclass
class Table(Construct):
    """
    A high-level SQL table.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.TABLE

    name: str
    columns: tuple[Column, ...]
    constraints: tuple[Constraint, ...] = ()
    indexes: tuple[Index, ...] = ()

    def __post_init__(self):
        for construct in chain(self.columns, self.constraints, self.indexes):
            if construct._table is not None:
                raise ValueError(f"{construct} is already attached to {construct._table}")
            construct._table = self

    def __str__(self):
        return (
            f"{self.name} ({self.columns}, constraints={self.constraints}, indexes={self.indexes})"
        )

    def __repr__(self):
        return f"<Table {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name, self.columns, self.constraints, self.indexes)

    def __eq__(self, other):
        return hash(self) == hash(other)

    def walk(self) -> tuple[Construct, ...]:
        return self, *self.columns, *self.constraints, *self.indexes


# template for actual record tables
BASE_RECORD_TABLE = Table(
    "record_base",
    columns=(
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
    ),
    constraints=(
        # ck + statement_key must be unique
        Constraint(
            "unique_statement_key_ck", ConstraintType.UNIQUE, columns=["ck", "statement_key"]
        ),
    ),
)
# 'hufflepuff' table for ephemeral 'tables' without actual tables
EPHEMERAL_RECORD_TABLE = Table(
    "record_ephemeral",
    columns=(
        *(c.clone() for c in BASE_RECORD_TABLE.columns),
        Column("statement_ck", ColumnType.UUID),
        Column("statement_id", ColumnType.UUID),
        Column("value", ColumnType.JSON, is_nullable=True),
    ),
)


def get_record_table_name(statement_ck: UUID) -> str:
    return f"record_{str(statement_ck).replace('-', '')}"


# for internal use only
MIGRATION_TABLE = Table(
    "_migration",
    columns=(
        Column("id", ColumnType.INT, is_primary_key=True),
        Column("applied_at", ColumnType.DATETIME),
        Column("runtime_version", ColumnType.STRING),
        Column("module_version", ColumnType.STRING),
        Column("hash", ColumnType.INT),
    ),
)
CONSTRUCT_TABLE = Table(
    "_construct",
    columns=(
        Column("id", ColumnType.UUID, is_primary_key=True),
        Column("kind", ColumnType.STRING),
        Column("name", ColumnType.STRING),
        Column("hash", ColumnType.INT),
        Column("migration_id", ColumnType.INT),
    ),
)

COMMON_TABLES = (EPHEMERAL_RECORD_TABLE, MIGRATION_TABLE, CONSTRUCT_TABLE)


@dataclass
class ConstructInfo:
    """Reconstructed info for a construct."""

    id: int
    kind: ConstructKind
    name: str
    hash: int


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

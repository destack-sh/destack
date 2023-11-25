import enum
import hashlib
from dataclasses import dataclass, field, is_dataclass, replace
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, ClassVar, Union
from uuid import UUID, uuid5

import psycopg
from more_itertools import first

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
class ConstructInfo:
    """Reconstructed info for a construct."""

    id: UUID
    kind: ConstructKind
    table_name: str | None
    name: str
    hash: int


@dataclass
class Construct:
    kind: ClassVar[ConstructKind]

    if TYPE_CHECKING:
        name: str  # defined in subclasses as either property or field

    def sql(self) -> str:
        raise NotImplementedError

    def walk(self) -> tuple["Construct", ...]:
        return (self,)

    @property
    def hash(self) -> int:
        return hash(self)

    def __hash__(self):
        """Computes a stable hash of this construct and any child constructs."""
        raise NotImplementedError

    @property
    def id(self):
        return uuid5(BENCH_UUID_NAMESPACE, f"{self.kind.value}:{self.name}")


@dataclass
class TableConstruct(Construct):
    @property
    def table(self) -> "Table":
        assert self._table is not None, f"{self} is not attached to a table"
        return self._table

    @property
    def table_name(self) -> str:
        return self.table.name

    @property
    def _table(self) -> Union["Table", None]:
        raise NotImplementedError

    @property
    def id(self) -> UUID:
        return uuid5(BENCH_UUID_NAMESPACE, f"{self.kind.value}:{self._table.name}.{self.name}")

    def clone(self) -> "TableConstruct":
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


SqlPrimitiveSingle = Union[str, int, float, bool, datetime, UUID, bytes, type(None)]
SqlPrimitive = Union[SqlPrimitiveSingle, list[SqlPrimitiveSingle], dict[str, SqlPrimitiveSingle]]


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
    length: int | None = None
    default: str | None = None
    _table: Union["Table", None] = None

    def __str__(self):
        args_str = ", ".join(
            f"{name}={self.__dict__[name]}"
            for name in ("is_array", "is_primary_key", "is_unique", "is_nullable", "default")
            if self.__dict__[name]
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
        if self.type == ColumnType.STRING and self.length is not None:
            pg_type = f"VARCHAR({self.length})"
        else:
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
class Constraint(TableConstruct):
    """
    A high-level SQL constraint.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.CONSTRAINT

    inner_name: str
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

    @property
    def name(self):
        return f"{self.table_name}_{self.inner_name}"

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
class Index(TableConstruct):
    """
    A high-level SQL index.
    """

    kind: ClassVar[ConstructKind] = ConstructKind.INDEX

    inner_name: str
    type: IndexType
    columns: list[str]
    expression: str | None = None
    condition: str | None = None
    _table: Union["Table", None] = None

    def __post_init__(self):
        if self.expression is not None:
            # check that all columns are in the expression
            for column in self.columns:
                if column not in self.expression:
                    raise ValueError(f"{column} is not in {self.expression} (in {self!r})")

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name, self.type, self.columns, self.condition)

    def __eq__(self, other):
        return hash(self) == hash(other)

    @property
    def name(self):
        return f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        parts = [self.name, f"ON {self._table.name}", f"USING {self.type}"]
        if self.expression is not None:
            parts.append(f"({self.expression})")
        else:
            parts.append(f"({', '.join(self.columns)})")
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
    columns_by_name: dict[str, Column] = field(init=False)
    primary_key: Column | None = field(init=False)
    constraints: tuple[Constraint, ...] = ()
    indexes: tuple[Index, ...] = ()

    def __post_init__(self):
        for construct in chain(self.columns, self.constraints, self.indexes):
            if construct._table is not None:
                raise ValueError(f"{construct} is already attached to {construct._table}")
            construct._table = self
        self.columns_by_name = {}
        for column in self.columns:
            if column.name in self.columns_by_name:
                raise ValueError(f"column {column.name} is already defined in {self}")
            self.columns_by_name[column.name] = column
        self.primary_key = first((c for c in self.columns if c.is_primary_key), None)

    def __str__(self):
        columns_str = ", ".join(f"{c.name} {c.type}" for c in self.columns)
        constraints_str = ", ".join(f"{c.name} {c.type}" for c in self.constraints)
        indexes_str = ", ".join(f"{c.name} {c.type}" for c in self.indexes)
        return (
            f"{self.name} ({columns_str}, constraints=[{constraints_str}], indexes=[{indexes_str}])"
        )

    def __repr__(self):
        return f"<Table {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name, self.columns, self.constraints, self.indexes)

    def __eq__(self, other):
        return hash(self) == hash(other)

    def walk(self) -> tuple[Construct, ...]:
        return self, *self.columns, *self.constraints, *self.indexes

    def columns_include(self, other: "Table") -> bool:
        """Returns True if the columns are equal, ignoring order."""
        for column in self.columns_by_name:
            if column not in other.columns_by_name:
                return False
            if self.columns_by_name[column].type != other.columns_by_name[column].type:
                return False
        return True


# template for actual record tables
BASE_RECORD_TABLE = Table(
    "record_base",
    columns=(
        Column("id", ColumnType.UUID, is_primary_key=True),
        Column("ck", ColumnType.UUID),
        Column("created_at", ColumnType.DATETIME, default="now()"),
        Column("updated_at", ColumnType.DATETIME, default="now()"),
        Column("deleted_at", ColumnType.DATETIME, is_nullable=True),
        Column("created_by_id", ColumnType.UUID, is_nullable=True),
        Column("last_edited_at", ColumnType.DATETIME, default="now()"),
        Column("last_edited_by_id", ColumnType.UUID, is_nullable=True),
        Column("revision", ColumnType.BIGINT, default="0"),
        Column("statement_key", ColumnType.STRING, length=16),
    ),
    constraints=(
        # ck + statement_key must be unique
        Constraint(
            "unique_ck_statement_key", ConstraintType.UNIQUE, columns=["ck", "statement_key"]
        ),
    ),
    indexes=(
        # for fetching all records of a 'database'
        Index("statement_key_deleted_at", IndexType.BTREE, columns=["statement_key", "deleted_at"]),
    ),
)
# 'hufflepuff' table for ephemeral 'tables' without real tables
EPHEMERAL_RECORD_TABLE = Table(
    "record_ephemeral",
    columns=(
        *(c.clone() for c in BASE_RECORD_TABLE.columns),
        Column("statement_ck", ColumnType.UUID),
        Column("statement_id", ColumnType.UUID),
        Column("value", ColumnType.JSON, is_nullable=True),
    ),
    constraints=(*(c.clone() for c in BASE_RECORD_TABLE.constraints),),
    indexes=(*(i.clone() for i in BASE_RECORD_TABLE.indexes),),
)


def get_record_table_name(statement_ck: UUID) -> str:
    """First 16 hex digits without dashes."""
    return f"record_{str(statement_ck).replace('-', '')}"


# for internal use only
MIGRATION_TABLE = Table(
    "_migration",
    columns=(
        Column("id", ColumnType.INT, is_primary_key=True),
        Column("applied_at", ColumnType.DATETIME),
        Column("runtime_version", ColumnType.STRING),
        Column("module_version", ColumnType.STRING),
        Column("hash", ColumnType.BIGINT),
    ),
)
CONSTRUCT_TABLE = Table(
    "_construct",
    columns=(
        Column("id", ColumnType.UUID, is_primary_key=True),
        Column("kind", ColumnType.STRING),
        Column("table_name", ColumnType.STRING, is_nullable=True),
        Column("name", ColumnType.STRING),
        Column("hash", ColumnType.BIGINT),
    ),
)

INTERNAL_TABLES = (EPHEMERAL_RECORD_TABLE, MIGRATION_TABLE, CONSTRUCT_TABLE)


@dataclass
class MigrationInfo:
    """Reconstructed info for a migration."""

    id: int
    version: str
    hash: int
    applied_at: datetime


@dataclass
class Migration:
    """
    A stored SQL migration for internal mappings.
    This does NOT concern Bench field changes, which nave no 'migration' concept
      (for now, and if they did it would be separate from this).
    """

    id: int

    async def apply(
        self,
        cur: psycopg.AsyncCursor,
        tables: dict[str, Table],
        constructs: dict[UUID, Construct],
    ):
        raise NotImplementedError


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
    ColumnType.VECTOR: PostgresColumnType.BYTEA,
    ColumnType.UUID: PostgresColumnType.UUID,
}

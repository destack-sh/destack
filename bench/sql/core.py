import dataclasses
import enum
import hashlib
from dataclasses import dataclass, field, is_dataclass, replace
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, ClassVar, Union
from uuid import UUID, uuid5

# TODO @Performance: check out asyncpg instead of psycopg (up to 5x faster)
#  see https://github.com/MagicStack/asyncpg
from more_itertools import first

from bench.language.const import UUID_NAMESPACE


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


class ObjectKind(enum.StrEnum):
    TABLE = "TABLE"
    COLUMN = "COLUMN"
    CONSTRAINT = "CONSTRAINT"
    INDEX = "INDEX"


@dataclass
class Object:
    kind: ClassVar[ObjectKind]

    if TYPE_CHECKING:
        name: str  # defined in subclasses as either property or field
        source: int | str | None  # 'source' of this object (if mapped)

    def sql(self) -> str:
        """Turns this object into a SQL statement."""
        raise NotImplementedError

    def source_repr(self) -> str:
        """Turns this object into Python code that defines it."""

        def _source_repr(value: Any) -> str | None:
            if hasattr(value, "source_repr"):
                return value.source_repr()
            elif isinstance(value, tuple):
                if not value:
                    return None
                if len(value) > 1:
                    return f"({', '.join(_source_repr(v) for v in value)})"
                else:
                    return f"({_source_repr(value[0])},)"
            elif isinstance(value, list):
                if not value:
                    return None
                return f"[{', '.join(_source_repr(v) for v in value)}]"
            elif isinstance(value, enum.Enum):
                return f"{value.__class__.__name__}.{value.name}"
            else:
                return repr(value)

        fields = dataclasses.fields(self)
        args = []
        arg_idx = 0
        for field_idx, field in enumerate(fields):
            if field.name.startswith("_"):
                continue
            value = getattr(self, field.name)
            if value == field.default:
                continue
            value = _source_repr(value)
            if value is None:
                continue
            if arg_idx == field_idx and field.default:
                args.append(value)
            else:
                args.append(f"{field.name}={value}")
            arg_idx += 1
        args_str = ", ".join(args)
        return f"{self.__class__.__name__}({args_str})"

    def walk(self) -> tuple["Object", ...]:
        return (self,)

    @property
    def hash(self) -> int:
        return hash(self)

    def __hash__(self):
        """Computes a stable hash of this object and any child objects."""
        raise NotImplementedError

    @property
    def id(self):
        return uuid5(UUID_NAMESPACE, f"{self.kind.value}:{self.name}")


@dataclass
class TableObject(Object):
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
        return uuid5(UUID_NAMESPACE, f"{self.kind.value}:{self._table.name}.{self.name}")

    def clone(self) -> "TableObject":
        """Deep copy this table object without the table reference."""
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


class CascadeAction(enum.StrEnum):
    """
    A SQL cascade action.
    """

    RESTRICT = "RESTRICT"
    CASCADE = "CASCADE"
    SET_NULL = "SET NULL"
    NO_ACTION = "NO ACTION"
    SET_DEFAULT = "SET DEFAULT"


@dataclass
class Column(TableObject):
    """
    A SQL column definition.
    """

    kind: ClassVar[ObjectKind] = ObjectKind.COLUMN

    name: str
    type: ColumnType
    source: str | int | None = None
    is_array: bool = False
    is_primary_key: bool = False
    is_foreign_key_to: str | None = None
    on_delete: CascadeAction | None = None
    is_unique: bool = False
    is_nullable: bool = False
    is_encrypted: bool = False  # nocheckin: handle Column.is_encrypted
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
            pg_type = POSTGRES_TYPE_BY_COLUMN_TYPE[self.type]
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
        if self.is_foreign_key_to is not None:
            parts.append(f"REFERENCES {self.is_foreign_key_to}")
            if self.on_delete is not None:
                parts.append(f"ON DELETE {self.on_delete}")
        return " ".join(parts)


class ConstraintType(enum.StrEnum):
    """
    A SQL constraint type.
    """

    PRIMARY_KEY = "PRIMARY KEY"
    FOREIGN_KEY = "FOREIGN KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


@dataclass
class Constraint(TableObject):
    """
    A SQL constraint.
    """

    kind: ClassVar[ObjectKind] = ObjectKind.CONSTRAINT

    inner_name: str
    type: ConstraintType
    columns: list[str] | None = None
    condition: str | None = None
    source: str | int | None = None
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
    A SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"


@dataclass
class Index(TableObject):
    """
    A SQL index.
    """

    kind: ClassVar[ObjectKind] = ObjectKind.INDEX

    inner_name: str
    type: IndexType
    columns: list[str]
    condition: str | None = None
    source: str | int | None = None
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

    @property
    def name(self):
        return f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        parts = [
            self.name,
            f"ON {self._table.name}",
            f"USING {self.type}",
            f"({', '.join(self.columns)})",
        ]
        if self.condition is not None:
            parts.append(f"WHERE {self.condition}")
        return " ".join(parts)


@dataclass
class Table(Object):
    """
    A SQL table.
    """

    kind: ClassVar[ObjectKind] = ObjectKind.TABLE

    name: str
    columns: tuple[Column, ...]
    constraints: tuple[Constraint, ...] = ()
    indexes: tuple[Index, ...] = ()
    source: str | int | None = None
    _columns_by_name: dict[str, Column] = field(init=False)
    _primary_key: Column | None = field(init=False)

    def __post_init__(self):
        for object in chain(self.columns, self.constraints, self.indexes):
            if object._table is not None:
                raise ValueError(f"{object} is already attached to {object._table}")
            object._table = self
        self._columns_by_name = {}
        for column in self.columns:
            existing = self._columns_by_name.get(column.name)
            if existing is not None:
                raise ValueError(f"column {column!r} is already defined in {self!r}: {existing!r}")
            self._columns_by_name[column.name] = column
        self._primary_key = first((c for c in self.columns if c.is_primary_key), None)

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

    def walk(self) -> tuple[Object, ...]:
        return self, *self.columns, *self.constraints, *self.indexes

    def columns_include(self, other: "Table") -> bool:
        """Returns True if the columns are equal, ignoring order."""
        for column in self._columns_by_name:
            if column not in other._columns_by_name:
                return False
            if self._columns_by_name[column].type != other._columns_by_name[column].type:
                return False
        return True


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


POSTGRES_TYPE_BY_COLUMN_TYPE = {
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
COLUMN_TYPE_BY_POSTGRES_TYPE = {v: k for k, v in POSTGRES_TYPE_BY_COLUMN_TYPE.items()}

#
# Default tables
# Migrations to these are NOT auto-generated.
#


MIGRATION_TABLE = Table(  # see bench/sql/migration.py
    "bench_migration",
    columns=(
        Column("id", ColumnType.INT, is_primary_key=True),
        Column("commit", ColumnType.STRING, length=32),
        Column("version", ColumnType.STRING, length=64),
        Column("has_global", ColumnType.BOOLEAN),
        Column("has_local", ColumnType.BOOLEAN),
        Column("applied_at", ColumnType.DATETIME, is_nullable=True),
    ),
)

# 'abstract' template for actual record tables (not a real table) :RecordSchema
RECORD_BASE_TABLE = Table(
    "bench_record_base",
    columns=(
        # ids should match with Node/RecordData property ids for clarity
        Column("id", ColumnType.UUID, is_primary_key=True, source=2),
        Column("ck", ColumnType.UUID, source=3),
        Column("revision", ColumnType.BIGINT, default="0", source=10),
        Column("created_at", ColumnType.DATETIME, default="now()", source=11),
        Column("updated_at", ColumnType.DATETIME, default="now()", source=12),
        Column("deleted_at", ColumnType.DATETIME, is_nullable=True, source=13),
        Column("created_by_id", ColumnType.UUID, is_nullable=True),
        Column("last_edited_at", ColumnType.DATETIME, default="now()", source=16),
        Column("last_edited_by_id", ColumnType.UUID, is_nullable=True),
        Column("statement_key", ColumnType.STRING, length=16, source=20),
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
RECORD_EPHEMERAL_TABLE = Table(
    "bench_record_ephemeral",
    columns=(
        *(c.clone() for c in RECORD_BASE_TABLE.columns),
        Column("statement_ck", ColumnType.UUID, source=21),
        Column("statement_id", ColumnType.UUID, source=22),
        Column("value", ColumnType.JSON, is_nullable=True, source=23),
    ),
    constraints=(*(c.clone() for c in RECORD_BASE_TABLE.constraints),),
    indexes=(*(i.clone() for i in RECORD_BASE_TABLE.indexes),),
)

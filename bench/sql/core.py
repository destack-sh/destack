import dataclasses
import enum
import hashlib
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, ClassVar, Union
from uuid import UUID

# TODO @Performance: check out asyncpg instead of psycopg (up to 5x faster)
#  see https://github.com/MagicStack/asyncpg
from more_itertools import first

from bench.language.const import ColumnType
from bench.proto.core import ProtoStrEnum


def stable_hash(*args) -> int:
    """
    Hashes a tuple of arguments deterministically.
    """
    hasher = hashlib.sha256()

    def update_hash(value):
        if isinstance(value, (list, tuple)):
            for item in value:
                update_hash(item)
        elif isinstance(value, (str, int, enum.Enum, type(None))):
            hasher.update(str(value).encode())
        else:
            raise ValueError(f"cannot hash {value!r}")

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
    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]]
    kind: ClassVar[ObjectKind]

    if TYPE_CHECKING:
        name: str  # defined in subclasses as either property or field
        _source: int | str | None  # 'source' of this object (if mapped)

    def sql(self) -> str:
        """Turns this object into a SQL block."""
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

    def hash_flat(self) -> int:
        """Get a stable hash of this object's data attributes, ignoring nested objects."""
        values = (
            getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) is not None
        )
        return stable_hash(*values)

    def diff_flat(self, other: "TableObject") -> dict[str, Any]:
        """Get a diff of this object's data attributes, ignoring nested objects."""
        assert type(self) == type(other), f"cannot diff {self!r} with {other!r}"
        return {
            field_name: getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        }

    def diff_keys(self, other: "TableObject") -> tuple[str, ...]:
        """Get the keys (field names) where this object differs from another."""
        assert type(self) == type(other), f"cannot diff {self!r} with {other!r}"
        return tuple(
            field_name
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        )


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
    def qualified_name(self) -> str:
        if self.kind == ObjectKind.TABLE or self.kind == ObjectKind.INDEX:
            return self.name
        else:
            return f"{self.table_name}.{self.name}"

    @property
    def _table(self) -> Union["Table", None]:
        raise NotImplementedError

    def clone(self) -> "TableObject":
        """Deep copy this table object without the table reference."""
        return dataclasses.replace(self, _table=None)


SqlPrimitiveSingle = Union[str, int, float, bool, datetime, UUID, bytes, type(None)]
SqlPrimitive = Union[SqlPrimitiveSingle, list[SqlPrimitiveSingle], dict[str, SqlPrimitiveSingle]]


class CascadeAction(ProtoStrEnum):
    """
    A SQL cascade action.
    """

    RESTRICT = "RESTRICT", 1
    CASCADE = "CASCADE", 2
    SET_NULL = "SET NULL", 3
    NO_ACTION = "NO ACTION", 4
    SET_DEFAULT = "SET DEFAULT", 5


@dataclass
class Column(TableObject):
    """
    A SQL column definition.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = (
        "name",
        "type",
        "is_array",
        "is_primary_key",
        "is_foreign_key_to",
        "on_delete",
        # "is_unique",, handled via constraints
        "is_nullable",
        # "is_encrypted", handled in read/write
        "length",
        "default",
    )
    kind: ClassVar[ObjectKind] = ObjectKind.COLUMN

    name: str
    type: ColumnType
    is_array: bool = False
    is_primary_key: bool = False
    is_foreign_key_to: str | None = None
    on_delete: CascadeAction | None = None
    is_unique: bool = False  # handled via constraints
    is_nullable: bool = False
    is_encrypted: bool = False
    length: int | None = None
    default: str | None = None
    _source: str | int | None = None
    _table: Union["Table", None] = None

    def __str__(self):
        args_str = ", ".join(
            f"{name}={self.__dict__[name]}"
            for name in (
                "is_array",
                "is_unique",
                "is_nullable",
                "default",
                "is_primary_key",
                "is_foreign_key_to",
                "on_delete",
            )
            if self.__dict__[name]
        )
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{args_str}])"

    def __repr__(self):
        return f"<Column {self}>"

    def type_sql(self) -> str:
        if self.type == ColumnType.STRING and self.length is not None:
            pg_type = f"VARCHAR({self.length})"
        else:
            pg_type = POSTGRES_TYPE_BY_COLUMN_TYPE[self.type]
        if self.is_array:
            pg_type += "[]"
        return pg_type

    def sql(self) -> str:
        parts = [self.name, self.type_sql()]
        if not self.is_nullable:
            parts.append("NOT NULL")
        if self.is_primary_key:
            parts.append("PRIMARY KEY")
        # uniqueness is managed via constraints
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

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("inner_name", "type", "columns", "condition")
    kind: ClassVar[ObjectKind] = ObjectKind.CONSTRAINT

    inner_name: str
    type: ConstraintType
    columns: tuple[str, ...] | None = None
    condition: str | None = None
    index: str | None = None  # existing index to use (name must be relative to same table)
    _full_name: str | None = None  # as introspected from pg (naming can change)
    _source: str | int | None = None
    _table: Union["Table", None] = None

    def __post_init__(self):
        if self.condition is not None:
            # must be wrapped in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(
                ")"
            ), f"invalid condition: {self!r}"

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Constraint {self}>"

    @property
    def name(self):
        return self._full_name or f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        parts = [self.name, self.type]
        if self.type == ConstraintType.CHECK:
            parts.append(f"({self.condition})")
        elif self.type == ConstraintType.UNIQUE:
            if self.index is not None:
                parts.append(f"USING INDEX {self.table_name}_{self.index}")
            else:
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

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = (
        "inner_name",
        "type",
        "columns",
        "is_unique",
        "condition",
    )
    kind: ClassVar[ObjectKind] = ObjectKind.INDEX

    inner_name: str
    type: IndexType
    columns: tuple[str, ...]
    is_unique: bool = False
    condition: str | None = None
    _full_name: str | None = None  # as introspected from pg (naming may change)
    _source: str | int | None = None
    _table: Union["Table", None] = None

    def __post_init__(self):
        if self.condition is not None:
            # must be wrapped in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(
                ")"
            ), f"invalid condition: {self!r}"

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"

    @property
    def name(self):
        return self._full_name or f"{self.table_name}_{self.inner_name}"

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
class Table(TableObject):
    """
    A SQL table.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[ObjectKind] = ObjectKind.TABLE

    name: str
    columns: tuple[Column, ...]
    indexes: tuple[Index, ...] = ()
    constraints: tuple[Constraint, ...] = ()
    _source: str | int | None = None
    _columns_by_name: dict[str, Column] = dataclasses.field(init=False)
    _primary_key: Column | None = dataclasses.field(init=False)

    def __post_init__(self):
        for object in chain(self.columns, self.indexes, self.constraints):
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
        return stable_hash(self.kind, self.name, self.columns, self.indexes, self.constraints)

    @property
    def table(self) -> "Table":
        return self

    @property
    def _table(self) -> "Table":
        return self

    def walk(self) -> tuple[TableObject, ...]:
        # NOTE: the order here matters and is assumed in the diff logic
        return self, *self.columns, *self.indexes, *self.constraints

    def columns_include(self, other: "Table") -> bool:
        """Returns True if the columns are equal, ignoring order."""
        for column in self._columns_by_name:
            if column not in other._columns_by_name:
                return False
            if self._columns_by_name[column].type != other._columns_by_name[column].type:
                return False
        return True

    def columns_by_name(self, *names: str) -> tuple[Column, ...]:
        return tuple(self._columns_by_name[name] for name in names)


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
    CHARACTER_VARYING = "varchar"
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


# internal postgres "udt"s (user-defined types) that we use
POSTGRES_TYPE_BY_UDT: dict[str, PostgresColumnType] = {
    "uuid": PostgresColumnType.UUID,
    "varchar": PostgresColumnType.CHARACTER_VARYING,
    "bool": PostgresColumnType.BOOLEAN,
    "int4": PostgresColumnType.INTEGER,
    "int8": PostgresColumnType.BIGINT,
    "float4": PostgresColumnType.REAL,
    "float8": PostgresColumnType.DOUBLE_PRECISION,
    "timestamptz": PostgresColumnType.TIMESTAMP,
    "timestamp": PostgresColumnType.TIMESTAMP,
    "interval": PostgresColumnType.INTERVAL,
    "jsonb": PostgresColumnType.JSONB,
    "bytea": PostgresColumnType.BYTEA,
    "text": PostgresColumnType.TEXT,
    "numeric": PostgresColumnType.NUMERIC,
}

# our column types
POSTGRES_TYPE_BY_COLUMN_TYPE: dict[ColumnType, PostgresColumnType] = {
    ColumnType.STRING: PostgresColumnType.CHARACTER_VARYING,
    ColumnType.BOOLEAN: PostgresColumnType.BOOLEAN,
    ColumnType.INT: PostgresColumnType.INTEGER,
    ColumnType.BIGINT: PostgresColumnType.BIGINT,
    ColumnType.FLOAT: PostgresColumnType.REAL,
    ColumnType.DATETIME: PostgresColumnType.TIMESTAMP,
    ColumnType.INTERVAL: PostgresColumnType.INTERVAL,
    ColumnType.JSON: PostgresColumnType.JSONB,
    ColumnType.BINARY: PostgresColumnType.BYTEA,
    ColumnType.VECTOR: PostgresColumnType.BYTEA,
    ColumnType.UUID: PostgresColumnType.UUID,
    ColumnType.BYTES: PostgresColumnType.BYTEA,
}
COLUMN_TYPE_BY_POSTGRES_TYPE = {v: k for k, v in POSTGRES_TYPE_BY_COLUMN_TYPE.items()}

#
# Default tables
#


MIGRATION_TABLE = Table(  # see bench/sql/migration.py
    "bench_migration",
    columns=(
        Column("id", ColumnType.INT, is_primary_key=True, _source=2),
        Column("version", ColumnType.STRING, is_unique=True, _source=30),
        Column("has_global", ColumnType.BOOLEAN, _source=31),
        Column("has_local", ColumnType.BOOLEAN, _source=32),
        Column("applied_at", ColumnType.DATETIME, is_nullable=True, _source=33),
    ),
)

# 'abstract' template for actual record tables (not a real table) :RecordSchema
RECORD_BASE_TABLE = Table(
    "bench_record_base",
    columns=(
        # ids should match with Node/RecordData property ids for clarity
        Column("id", ColumnType.UUID, is_primary_key=True, _source=2),
        Column("ck", ColumnType.UUID, _source=3),
        Column("revision", ColumnType.BIGINT, default="0", _source=10),
        Column("created_at", ColumnType.DATETIME, default="now()", _source=11),
        Column("updated_at", ColumnType.DATETIME, default="now()", _source=12),
        Column("deleted_at", ColumnType.DATETIME, is_nullable=True, _source=13),
        Column("archived_at", ColumnType.DATETIME, is_nullable=True, _source=14),
        Column("created_by_id", ColumnType.UUID, is_nullable=True),
        Column("last_edited_at", ColumnType.DATETIME, default="now()", _source=16),
        Column("last_edited_by_id", ColumnType.UUID, is_nullable=True),
        Column("block_key", ColumnType.STRING, _source=20),
    ),
    indexes=(
        # for fetching all records of a database
        Index(
            "bench_idx_block_key_deleted_at",
            IndexType.BTREE,
            columns=("block_key", "deleted_at"),
        ),
        Index(
            "bench_idx_block_key_archive_at",
            IndexType.BTREE,
            columns=("block_key", "archived_at"),
        ),
        # control the unique index for the ck/block_key
        # ck + block_key must be unique (order is deliberate to get ck_ and block_key_ indices)
        Index(
            "bench_idx_ck_block_key",
            IndexType.BTREE,
            is_unique=True,
            columns=("ck", "block_key"),
        ),
    ),
    constraints=(
        Constraint(
            "bench_idx_ck_block_key",
            ConstraintType.UNIQUE,
            columns=("ck", "block_key"),
            index="bench_idx_ck_block_key",
        ),
    ),
)
# 'hufflepuff' table for ephemeral 'tables' without real tables
RECORD_EPHEMERAL_TABLE = Table(
    "bench_record_ephemeral",
    columns=(
        *(c.clone() for c in RECORD_BASE_TABLE.columns),
        Column("block_ck", ColumnType.UUID, _source=21),
        Column("block_id", ColumnType.UUID, _source=22),
        Column("value_packed", ColumnType.JSON, is_nullable=True, _source=30),
    ),
    indexes=(*(i.clone() for i in RECORD_BASE_TABLE.indexes),),
    constraints=(*(c.clone() for c in RECORD_BASE_TABLE.constraints),),
)

DEFAULT_LOCAL_TABLES: tuple[Table, ...] = (MIGRATION_TABLE, RECORD_EPHEMERAL_TABLE)
DEFAULT_GLOBAL_TABLES: tuple[Table, ...] = (MIGRATION_TABLE,)

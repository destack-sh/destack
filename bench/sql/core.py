import dataclasses
import enum
from dataclasses import dataclass
from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Any, ClassVar, Self, Union, cast
from uuid import UUID

from more_itertools import first

from bench.language.const import PrimitiveType
from bench.utils.func import stable_hash


@dataclass(slots=True)
class Schema:
    extensions: tuple["Extension", ...]
    tables: tuple["Table", ...]

    @staticmethod
    def blank():
        return Schema(extensions=(), tables=())

    def walk(self):
        yield from self.extensions
        for table in self.tables:
            yield from table.walk()


class ObjectKind(enum.StrEnum):
    EXTENSION = "EXTENSION"
    TABLE = "TABLE"
    COLUMN = "COLUMN"
    CONSTRAINT = "CONSTRAINT"
    INDEX = "INDEX"


@dataclass(slots=True)
class Object:
    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]]
    kind: ClassVar[ObjectKind]

    if TYPE_CHECKING:

        @property
        def name(self) -> str:
            raise NotImplementedError

    @property
    def qualified_name(self) -> str:
        raise NotImplementedError

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
                    return f"({', '.join(cast(str, _source_repr(v)) for v in value)})"
                else:
                    assert len(value) > 0, f"empty tuple: {value!r}"
                    return f"({_source_repr(value[0])},)"
            elif isinstance(value, list):
                if not value:
                    return None
                return f"[{', '.join(cast(str, _source_repr(v)) for v in value)}]"
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
            if self.kind == ObjectKind.COLUMN and field.name == "type":
                # we want to reproduce the original type, not the encrypted type
                #  (we sneakily change the type in __post_init__)
                assert isinstance(self, Column)
                value = self._unencrypted_type or self.type
            else:
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


@dataclass(slots=True)
class Extension(Object):
    """
    A SQL extension.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[ObjectKind] = ObjectKind.EXTENSION

    name: str  # type: ignore

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<Extension {self}>"

    def __hash__(self):
        return stable_hash(self.kind, self.name)

    def __eq__(self, other):
        return isinstance(other, Extension) and self.name == other.name

    @property
    def qualified_name(self) -> str:
        return self.name

    def sql(self) -> str:
        return f"CREATE EXTENSION IF NOT EXISTS {self.name}"


@dataclass(slots=True)
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
        if self.kind == ObjectKind.TABLE:
            return cast("Table", self).name
        elif self.kind == ObjectKind.INDEX:
            return cast("Index", self).name
        else:
            return f"{self.table_name}.{getattr(self, 'name')}"

    @property
    def _table(self) -> Union["Table", None]:
        raise NotImplementedError

    def clone(self) -> "Self":
        """Deep copy this table object without the table reference."""
        return dataclasses.replace(self, _table=None)


SqlPrimitiveScalar = Union[str, int, float, bool, datetime, UUID, bytes, type(None)]
SqlPrimitive = Union[SqlPrimitiveScalar, list["SqlPrimitive"], dict[str, SqlPrimitiveScalar]]


class CascadeAction(enum.StrEnum):
    """
    A SQL cascade action.
    """

    RESTRICT = "RESTRICT"
    CASCADE = "CASCADE"
    SET_NULL = "SET NULL"
    NO_ACTION = "NO ACTION"
    SET_DEFAULT = "SET DEFAULT"


@dataclass(slots=True)
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
        # "is_unique", handled via constraints
        "is_nullable",
        # "is_encrypted", handled in read/write
        "length",
        "default",
    )
    kind: ClassVar[ObjectKind] = ObjectKind.COLUMN

    name: str  # type: ignore
    type: PrimitiveType
    is_array: bool = False
    is_primary_key: bool = False
    is_foreign_key_to: str | None = None
    on_delete: CascadeAction | None = None
    is_unique: bool = False  # handled via constraints
    is_nullable: bool = False
    is_encrypted: bool = False  # encrypted columns are always stored as bytes
    length: int | None = None
    precision: int | None = None
    scale: int | None = None
    default: str | None = None
    _source: str | int | None = None
    _table: Union["Table", None] = None  # type: ignore
    _unencrypted_type: PrimitiveType | None = None  # for encrypted columns

    def __post_init__(self):
        if self.is_encrypted:  # sneakily change the type
            self._unencrypted_type = self.type
            self.type = PrimitiveType.BYTES

    @property
    def underlying_type(self):
        return self._unencrypted_type or self.type

    def __flags_str__(self):
        parts = []
        if self.is_primary_key:
            parts.append("P")
        if self.is_foreign_key_to is not None:
            parts.append("F")
        if self.is_unique:
            parts.append("U")
        if not self.is_nullable:
            parts.append("!")
        if self.is_encrypted:
            parts.append("E")
        if self.is_array:
            parts.append("[]")
        return "".join(parts)

    def __str__(self):
        args_str = ", ".join(
            (f"{name}={getattr(self, name)}" if not isinstance(getattr(self, name), bool) else name)
            for name in (
                "is_array",
                "is_unique",
                "is_nullable",
                "is_encrypted",
                "default",
                "is_primary_key",
                "is_foreign_key_to",
                "on_delete",
            )
            if getattr(self, name, None) is not None
        )
        table_name = self._table.name if self._table else None
        if args_str:
            args_str = ", " + args_str
        return f"{table_name or '<detached>'}.{self.name} ({self.underlying_type.bench_name}{args_str}))"

    def __repr__(self):
        return f"<Column {self}>"

    def type_sql(self) -> str:
        if self.type == PrimitiveType.STRING and self.length is not None:
            pg_type = f"VARCHAR({self.length})"
        elif self.type == PrimitiveType.DECIMAL:
            pg_type = f"NUMERIC({self.precision}, {self.scale})"
        else:
            pg_type = POSTGRES_TYPE_BY_PRIMITIVE_TYPE[self.type]
        if self.is_array:
            pg_type += "[]"
        return pg_type

    def sql(self) -> str:
        parts = [f'"{self.name}"', self.type_sql()]
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


@dataclass(slots=True)
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
    _table: Union["Table", None] = None  # type: ignore

    def __post_init__(self):
        if self.columns is not None:
            self.columns = tuple(sorted(self.columns))  # ensure consistent sorting
        if self.condition is not None:
            # must be wrapped in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(
                ")"
            ), f"invalid condition: {self.condition}"

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Constraint {self}>"

    @property
    def name(self):  # type: ignore
        return self._full_name or f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        parts = [f'"{self.name}"', self.type]
        if self.type == ConstraintType.CHECK:
            parts.append(f"({self.condition})")
        elif self.type == ConstraintType.UNIQUE:
            if self.index is not None:
                parts.append(f"USING INDEX {self.table_name}_{self.index}")
            else:
                parts.append(f"({', '.join(self.columns or ())})")
        return " ".join(parts)


class IndexType(enum.StrEnum):
    """
    A SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"
    BLOOM = "BLOOM"


@dataclass(slots=True)
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
    _table: Union["Table", None] = None  # type: ignore

    def __post_init__(self):
        if self.condition is not None:
            # wrap condition in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(
                ")"
            ), f"invalid condition: {self!r}"

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"

    @property
    def name(self):  # type: ignore
        return self._full_name or f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        assert isinstance(self._table, Table), f"{self} is not attached to a table"
        parts = [
            f'"{self.name}"',
            f"ON {self._table.name}",
            f"USING {self.type}",
            f"({', '.join(self.columns)})",
        ]
        if self.condition is not None:
            parts.append(f"WHERE {self.condition}")
        return " ".join(parts)


@dataclass(slots=True)
class Table(TableObject):
    """
    A SQL table.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[ObjectKind] = ObjectKind.TABLE

    name: str  # type: ignore
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
        return f"{self.name} (columns={len(self.columns)}, constraints={len(self.constraints)}, indexes={len(self.indexes)})"

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
    "int2": PostgresColumnType.SMALLINT,
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
POSTGRES_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, PostgresColumnType] = {
    PrimitiveType.STRING: PostgresColumnType.CHARACTER_VARYING,
    PrimitiveType.BOOLEAN: PostgresColumnType.BOOLEAN,
    PrimitiveType.INT16: PostgresColumnType.SMALLINT,
    PrimitiveType.INT32: PostgresColumnType.INTEGER,
    PrimitiveType.INT64: PostgresColumnType.BIGINT,
    PrimitiveType.FLOAT32: PostgresColumnType.REAL,
    PrimitiveType.DATETIME: PostgresColumnType.TIMESTAMP,
    PrimitiveType.INTERVAL: PostgresColumnType.INTERVAL,
    PrimitiveType.JSON: PostgresColumnType.JSONB,
    PrimitiveType.VECTOR: PostgresColumnType.BYTEA,
    PrimitiveType.UUID: PostgresColumnType.UUID,
    PrimitiveType.BYTES: PostgresColumnType.BYTEA,
}
PRIMITIVE_TYPE_BY_POSTGRES_TYPE = {v: k for k, v in POSTGRES_TYPE_BY_PRIMITIVE_TYPE.items()}

#
# Default tables
#

BASE_EXTENSIONS = (
    Extension("plpgsql"),
    Extension("uuid-ossp"),
    Extension("pgcrypto"),
    Extension("bloom"),
)
LOCAL_EXTENSIONS = (
    *BASE_EXTENSIONS,
    # Extension("vector"), # until timescaledb image is updated (see docker-compose.dev.yml)
    Extension("pg_trgm"),
    Extension("timescaledb"),
)
GLOBAL_EXTENSIONS = (*BASE_EXTENSIONS,)
ALL_EXTENSIONS = (
    *LOCAL_EXTENSIONS,
    *(ex for ex in GLOBAL_EXTENSIONS if not any(ex.name == e.name for e in LOCAL_EXTENSIONS)),
)


MIGRATION_TABLE = Table(  # see bench/sql/migration.py
    "bench_migration",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True, _source=2),
        Column("version", PrimitiveType.STRING, is_unique=True, _source=30),
        Column("has_global", PrimitiveType.BOOLEAN, _source=31),
        Column("has_local", PrimitiveType.BOOLEAN, _source=32),
        Column("applied_at", PrimitiveType.DATETIME, is_nullable=True, _source=33),
    ),
)

# 'abstract' template for actual record tables (not a real table) :RecordSchema
RECORD_BASE_TABLE = Table(
    "bench_record_base",
    columns=(
        # ids should match with Node/RecordData property ids for clarity
        Column("id", PrimitiveType.UUID, is_primary_key=True, _source=2),
        Column("ck", PrimitiveType.UUID, _source=3),
        Column("revision", PrimitiveType.INT64, default="0", _source=10),
        Column("created_at", PrimitiveType.DATETIME, default="now()", _source=11),
        Column("created_epoch", PrimitiveType.INT64, _source=12),
        Column("updated_at", PrimitiveType.DATETIME, default="now()", _source=13),
        Column("updated_epoch", PrimitiveType.INT64, _source=14),
        Column("deleted_at", PrimitiveType.DATETIME, is_nullable=True, _source=15),
        Column("archived_at", PrimitiveType.DATETIME, is_nullable=True, _source=16),
    ),
    indexes=(),
    constraints=(),
)
# shared record table for database blocks without real materialized tables
RECORD_SHARED_TABLE = Table(
    "bench_record_shared",
    columns=(
        *(c.clone() for c in RECORD_BASE_TABLE.columns),
        Column("block_tk", PrimitiveType.UUID, _source=21),
        Column("block_ck", PrimitiveType.UUID, _source=21),
        Column("block_id", PrimitiveType.UUID, _source=22),
        Column("value_packed", PrimitiveType.JSON, is_nullable=True, _source=30),
    ),
    indexes=(
        *(i.clone() for i in RECORD_BASE_TABLE.indexes),
        # ...?
    ),
    constraints=(*(c.clone() for c in RECORD_BASE_TABLE.constraints),),
)

DEFAULT_LOCAL_TABLES: tuple[Table, ...] = (MIGRATION_TABLE, RECORD_SHARED_TABLE)
DEFAULT_GLOBAL_TABLES: tuple[Table, ...] = (MIGRATION_TABLE,)

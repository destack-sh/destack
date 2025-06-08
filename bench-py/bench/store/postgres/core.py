import abc
import dataclasses
import enum
from collections.abc import Sequence
from dataclasses import dataclass
from itertools import chain
from typing import TYPE_CHECKING, Any, ClassVar, Self, Union, cast

from more_itertools import first

from bench.language import (
    CustomEntityDefinition,
    Edit,
    Field,
    IndexIn,
    NodeReference,
    NodeType,
    PrimitiveType,
    Property,
    RelationReference,
)
from bench.utils.func import hash_stable

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@dataclass(slots=True)
class PostgresSchema:
    extensions: tuple["PostgresExtension", ...]
    tables: tuple["PostgresTable", ...]
    _tables_by_name: dict[str, "PostgresTable"] = dataclasses.field(init=False)

    def __post_init__(self):
        self._tables_by_name = {table.name: table for table in self.tables}

    def __str__(self):
        return f"extensions={len(self.extensions)}, tables={len(self.tables)}"

    def __repr__(self):
        return f"<Schema {self}>"

    def walk(self):
        yield from self.extensions
        for table in self.tables:
            yield from table.walk()

    @staticmethod
    def blank():
        return PostgresSchema(extensions=(), tables=())


class DatabaseObjectKind(enum.StrEnum):
    EXTENSION = "EXTENSION"
    TABLE = "TABLE"
    COLUMN = "COLUMN"
    CONSTRAINT = "CONSTRAINT"
    INDEX = "INDEX"


@dataclass(slots=True)
class PostgresObject:
    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]]
    kind: ClassVar[DatabaseObjectKind]

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

    def walk(self) -> tuple["PostgresObject", ...]:
        return (self,)

    def hash_flat(self) -> int:
        """Get a stable hash of this object's data attributes, ignoring nested objects."""
        values = (
            getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) is not None
        )
        return hash_stable(*values)

    def diff_flat(self, other: "PostgresTableObject") -> dict[str, Any]:
        """Get a diff of this object's data attributes, ignoring nested objects."""
        assert type(self) is type(other), f"cannot diff {self!r} with {other!r}"
        return {
            field_name: getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        }

    def diff_keys(self, other: "PostgresTableObject") -> tuple[str, ...]:
        """Get the keys (field names) where this object differs from another."""
        assert type(self) is type(other), f"cannot diff {self!r} with {other!r}"
        return tuple(
            field_name
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        )


@dataclass(slots=True)
class PostgresExtension(PostgresObject):
    """
    A SQL extension.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[DatabaseObjectKind] = DatabaseObjectKind.EXTENSION

    name: str  # type: ignore

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<Extension {self}>"

    def __hash__(self):
        return hash_stable(self.kind, self.name)

    def __eq__(self, other):
        return isinstance(other, PostgresExtension) and self.name == other.name

    @property
    def qualified_name(self) -> str:
        return self.name

    def sql(self) -> str:
        return f"CREATE EXTENSION IF NOT EXISTS {self.name}"


@dataclass(slots=True)
class PostgresTableObject(PostgresObject):
    @property
    def table(self) -> "PostgresTable":
        assert self._table is not None, f"{self} is not attached to a table"
        return self._table

    @property
    def table_name(self) -> str:
        return self.table.name

    @property
    def qualified_name(self) -> str:
        if self.kind == DatabaseObjectKind.TABLE:
            return cast("PostgresTable", self).name
        elif self.kind == DatabaseObjectKind.INDEX:
            return cast("PostgresIndex", self).name
        else:
            return f"{self.table_name}.{getattr(self, 'name')}"

    @property
    def _table(self) -> Union["PostgresTable", None]:
        raise NotImplementedError

    def clone(self) -> "Self":
        """Deep copy this table object without any external references."""
        return dataclasses.replace(self, _table=None)


class PostgresCascadeAction(enum.StrEnum):
    """
    A SQL cascade action.
    """

    RESTRICT = "RESTRICT"
    CASCADE = "CASCADE"
    SET_NULL = "SET NULL"
    NO_ACTION = "NO ACTION"
    SET_DEFAULT = "SET DEFAULT"


@dataclass(slots=True)
class PostgresColumn(PostgresTableObject):
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
        "length",
        "default",
    )
    kind: ClassVar[DatabaseObjectKind] = DatabaseObjectKind.COLUMN

    name: str  # type: ignore
    type: PrimitiveType
    prop: "Property"
    field: Union["Field", None] = None
    is_array: bool = False
    is_primary_key: bool = False
    is_foreign_key_to: str | None = None
    on_delete: PostgresCascadeAction | None = None
    is_unique: bool = False  # handled via constraints
    is_nullable: bool = False
    length: int | None = None
    precision: int | None = None
    scale: int | None = None
    default: str | None = None
    _table: Union["PostgresTable", None] = None  # type: ignore

    def clone(self) -> "Self":
        """Deep copy this Column without the Table / Field reference."""
        return dataclasses.replace(self, _table=None)

    def __flags_str__(self):
        parts = []
        if self.is_primary_key:
            parts.append("is_primary_key")
        if self.is_foreign_key_to is not None:
            parts.append(f"is_foreign_key_to={self.is_foreign_key_to}")
        if self.is_unique:
            parts.append("is_unique")
        if self.is_nullable:
            parts.append("is_nullable")
        if self.is_array:
            parts.append("is_array")
        return ", ".join(parts)

    def __str__(self):
        args_str = ", ".join(
            (f"{name}={getattr(self, name)}" if not isinstance(getattr(self, name), bool) else name)
            for name in (
                "is_array",
                "is_unique",
                "is_nullable",
                "default",
                "is_primary_key",
                "is_foreign_key_to",
                "on_delete",
            )
            if getattr(self, name, None)
        )
        table_name = self._table.name if self._table else None
        if args_str:
            args_str = ", " + args_str
        return f"{table_name or '<detached>'}.{self.name} ({self.type.camel_name}{args_str})"

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


class PostgresConstraintType(enum.StrEnum):
    """
    A SQL constraint type.
    """

    PRIMARY_KEY = "PRIMARY KEY"
    FOREIGN_KEY = "FOREIGN KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


@dataclass(slots=True)
class PostgresConstraint(PostgresTableObject):
    """
    A SQL constraint.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("inner_name", "type", "columns", "condition")
    kind: ClassVar[DatabaseObjectKind] = DatabaseObjectKind.CONSTRAINT

    inner_name: str
    type: PostgresConstraintType
    columns: tuple[str, ...] | None = None
    condition: str | None = None
    index: str | None = None  # existing index to use (name must be relative to same table)
    _full_name: str | None = None  # as introspected from pg (naming can change)
    _source: str | int | None = None
    _table: Union["PostgresTable", None] = None  # type: ignore

    def __post_init__(self):
        if self.columns is not None:
            self.columns = tuple(sorted(self.columns))  # ensure consistent sorting
        if self.condition is not None:
            # must be wrapped in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(")"), (
                f"invalid condition: {self.condition}"
            )

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
        if self.type == PostgresConstraintType.CHECK:
            parts.append(f"({self.condition})")
        elif self.type == PostgresConstraintType.UNIQUE:
            if self.index is not None:
                parts.append(f"USING INDEX {self.table_name}_{self.index}")
            else:
                parts.append(f"({', '.join(self.columns or ())})")
        return " ".join(parts)


class PostgresIndexType(enum.StrEnum):
    """
    A SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"
    BLOOM = "BLOOM"


@dataclass(slots=True)
class PostgresIndex(PostgresTableObject):
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
    kind: ClassVar[DatabaseObjectKind] = DatabaseObjectKind.INDEX

    inner_name: str
    type: PostgresIndexType
    columns: tuple[str, ...]
    cover: tuple[str, ...] = ()
    is_unique: bool = False
    condition: str | None = None
    _full_name: str | None = None  # as introspected from pg (naming may change)
    _source: str | int | None = None
    _table: Union["PostgresTable", None] = None  # type: ignore

    def __post_init__(self):
        if self.condition is not None:
            # wrap condition in parentheses
            assert self.condition.startswith("(") and self.condition.endswith(")"), (
                f"invalid condition: {self!r}"
            )

    def __str__(self):
        table_name = self._table.name if self._table else None
        return f"{table_name or '<detached>'}.{self.name} ({self.type}) [{self.columns}, condition={self.condition}])"

    def __repr__(self):
        return f"<Index {self}>"

    @property
    def name(self):  # type: ignore
        return self._full_name or f"{self.table_name}_{self.inner_name}"

    def sql(self) -> str:
        assert isinstance(self._table, PostgresTable), f"{self} is not attached to a table"
        parts = [
            f'"{self.name}"',
            f'ON "{self._table.name}"',
            f"USING {self.type}",
            f"({', '.join(self.columns)})",
        ]
        if self.cover:
            parts.append(f"INCLUDE ({', '.join(self.cover)})")
        if self.condition is not None:
            parts.append(f"WHERE {self.condition}")
        return " ".join(parts)

    @staticmethod
    def from_index_in(name: str, index_in: IndexIn) -> "PostgresIndex":
        return PostgresIndex(
            inner_name=name,
            type=PostgresIndexType.BTREE,
            columns=index_in.columns,
            cover=index_in.cover,
            is_unique=index_in.is_unique,
            condition=index_in.condition,
        )


@dataclass(slots=True)
class PostgresTable(PostgresTableObject):
    """
    A SQL table for a Node (builtin or custom).
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[DatabaseObjectKind] = DatabaseObjectKind.TABLE

    name: str  # type: ignore
    node_type: NodeType
    columns: tuple[PostgresColumn, ...]
    indexes: tuple[PostgresIndex, ...] = ()
    constraints: tuple[PostgresConstraint, ...] = ()
    _columns_by_name: dict[str, PostgresColumn] = dataclasses.field(init=False)
    _primary_key: PostgresColumn | None = dataclasses.field(init=False)
    _table: Union["CustomEntityDefinition", None] = None  # type: ignore

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
        return hash_stable(self.kind, self.name, self.columns, self.indexes, self.constraints)

    @property
    def table(self) -> "PostgresTable":
        return self

    @property
    def _table(self) -> "PostgresTable":
        return self

    def walk(self) -> tuple[PostgresTableObject, ...]:
        # NOTE: the order here matters and is assumed in the diff logic
        return self, *self.columns, *self.indexes, *self.constraints


class PostgresContext(abc.ABC):
    """Progressive context for Database operations."""

    @abc.abstractmethod
    def apply(self, edits: Sequence[Edit]) -> Sequence[Edit]:
        """Apply the Edits to the context. Returns the Edits that were applied."""
        ...

    @abc.abstractmethod
    def resolve_relation(self, relation: RelationReference) -> Sequence[RelationReference]:
        """Expand the specific Relations for a RelationReference."""
        ...

    @abc.abstractmethod
    def get_relation(self, relation: RelationReference | NodeReference) -> PostgresTable:
        """Get the (single) Table for a node / relation. Doesn't work for multi-relations."""
        ...


class PostgresColumnType(enum.StrEnum):
    """
    A PostgreSQL column type.
    """

    BIGINT = "bigint"
    BIGSERIAL = "bigserial"
    BIT = "bit"
    BIT_VARYING = "bit varying"
    BOOLEAN = "boolean"
    BOX = "box"
    BYTEA = "bytea"
    CHARACTER = "character"
    CHARACTER_VARYING = "varchar"
    CIDR = "cidr"
    CIRCLE = "circle"
    DATE = "date"
    DOUBLE_PRECISION = "double precision"
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
    "date": PostgresColumnType.DATE,
    "time": PostgresColumnType.TIME,
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
    PrimitiveType.FLOAT64: PostgresColumnType.DOUBLE_PRECISION,
    PrimitiveType.DATETIME: PostgresColumnType.TIMESTAMP,
    PrimitiveType.DATE: PostgresColumnType.DATE,
    PrimitiveType.TIME: PostgresColumnType.TIME,
    PrimitiveType.DURATION: PostgresColumnType.INTERVAL,
    PrimitiveType.JSON: PostgresColumnType.JSONB,
    PrimitiveType.VECTOR: PostgresColumnType.BYTEA,
    PrimitiveType.UUID: PostgresColumnType.UUID,
    PrimitiveType.BYTES: PostgresColumnType.BYTEA,
}
PRIMITIVE_TYPE_BY_POSTGRES_TYPE = {v: k for k, v in POSTGRES_TYPE_BY_PRIMITIVE_TYPE.items()}


EXTENSIONS = (
    PostgresExtension("plpgsql"),
    PostgresExtension("uuid-ossp"),
    PostgresExtension("pgcrypto"),
)

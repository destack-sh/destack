import dataclasses
import enum
from dataclasses import dataclass
from itertools import chain
from typing import TYPE_CHECKING, Any, ClassVar, Self, Union, cast

from more_itertools import first

from bench.language import ConditionalType, IndexIn, PrimitiveType, SortType
from bench.utils.func import hash_stable

if TYPE_CHECKING:
    from bench.language import CustomNodeDefinition, Field


@dataclass(slots=True)
class SqlSchema:
    extensions: tuple["SqlExtension", ...]
    tables: tuple["SqlTable", ...]
    _tables_by_name: dict[str, "SqlTable"] = dataclasses.field(init=False)

    def __post_init__(self):
        self._tables_by_name = {table.name: table for table in self.tables}

    def __str__(self):
        return f"extensions={len(self.extensions)}, tables={len(self.tables)}"

    def __repr__(self):
        return f"<Schema {self}>"

    def get_table(self, name: str) -> "SqlTable":
        table = self._tables_by_name.get(name)
        if table is None:
            raise KeyError(f"no table {name!r} in {self!r}")
        return table

    def walk(self):
        yield from self.extensions
        for table in self.tables:
            yield from table.walk()

    @staticmethod
    def blank():
        return SqlSchema(extensions=(), tables=())


class SqlObjectKind(enum.StrEnum):
    EXTENSION = "EXTENSION"
    TABLE = "TABLE"
    COLUMN = "COLUMN"
    CONSTRAINT = "CONSTRAINT"
    INDEX = "INDEX"


@dataclass(slots=True)
class SqlObject:
    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]]
    kind: ClassVar[SqlObjectKind]

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
            if self.kind == SqlObjectKind.COLUMN and field.name == "type":
                # we want to reproduce the original type, not the encrypted type
                #  (we sneakily change the type in __post_init__)
                assert isinstance(self, SqlColumn)
                value = self.type
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

    def walk(self) -> tuple["SqlObject", ...]:
        return (self,)

    def hash_flat(self) -> int:
        """Get a stable hash of this object's data attributes, ignoring nested objects."""
        values = (
            getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) is not None
        )
        return hash_stable(*values)

    def diff_flat(self, other: "SqlTableObject") -> dict[str, Any]:
        """Get a diff of this object's data attributes, ignoring nested objects."""
        assert type(self) is type(other), f"cannot diff {self!r} with {other!r}"
        return {
            field_name: getattr(self, field_name)
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        }

    def diff_keys(self, other: "SqlTableObject") -> tuple[str, ...]:
        """Get the keys (field names) where this object differs from another."""
        assert type(self) is type(other), f"cannot diff {self!r} with {other!r}"
        return tuple(
            field_name
            for field_name in self.FLAT_DATA_FIELDS
            if getattr(self, field_name) != getattr(other, field_name)
        )


@dataclass(slots=True)
class SqlExtension(SqlObject):
    """
    A SQL extension.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[SqlObjectKind] = SqlObjectKind.EXTENSION

    name: str  # type: ignore

    def __str__(self):
        return self.name

    def __repr__(self):
        return f"<Extension {self}>"

    def __hash__(self):
        return hash_stable(self.kind, self.name)

    def __eq__(self, other):
        return isinstance(other, SqlExtension) and self.name == other.name

    @property
    def qualified_name(self) -> str:
        return self.name

    def sql(self) -> str:
        return f"CREATE EXTENSION IF NOT EXISTS {self.name}"


@dataclass(slots=True)
class SqlTableObject(SqlObject):
    @property
    def table(self) -> "SqlTable":
        assert self._table is not None, f"{self} is not attached to a table"
        return self._table

    @property
    def table_name(self) -> str:
        return self.table.name

    @property
    def qualified_name(self) -> str:
        if self.kind == SqlObjectKind.TABLE:
            return cast("SqlTable", self).name
        elif self.kind == SqlObjectKind.INDEX:
            return cast("SqlIndex", self).name
        else:
            return f"{self.table_name}.{getattr(self, 'name')}"

    @property
    def _table(self) -> Union["SqlTable", None]:
        raise NotImplementedError

    def clone(self) -> "Self":
        """Deep copy this table object without any external references."""
        return dataclasses.replace(self, _table=None)


class SqlCascadeAction(enum.StrEnum):
    """
    A SQL cascade action.
    """

    RESTRICT = "RESTRICT"
    CASCADE = "CASCADE"
    SET_NULL = "SET NULL"
    NO_ACTION = "NO ACTION"
    SET_DEFAULT = "SET DEFAULT"


@dataclass(slots=True)
class SqlColumn(SqlTableObject):
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
    kind: ClassVar[SqlObjectKind] = SqlObjectKind.COLUMN

    name: str  # type: ignore
    type: PrimitiveType
    is_array: bool = False
    is_primary_key: bool = False
    is_foreign_key_to: str | None = None
    on_delete: SqlCascadeAction | None = None
    is_unique: bool = False  # handled via constraints
    is_nullable: bool = False
    length: int | None = None
    precision: int | None = None
    scale: int | None = None
    default: str | None = None
    _field: Union["Field", None] = None
    _table: Union["SqlTable", None] = None  # type: ignore

    def clone(self) -> "Self":
        """Deep copy this Column without the Table / Field reference."""
        return dataclasses.replace(self, _table=None, _field=None)

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
        return f"{table_name or '<detached>'}.{self.name} ({self.type.bench_name}{args_str})"

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


class SqlConstraintType(enum.StrEnum):
    """
    A SQL constraint type.
    """

    PRIMARY_KEY = "PRIMARY KEY"
    FOREIGN_KEY = "FOREIGN KEY"
    UNIQUE = "UNIQUE"
    CHECK = "CHECK"


@dataclass(slots=True)
class SqlConstraint(SqlTableObject):
    """
    A SQL constraint.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("inner_name", "type", "columns", "condition")
    kind: ClassVar[SqlObjectKind] = SqlObjectKind.CONSTRAINT

    inner_name: str
    type: SqlConstraintType
    columns: tuple[str, ...] | None = None
    condition: str | None = None
    index: str | None = None  # existing index to use (name must be relative to same table)
    _full_name: str | None = None  # as introspected from pg (naming can change)
    _source: str | int | None = None
    _table: Union["SqlTable", None] = None  # type: ignore

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
        if self.type == SqlConstraintType.CHECK:
            parts.append(f"({self.condition})")
        elif self.type == SqlConstraintType.UNIQUE:
            if self.index is not None:
                parts.append(f"USING INDEX {self.table_name}_{self.index}")
            else:
                parts.append(f"({', '.join(self.columns or ())})")
        return " ".join(parts)


class SqlIndexType(enum.StrEnum):
    """
    A SQL index type.
    """

    BTREE = "BTREE"
    HASH = "HASH"
    GIN = "GIN"
    GIST = "GIST"
    BLOOM = "BLOOM"


@dataclass(slots=True)
class SqlIndex(SqlTableObject):
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
    kind: ClassVar[SqlObjectKind] = SqlObjectKind.INDEX

    inner_name: str
    type: SqlIndexType
    columns: tuple[str, ...]
    cover: tuple[str, ...] = ()
    is_unique: bool = False
    condition: str | None = None
    _full_name: str | None = None  # as introspected from pg (naming may change)
    _source: str | int | None = None
    _table: Union["SqlTable", None] = None  # type: ignore

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
        assert isinstance(self._table, SqlTable), f"{self} is not attached to a table"
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
    def from_index_in(name: str, index_in: IndexIn) -> "SqlIndex":
        return SqlIndex(
            inner_name=name,
            type=SqlIndexType.BTREE,
            columns=index_in.columns,
            cover=index_in.cover,
            is_unique=index_in.is_unique,
            condition=index_in.condition,
        )


@dataclass(slots=True)
class SqlTable(SqlTableObject):
    """
    A SQL table.
    """

    FLAT_DATA_FIELDS: ClassVar[tuple[str, ...]] = ("name",)
    kind: ClassVar[SqlObjectKind] = SqlObjectKind.TABLE

    name: str  # type: ignore
    columns: tuple[SqlColumn, ...]
    indexes: tuple[SqlIndex, ...] = ()
    constraints: tuple[SqlConstraint, ...] = ()
    _table: Union["CustomNodeDefinition", None] = None  # type: ignore
    _columns_by_name: dict[str, SqlColumn] = dataclasses.field(init=False)
    _columns_by_field: dict["Field", SqlColumn] = dataclasses.field(init=False)
    _primary_key: SqlColumn | None = dataclasses.field(init=False)

    def __post_init__(self):
        for object in chain(self.columns, self.indexes, self.constraints):
            if object._table is not None:
                raise ValueError(f"{object} is already attached to {object._table}")
            object._table = self
        self._columns_by_name = {}
        self._columns_by_field = {}
        for column in self.columns:
            existing = self._columns_by_name.get(column.name)
            if existing is not None:
                raise ValueError(f"column {column!r} is already defined in {self!r}: {existing!r}")
            self._columns_by_name[column.name] = column
            if column._field is not None:
                if column._field in self._columns_by_field:
                    raise ValueError(
                        f"field {column._field!r} is already bound to {self._columns_by_field[column._field]!r}"
                    )
                self._columns_by_field[column._field] = column
        self._primary_key = first((c for c in self.columns if c.is_primary_key), None)

    def __str__(self):
        return f"{self.name} (columns={len(self.columns)}, constraints={len(self.constraints)}, indexes={len(self.indexes)})"

    def __repr__(self):
        return f"<Table {self}>"

    def __hash__(self):
        return hash_stable(self.kind, self.name, self.columns, self.indexes, self.constraints)

    @property
    def table(self) -> "SqlTable":
        return self

    @property
    def _table(self) -> "SqlTable":
        return self

    def walk(self) -> tuple[SqlTableObject, ...]:
        # NOTE: the order here matters and is assumed in the diff logic
        return self, *self.columns, *self.indexes, *self.constraints

    def get_column(self, key: "str | Field") -> SqlColumn:
        if isinstance(key, str):
            column = self._columns_by_name.get(key)
            if column is None:
                raise KeyError(f"no column {key!r} in {self!r}")
            return column
        else:
            column = self._columns_by_field.get(key)
            if column is None:
                context_fields = [*self._columns_by_field.keys()]
                source_fields = key.parent.get_children(Field) if key.parent else ()
                raise KeyError(
                    f"no column for field {key!r} in {self!r} (context={context_fields}, tracked={source_fields})"
                )
            return column

    def columns_by_name(self, *names: str) -> tuple[SqlColumn, ...]:
        return tuple(self._columns_by_name[name] for name in names)


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


PG_CAST_PRIMITIVE_TYPE: dict[PrimitiveType, str] = {
    PrimitiveType.BOOLEAN: "boolean",
    PrimitiveType.INT32: "int",
    PrimitiveType.INT64: "bigint",
    PrimitiveType.FLOAT32: "float",
    PrimitiveType.FLOAT64: "double",
    PrimitiveType.DECIMAL: "decimal",
    PrimitiveType.STRING: "text",
    PrimitiveType.VECTOR: "float[]",
    PrimitiveType.BYTES: "bytea",
    PrimitiveType.DATETIME: "timestamptz",
    PrimitiveType.DATE: "date",
    PrimitiveType.TIME: "time",
    PrimitiveType.DURATION: "interval",
    PrimitiveType.JSON: "jsonb",
    PrimitiveType.UUID: "uuid",
}


class PostgresConditionalOp(enum.StrEnum):
    # logical
    TRUE = "TRUE"
    FALSE = "FALSE"
    AND = "AND"
    OR = "OR"
    NOT = "NOT"
    # standard
    IS_NULL = "IS NULL"
    IS_NOT_NULL = "IS NOT NULL"
    EQ = "="
    NEQ = "!="
    LT = "<"
    LTE = "<="
    GT = ">"
    GTE = ">="
    IN = "IN"
    NOT_IN = "NOT IN"
    # string
    LIKE = "LIKE"
    ILIKE = "ILIKE"
    REGEXP = "~"
    # array/json
    CONTAINS = "@>"
    CONTAINED_BY = "<@"
    OVERLAPS = "&&"


class PostgresJoinOp(enum.StrEnum):
    INNER_JOIN = "INNER JOIN"
    LEFT_OUTER_JOIN = "LEFT OUTER JOIN"
    RIGHT_OUTER_JOIN = "RIGHT OUTER JOIN"
    FULL_OUTER_JOIN = "FULL OUTER JOIN"


class PostgresSortOp(enum.StrEnum):
    ASC = "ASC"
    DESC = "DESC"


PG_CONDITIONAL_OP_BY_BENCH: dict[ConditionalType, PostgresConditionalOp] = {
    # logical
    ConditionalType.AND: PostgresConditionalOp.AND,
    ConditionalType.OR: PostgresConditionalOp.OR,
    ConditionalType.NOT: PostgresConditionalOp.NOT,
    # standard
    ConditionalType.EXISTS: PostgresConditionalOp.IS_NOT_NULL,
    ConditionalType.NOT_EXISTS: PostgresConditionalOp.IS_NULL,
    ConditionalType.EQUALS: PostgresConditionalOp.EQ,
    ConditionalType.NOT_EQUALS: PostgresConditionalOp.NEQ,
    ConditionalType.LESS_THAN: PostgresConditionalOp.LT,
    ConditionalType.LESS_THAN_OR_EQUALS: PostgresConditionalOp.LTE,
    ConditionalType.GREATER_THAN: PostgresConditionalOp.GT,
    ConditionalType.GREATER_THAN_OR_EQUALS: PostgresConditionalOp.GTE,
    # string
    ConditionalType.MATCHES: PostgresConditionalOp.ILIKE,
    ConditionalType.STARTS_WITH: PostgresConditionalOp.ILIKE,
    ConditionalType.ENDS_WITH: PostgresConditionalOp.ILIKE,
    # containment
    ConditionalType.CONTAINS: PostgresConditionalOp.CONTAINS,
    ConditionalType.IN: PostgresConditionalOp.IN,
    ConditionalType.NOT_IN: PostgresConditionalOp.NOT_IN,
}
POSTGRES_SORT_OP_BY_BENCH: dict[SortType, PostgresSortOp] = {
    SortType.ASCENDING: PostgresSortOp.ASC,
    SortType.DESCENDING: PostgresSortOp.DESC,
}


#
# Default tables
#

BASE_EXTENSIONS = (
    SqlExtension("plpgsql"),
    SqlExtension("uuid-ossp"),
    SqlExtension("pgcrypto"),
    SqlExtension("bloom"),
)
GLOBAL_EXTENSIONS = (*BASE_EXTENSIONS,)
MAIN_EXTENSIONS = (*BASE_EXTENSIONS,)
CUSTOM_EXTENSIONS = (*BASE_EXTENSIONS,)

MIGRATION_TABLE = SqlTable(  # see bench-py/bench/store/postgres/migration.py
    "bench_migration",
    columns=(
        SqlColumn("id", PrimitiveType.INT32, is_primary_key=True),
        SqlColumn("version", PrimitiveType.STRING, is_unique=True),
        SqlColumn("has_global", PrimitiveType.BOOLEAN),
        SqlColumn("has_main", PrimitiveType.BOOLEAN),
        SqlColumn("has_custom", PrimitiveType.BOOLEAN),
        SqlColumn("applied_at", PrimitiveType.DATETIME, is_nullable=True),
    ),
)

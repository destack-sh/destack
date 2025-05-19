import base64
import datetime
from dataclasses import dataclass
from typing import (
    Any,
    cast,
)

import psycopg
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp
from psycopg.types.json import Jsonb

from bench.language import (
    NODE_CLASSES,
    NODE_TYPES,
    RUNTIME_NODE_TYPES,
    UNSET,
    Bench,
    BenchNode,
    EditType,
    Field,
    GraphData,
    IsBased,
    LegacyQuery,
    Node,
    NodeArea,
    NodeReferenceKind,
    NodeType,
    PrimitiveType,
    Property,
    Record,
    Table,
    TypeKind,
    bittuple,
    pack_builtin_object_data,
    pack_proto_date,
    pack_proto_json,
    pack_proto_time,
    unpack_builtin_object_data,
    unpack_proto_date,
    unpack_proto_json,
    unpack_proto_time,
)
from bench.language.registry import HAS_CHILD_NODE_TYPES, NODE_CLASS_BY_TYPE
from bench.pb2 import AnyNodeData, Date, EditData, ScopeData, TimeOfDay
from bench.utils.base58 import base58_encode
from bench.utils.func import to_uuid
from bench.utils.string import Casing, to_casing
from bench.utils.time import timedelta_from_isoformat

from . import schema
from .core import (
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    DEFAULT_REGIONAL_TABLES,
    GLOBAL_EXTENSIONS,
    LOCAL_EXTENSIONS,
    REGIONAL_EXTENSIONS,
    SqlCascadeAction,
    SqlColumn,
    SqlConstraint,
    SqlConstraintType,
    SqlIndex,
    SqlIndexType,
    SqlPrimitive,
    SqlSchema,
)
from .core import (
    SqlTable as SqlTable,
)
from .engine import (
    SqlContext,
    _trace_pg_span,
)

GLOBAL_CONTEXT = SqlContext()
BENCH_TABLE_PREFIX = "bench_"
BENCH_RECORD_TABLE_PREFIX = "bench_record_"
BENCH_RECORD_VALUE_PREFIX = "value_"

CASCADING_EDIT_TYPES: bittuple[EditType] = bittuple(
    EditType.ARCHIVE,
    EditType.UNARCHIVE,
    EditType.DELETE,
    EditType.RESTORE,
    EditType.ERASE,
)
CASCADING_PARENT_NODE_TYPES = HAS_CHILD_NODE_TYPES
CASCADING_CHILD_NODE_TYPES = NODE_TYPES - RUNTIME_NODE_TYPES - bittuple(NodeType.MESSAGE)


@dataclass(slots=True)
class BenchSqlContext(SqlContext):
    """Host context for SQL operations (with custom tables)."""

    bench: Bench


#
# Mapping
#


def get_node_table_name(node_type: NodeType) -> str:
    return f"{BENCH_TABLE_PREFIX}{node_type.name.lower()}"


def get_record_table_name(table: Table) -> str:
    ck_str = base58_encode(table.ck.bytes)
    return f"{BENCH_RECORD_TABLE_PREFIX}{ck_str}"


def get_record_field_name(field: Field) -> str:
    ck_str = base58_encode(field.ck.bytes)
    return f"{BENCH_RECORD_VALUE_PREFIX}{ck_str}{field.identity_key}"


def map_builtin_object_to_sql_table(
    node: type[Node], properties: list[Property] | None = None
) -> SqlTable:
    """Maps a node type into its builtin Table schema."""
    table_name = get_node_table_name(node.metatype)
    columns: list[SqlColumn] = []
    constraints: list[SqlConstraint] = []
    indexes: list[SqlIndex] = []
    properties = properties or list(node.__properties__.values())
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if not prop.is_stored or prop.ptr_prop is not None:
            continue

        if prop.node_kind is not None:
            # node ptr property
            assert prop.runtime_prop is not None, f"no runtime prop for {prop!r}"
            prop = prop.runtime_prop
            assert not prop.is_list, f"node ptr {prop!r} is a list"
            assert not prop.is_unique, f"node ptr {prop!r} is unique"
            # id
            id_column = SqlColumn(
                name=f"{prop.name}_id", type=PrimitiveType.UUID, is_nullable=not prop.is_required
            )
            columns.append(id_column)
            # ck
            if "ck" not in prop.node_exclude and any(
                "ck" in NODE_CLASS_BY_TYPE[node_type].__properties__
                for node_type in prop.node_types
            ):
                ck_column = SqlColumn(
                    name=f"{prop.name}_ck",
                    type=PrimitiveType.UUID,
                    is_nullable=not prop.is_required,
                )
                columns.append(ck_column)
            # base_id
            if "base_id" not in prop.node_exclude and any(
                issubclass(NODE_CLASS_BY_TYPE[node_type], IsBased) for node_type in prop.node_types
            ):
                base_id_column = SqlColumn(
                    name=f"{prop.name}_base_id",
                    type=PrimitiveType.UUID,
                    is_nullable=not prop.is_required,
                )
                columns.append(base_id_column)
            # bench_id
            if prop.node_bench_from is None and any(
                issubclass(NODE_CLASS_BY_TYPE[node_type], BenchNode)
                for node_type in prop.node_types
            ):
                bench_id_column = SqlColumn(
                    name=f"{prop.name}_bench_id",
                    type=PrimitiveType.UUID,
                    is_nullable=not prop.is_required,
                )
                columns.append(bench_id_column)
        else:
            # regular column
            assert (
                prop.primitive_type is not UNSET and prop.primitive_type is not None
            ), f"undetermined type for {prop!r}"
            column = SqlColumn(
                name=prop.name,
                type=prop.primitive_type,
                is_array=prop.is_list,
                is_nullable=not prop.is_required,
                is_primary_key=prop.name == "id",
                is_unique=prop.is_unique,
            )
            columns.append(column)
            # FKs
            reference_nodes = (
                prop.node_types or () if prop.node_types != "any" else NODE_TYPES.tuple
            )
            if (
                prop.node_kind == NodeReferenceKind.NODE_PARENT
                and reference_nodes
                and all(r.area == node.__area__ for r in reference_nodes)
            ):
                assert len(reference_nodes) == 1, f"stored prop {prop!r} has multiple references"
                column.is_foreign_key_to = get_node_table_name(reference_nodes[0])
                if prop.node_kind in (
                    NodeReferenceKind.NODE_PARENT,
                    NodeReferenceKind.NODE_ANCESTOR,
                ):
                    column.on_delete = SqlCascadeAction.CASCADE
                else:
                    column.on_delete = SqlCascadeAction.SET_NULL
            # unique
            if prop.is_unique:
                index = SqlIndex(
                    f"bench_idx_{prop.name}",
                    type=SqlIndexType.BTREE,
                    columns=(column.name,),
                    is_unique=prop.is_unique,
                    _source=prop.id,
                )
                indexes.append(index)
                constraint = SqlConstraint(
                    index.inner_name,  # must be the same as the index name (postgres will rename otherwise)
                    type=SqlConstraintType.UNIQUE,
                    columns=(column.name,),
                    index=index.inner_name,
                    _source=prop.id,
                )
                constraints.append(constraint)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith(
            "bench_"
        ), f"index shouldn't include prefix: {index!r}"
        extra_index = SqlIndex.from_index_in(
            f"bench_idx_{index.name or '_'.join(index.columns)}", index
        )
        indexes.append(extra_index)

    table = SqlTable(
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def map_table_to_sql_table(table: Table, prev_sql_table: SqlTable | None) -> SqlTable:
    """
    Maps a table to its corresponding custom Record Table.
    If a previous table is passed in, all its constructs will exist in the new table
     (if they are not already present in the new table).
    """
    base_sql_table = map_builtin_object_to_sql_table(
        Record,
        # all stored Record properties except value, which we unfurl into columns
        properties=list(Record.__stored_properties__.values()),
    )
    table_name = get_record_table_name(table)
    columns: list[SqlColumn] = [column.clone() for column in base_sql_table.columns]
    constraints: list[SqlConstraint] = [
        constraint.clone() for constraint in base_sql_table.constraints
    ]
    indexes: list[SqlIndex] = [index.clone() for index in base_sql_table.indexes]

    # map fields into columns
    for field in table.get_children(Field):
        if field.kind == TypeKind.PRIMITIVE:
            assert field.primitive_type is not None, f"no primitive type for {field!r}"
            primitive_type = field.primitive_type
        elif field.kind == TypeKind.NODE:
            # NOTE :Architecture: unravel custom field node refs like in builtin objects?
            primitive_type = PrimitiveType.JSON
        elif field.kind == TypeKind.STRUCT:
            primitive_type = PrimitiveType.JSON
        elif field.kind == TypeKind.ENUM:
            primitive_type = PrimitiveType.INT16
        else:
            raise TypeError(f"cannot store field in {table!r}: {field!r}")

        column = SqlColumn(
            name=get_record_field_name(field),
            type=primitive_type,
            is_array=field.is_list,
            is_nullable=True,  # NOTE :Incomplete: support field constraints in table
            is_primary_key=False,
            _field=field,
        )
        columns.append(column)

    # keep old columns
    if prev_sql_table is not None:
        columns_by_name: dict[str, SqlColumn] = {c.name: c for c in columns}
        for old_column in prev_sql_table.columns:
            if old_column.name not in columns_by_name:
                columns.append(old_column.clone())

    return SqlTable(
        _table=table,
        name=table_name,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )


#
# Packing/unpacking
#


def _pack_builtin_object_data_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif prop.is_struct:
        value = pack_builtin_object_data(value)
        return Jsonb(value)  # type: ignore
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(unpack_proto_json(value))  # type: ignore
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return cast(Timestamp, value).ToDatetime()
    elif prop.primitive_type == PrimitiveType.DATE:
        return unpack_proto_date(cast(Date, value))
    elif prop.primitive_type == PrimitiveType.TIME:
        return unpack_proto_time(cast(TimeOfDay, value))
    elif prop.primitive_type == PrimitiveType.DURATION:
        return value.ToTimedelta()
    else:
        return value


def _pack_builtin_object_data_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the value of a BuiltinObject property for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_data_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_data_prop_scalar(prop, v) for v in value]


def _pack_builtin_object_value_prop_scalar(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if prop.is_struct:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.BYTES:
        return base64.b64decode(value)
    elif prop.primitive_type == PrimitiveType.UUID:
        return to_uuid(value)
    elif prop.primitive_type == PrimitiveType.JSON:
        return Jsonb(value)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        return datetime.datetime.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DATE:
        return datetime.date.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.TIME:
        return datetime.time.fromisoformat(value)
    elif prop.primitive_type == PrimitiveType.DURATION:
        return timedelta_from_isoformat(value)
    else:
        return value


def _pack_builtin_object_value_prop(prop: Property, value: Any) -> SqlPrimitive:
    """Packs the JSON-value-packed value of a BuiltinObject for storage in Postgres."""
    if value is None:
        return None
    elif not prop.is_list:
        return _pack_builtin_object_value_prop_scalar(prop, value)
    else:
        return [_pack_builtin_object_value_prop_scalar(prop, v) for v in value]


def _unpack_builtin_object_data_prop_scalar(
    prop: Property, value_packed: Any, into: Any | None = None
) -> Any:
    """Unpacks the value of a BuiltinObject property from Postgres."""
    if value_packed is None:
        return None
    elif prop.struct_type:
        return unpack_builtin_object_data(value_packed, into=into)
    elif prop.primitive_type == PrimitiveType.UUID:
        return str(value_packed)
    elif prop.primitive_type == PrimitiveType.JSON:
        return pack_proto_json(value_packed)
    elif prop.primitive_type == PrimitiveType.DATETIME:
        ts = Timestamp()
        ts.FromDatetime(value_packed)
        return ts
    elif prop.primitive_type == PrimitiveType.DATE:
        return pack_proto_date(value_packed)
    elif prop.primitive_type == PrimitiveType.TIME:
        return pack_proto_time(value_packed)
    elif prop.primitive_type == PrimitiveType.DURATION:
        dur = Duration()
        dur.FromTimedelta(value_packed)
        return dur
    else:
        return value_packed


@_trace_pg_span
async def pg_graph_select(
    *, cur: psycopg.AsyncCursor, ctx: SqlContext, query: LegacyQuery
) -> list[AnyNodeData]:
    """
    Selects the nodes from the graph matching the given query.
    Only the given node type is selected, no joins are performed (up/down or sideways).
    """
    raise NotImplementedError


@_trace_pg_span
async def pg_graph_count(*, cur: psycopg.AsyncCursor, ctx: SqlContext, query: LegacyQuery) -> int:
    """Counts the nodes from the graph matching the given query. Ignores pagination parameters."""
    raise NotImplementedError


@_trace_pg_span
async def pg_graph_exists(*, cur: psycopg.AsyncCursor, ctx: SqlContext, query: LegacyQuery) -> bool:
    """Checks if nodes from the graph matching the given query exist."""
    raise NotImplementedError


@_trace_pg_span
async def pg_graph_get(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    query: LegacyQuery,
    visited_graph: GraphData,
) -> None:
    """
    Gets the 'root' nodes from a Query (Query.roots) and recursively reads up/down the graph.
    Also performs any additional joins needed for the query.
    """
    raise NotImplementedError


@_trace_pg_span
async def pg_graph_search(
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    scope: ScopeData,
    query: LegacyQuery,
    count: bool,
) -> tuple[list[AnyNodeData], GraphData, int | None]:
    """
    Search for roots matching the filter and then get the graph up/down/joined from there.
    """
    raise NotImplementedError


# TODO :Performance!: use psycopg3/postgres pipelining to batch edits


@_trace_pg_span
async def pg_graph_edit(
    *,
    cur: psycopg.AsyncCursor,
    ctx: SqlContext,
    edits: list[EditData] | tuple[EditData, ...],
) -> list[EditData]:
    """Apply graph edits, cascading as needed. Returns the cascaded edits."""
    raise NotImplementedError


#
# Builtin table registry
#

BUILTIN_TABLE_BY_NODE_TYPE: dict[NodeType, SqlTable] = {
    # read previously generated tables in schema.py
    node_type: getattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
    for node_type in NODE_TYPES
    if hasattr(schema, f"{to_casing(node_type.name, Casing.ALL_CAPS)}_TABLE")
}
BUILTIN_NODE_BY_TABLE_NAME: dict[str, NodeType] = {
    table.name: node_type for node_type, table in BUILTIN_TABLE_BY_NODE_TYPE.items()
}
BUILTIN_NODE_TABLES: tuple[SqlTable, ...] = tuple(BUILTIN_TABLE_BY_NODE_TYPE.values())

BUILTIN_GLOBAL_TABLES: tuple[SqlTable, ...] = DEFAULT_GLOBAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.GLOBAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_REGIONAL_TABLES: tuple[SqlTable, ...] = DEFAULT_REGIONAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.REGIONAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_LOCAL_TABLES: tuple[SqlTable, ...] = DEFAULT_LOCAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASSES
    if node.__area__ == NodeArea.LOCAL and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_TABLES_BY_AREA: dict[NodeArea, tuple[SqlTable, ...]] = {
    NodeArea.GLOBAL: BUILTIN_GLOBAL_TABLES,
    NodeArea.REGIONAL: BUILTIN_REGIONAL_TABLES,
    NodeArea.LOCAL: BUILTIN_LOCAL_TABLES,
}

BUILTIN_GLOBAL_SCHEMA = SqlSchema(GLOBAL_EXTENSIONS, BUILTIN_GLOBAL_TABLES)
BUILTIN_REGIONAL_SCHEMA = SqlSchema(REGIONAL_EXTENSIONS, BUILTIN_REGIONAL_TABLES)
BUILTIN_LOCAL_SCHEMA = SqlSchema(LOCAL_EXTENSIONS, BUILTIN_LOCAL_TABLES)
BUILTIN_SCHEMA_BY_AREA: dict[NodeArea, SqlSchema] = {
    NodeArea.GLOBAL: BUILTIN_GLOBAL_SCHEMA,
    NodeArea.REGIONAL: BUILTIN_REGIONAL_SCHEMA,
    NodeArea.LOCAL: BUILTIN_LOCAL_SCHEMA,
}

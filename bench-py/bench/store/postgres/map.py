from bench.language import (
    NODE_TYPES,
    UNSET,
    CustomNodeDefinition,
    EdgeType,
    IsInBench,
    Node,
    NodeType,
    PrimitiveType,
    TraitType,
    expand_node_types,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.utils.string import Casing, to_casing

from . import schema
from .core import (
    GLOBAL_EXTENSIONS,
    MAIN_EXTENSIONS,
    MIGRATION_TABLE,
    SqlColumn,
    SqlConstraint,
    SqlIndex,
    SqlSchema,
)
from .core import SqlTable as SqlTable

BENCH_TABLE_PREFIX = "bench_"
BENCH_CUSTOM_NODE_PREFIX = "bench_custom_"
BENCH_CUSTOM_FIELD_PREFIX = "field_"


def get_node_table_name(node_type: NodeType) -> str:
    return f"{BENCH_TABLE_PREFIX}{node_type.name.lower()}"


def map_builtin_node_to_sql_table(node: type[Node]) -> SqlTable:
    """Maps a node type into its builtin Table schema."""
    table_name = get_node_table_name(node.metatype)
    columns: list[SqlColumn] = []
    constraints: list[SqlConstraint] = []
    indexes: list[SqlIndex] = []
    properties = [p for p in node.__properties__.values() if p.is_stored and p.ptr_prop is None]
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if prop.edge_type == EdgeType.NODE_PARENT and node.__root_type__ is None:
            continue  # no parent for root nodes

        if prop.scalar_type == "node":
            # node ptr property
            assert prop.runtime_prop is not None, f"no runtime prop for {prop!r}"
            prop = prop.runtime_prop
            assert prop.cardinality == "scalar", f"non-scalar {prop!r}"
            # id
            column = SqlColumn(
                name=f"{prop.name}_id", type=PrimitiveType.UUID, is_nullable=not prop.is_required
            )
            columns.append(column)
            node_types = expand_node_types(prop.node_types or ())
            # definition_id
            if prop.node_is_customizable and NodeType.CUSTOM_NODE_INSTANCE in node_types:
                table_id_column = SqlColumn(
                    name=f"{prop.name}_definition_id",
                    type=PrimitiveType.UUID,
                    is_nullable=not prop.is_required,
                )
                columns.append(table_id_column)
            # bench_id
            if prop.node_bench_from is None and any(
                issubclass(NODE_CLASS_BY_TYPE[node_type], IsInBench) for node_type in node_types
            ):
                bench_id_column = SqlColumn(
                    name=f"{prop.name}_bench_id",
                    type=PrimitiveType.UUID,
                    is_nullable=not prop.is_required,
                )
                columns.append(bench_id_column)
        else:
            # regular column
            assert not prop.name.endswith("_ptr"), f"unexpected regular ptr: {prop!r}"
            assert prop.primitive_type is not None, f"undetermined type for {prop!r}"
            column = SqlColumn(
                name=prop.name,
                type=prop.primitive_type,
                is_array=prop.cardinality == "list",
                is_nullable=not prop.is_required,
                is_primary_key=prop.name == "id",
            )
            columns.append(column)

        # variable properties
        assert prop.is_variable is not UNSET, f"undetermined variable: {prop!r}"
        if prop.is_variable:
            column.is_nullable = True
            variable_column = SqlColumn(
                name=f"{prop.name}_variable", type=PrimitiveType.JSON, is_nullable=True
            )
            columns.append(variable_column)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith("bench_"), f"bad idnex name: {index!r}"
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


def map_custom_node_to_sql_table(
    table: CustomNodeDefinition, prev_sql_table: SqlTable | None
) -> SqlTable:
    """
    Maps a table to its corresponding custom Record Table.
    If a previous table is passed in, all its constructs will exist in the new table
     (if they are not already present in the new table).
    """
    raise NotImplementedError


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

BUILTIN_GLOBAL_TABLES: tuple[SqlTable, ...] = (
    MIGRATION_TABLE,
    *tuple(
        BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
        for node in NODE_CLASS_BY_TYPE.values()
        if TraitType.GLOBAL in node.__traits__
    ),
)
BUILTIN_MAIN_TABLES: tuple[SqlTable, ...] = (
    MIGRATION_TABLE,
    *tuple(
        BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
        for node in NODE_CLASS_BY_TYPE.values()
        if TraitType.GLOBAL not in node.__traits__
        and node.metatype != NodeType.CUSTOM_NODE_INSTANCE
    ),
)
BUILTIN_GLOBAL_SCHEMA = SqlSchema(GLOBAL_EXTENSIONS, BUILTIN_GLOBAL_TABLES)
BUILTIN_MAIN_SCHEMA = SqlSchema(MAIN_EXTENSIONS, BUILTIN_MAIN_TABLES)

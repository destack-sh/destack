from collections.abc import Mapping

from bench.language import (
    NODE_TYPES,
    CustomNodeDefinition,
    EdgeType,
    Node,
    NodeArea,
    NodeReference,
    NodeType,
    PrimitiveType,
    TraitType,
)
from bench.language.registry import NODE_CLASS_BY_TYPE

from .core import (
    EXTENSIONS,
    DatabaseColumn,
    DatabaseConstraint,
    DatabaseIndex,
    DatabaseSchema,
)
from .core import DatabaseTable as DatabaseTable

BENCH_BUILTIN_TABLE_PREFIX = "bench_"
BENCH_CUSTOM_TABLE_PREFIX = "bench_custom_"
BENCH_CUSTOM_FIELD_PREFIX = "field_"


def get_table_name(node_ptr: NodeReference) -> str:
    if node_ptr.node_type != NodeType.CUSTOM_NODE_INSTANCE:
        return f"{BENCH_BUILTIN_TABLE_PREFIX}{node_ptr.node_type.name.lower()}"
    else:
        assert node_ptr.definition_id is not None, f"no definition_id for {node_ptr!r}"
        return f"{BENCH_CUSTOM_TABLE_PREFIX}{node_ptr.definition_id}"


def map_builtin_node_to_database_table(node: type[Node]) -> DatabaseTable:
    """Maps a node type into its builtin Table schema."""

    table_name = f"{BENCH_BUILTIN_TABLE_PREFIX}{node.metatype.name.lower()}"
    columns: list[DatabaseColumn] = []
    constraints: list[DatabaseConstraint] = []
    indexes: list[DatabaseIndex] = []
    properties = [p for p in node.__properties__.values() if p.is_stored and p.ptr_prop is None]
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if prop.edge_type == EdgeType.NODE_PARENT and node.__root_type__ is None:
            continue  # no parent for root nodes

        if prop.scalar_type == "node_reference":
            # node ptr property
            assert prop.runtime_prop is not None, f"no runtime prop for {prop!r}"
            prop = prop.runtime_prop
            assert prop.cardinality == "scalar", f"non-scalar node reference: {prop!r}"
            column = DatabaseColumn(
                name=f"{prop.name}_id",
                type=PrimitiveType.UUID,
                is_nullable=not prop.is_required,
                prop=prop,
            )
            columns.append(column)
            if prop.node_has_definition:
                table_id_column = DatabaseColumn(
                    name=f"{prop.name}_definition_id",
                    type=PrimitiveType.UUID,
                    is_nullable=prop.is_optional,
                    prop=prop,
                )
                columns.append(table_id_column)
            if prop.node_has_type:
                node_type_column = DatabaseColumn(
                    name=f"{prop.name}_type",
                    type=PrimitiveType.INT16,
                    is_nullable=prop.is_optional,
                    prop=prop,
                )
                columns.append(node_type_column)
            if prop.node_has_bench:
                bench_id_column = DatabaseColumn(
                    name=f"{prop.name}_bench_id",
                    type=PrimitiveType.UUID,
                    is_nullable=prop.is_optional,
                    prop=prop,
                )
                columns.append(bench_id_column)
        else:
            # regular column
            assert not prop.name.endswith("_ptr"), f"unexpected regular ptr: {prop!r}"
            assert prop.primitive_type is not None, f"undetermined type for {prop!r}"
            column = DatabaseColumn(
                name=prop.name,
                type=prop.primitive_type,
                is_array=prop.cardinality == "list",
                is_nullable=prop.is_optional,
                is_primary_key=prop.name == "id",
                prop=prop,
            )
            columns.append(column)

        # variable properties
        if prop.is_variable:
            column.is_nullable = True
            variable_column = DatabaseColumn(
                name=f"{prop.name}_variable",
                type=PrimitiveType.JSON,
                is_nullable=True,
                prop=prop,
            )
            columns.append(variable_column)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith("bench_"), f"bad idnex name: {index!r}"
        extra_index = DatabaseIndex.from_index_in(
            f"bench_idx_{index.name or '_'.join(index.columns)}", index
        )
        indexes.append(extra_index)

    table = DatabaseTable(
        name=table_name,
        node_type=node.metatype,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def map_custom_node_to_database_table(definition: CustomNodeDefinition) -> DatabaseTable:
    """Maps a CustomNodeDefinition to its corresponding CustomNodeTable."""

    raise NotImplementedError(definition)


BUILTIN_TABLE_BY_NODE_TYPE: Mapping[NodeType, DatabaseTable] = {
    node_type: map_builtin_node_to_database_table(NODE_CLASS_BY_TYPE[node_type])
    for node_type in NODE_TYPES
}
BUILTIN_TABLE_BY_NAME: Mapping[str, DatabaseTable] = {
    table.name: table for table in BUILTIN_TABLE_BY_NODE_TYPE.values()
}
BUILTIN_NODE_TABLES: tuple[DatabaseTable, ...] = tuple(BUILTIN_TABLE_BY_NODE_TYPE.values())

BUILTIN_GLOBAL_TABLES: tuple[DatabaseTable, ...] = tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASS_BY_TYPE.values()
    if TraitType.GLOBAL in node.__traits__ and node.metatype != NodeType.CUSTOM_NODE_INSTANCE
)
BUILTIN_MAIN_TABLES: tuple[DatabaseTable, ...] = tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASS_BY_TYPE.values()
    if TraitType.GLOBAL not in node.__traits__ and node.metatype != NodeType.CUSTOM_NODE_INSTANCE
)
BUILTIN_TABLE_BY_AREA: Mapping[NodeArea, tuple[DatabaseTable, ...]] = {
    NodeArea.GLOBAL_DATABASE: BUILTIN_GLOBAL_TABLES,
    NodeArea.MAIN_DATABASE: BUILTIN_MAIN_TABLES,
}
BUILTIN_GLOBAL_SCHEMA = DatabaseSchema(EXTENSIONS, BUILTIN_GLOBAL_TABLES)
BUILTIN_MAIN_SCHEMA = DatabaseSchema(EXTENSIONS, BUILTIN_MAIN_TABLES)

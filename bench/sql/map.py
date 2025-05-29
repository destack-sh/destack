from bench.language import (
    NODE_TYPES,
    UNSET,
    CustomNodeDefinition,
    IsInBench,
    Node,
    NodeArea,
    NodeType,
    PrimitiveType,
    Property,
    expand_node_types,
)
from bench.language.registry import NODE_CLASS_BY_TYPE
from bench.utils.string import Casing, to_casing

from . import schema
from .core import (
    DEFAULT_GLOBAL_TABLES,
    DEFAULT_LOCAL_TABLES,
    DEFAULT_REGIONAL_TABLES,
    GLOBAL_EXTENSIONS,
    LOCAL_EXTENSIONS,
    REGIONAL_EXTENSIONS,
    SqlColumn,
    SqlConstraint,
    SqlConstraintType,
    SqlIndex,
    SqlIndexType,
    SqlSchema,
)
from .core import SqlTable as SqlTable

BENCH_TABLE_PREFIX = "bench_"
BENCH_CUSTOM_NODE_PREFIX = "bench_custom_"
BENCH_CUSTOM_FIELD_PREFIX = "value_"


def get_node_table_name(node_type: NodeType) -> str:
    return f"{BENCH_TABLE_PREFIX}{node_type.name.lower()}"


def map_builtin_node_to_sql_table(
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

        if prop.scalar_type == "node":
            # node ptr property
            assert prop.runtime_prop is not None, f"no runtime prop for {prop!r}"
            prop = prop.runtime_prop
            assert prop.cardinality == "scalar", f"non-scalar {prop!r}"
            assert not prop.is_unique, f"node ptr {prop!r} is unique"
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
                is_unique=prop.is_unique,
            )
            columns.append(column)
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


def map_custom_node_to_sql_table(
    table: CustomNodeDefinition, prev_sql_table: SqlTable | None
) -> SqlTable:
    """
    Maps a table to its corresponding custom Record Table.
    If a previous table is passed in, all its constructs will exist in the new table
     (if they are not already present in the new table).
    """
    raise NotImplementedError


#
# Builtin SqlTables
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
    for node in NODE_CLASS_BY_TYPE.values()
    if node.__area__ == NodeArea.GLOBAL_POSTGRES and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_REGIONAL_TABLES: tuple[SqlTable, ...] = DEFAULT_REGIONAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASS_BY_TYPE.values()
    if node.__area__ == NodeArea.MAIN_POSTGRES and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_LOCAL_TABLES: tuple[SqlTable, ...] = DEFAULT_LOCAL_TABLES + tuple(
    BUILTIN_TABLE_BY_NODE_TYPE[node.metatype]
    for node in NODE_CLASS_BY_TYPE.values()
    if node.__area__ == NodeArea.CUSTOM_POSTGRES and node.metatype in BUILTIN_TABLE_BY_NODE_TYPE
)
BUILTIN_TABLES_BY_AREA: dict[NodeArea, tuple[SqlTable, ...]] = {
    NodeArea.GLOBAL_POSTGRES: BUILTIN_GLOBAL_TABLES,
    NodeArea.MAIN_POSTGRES: BUILTIN_REGIONAL_TABLES,
    NodeArea.CUSTOM_POSTGRES: BUILTIN_LOCAL_TABLES,
}

BUILTIN_GLOBAL_SCHEMA = SqlSchema(GLOBAL_EXTENSIONS, BUILTIN_GLOBAL_TABLES)
BUILTIN_REGIONAL_SCHEMA = SqlSchema(REGIONAL_EXTENSIONS, BUILTIN_REGIONAL_TABLES)
BUILTIN_LOCAL_SCHEMA = SqlSchema(LOCAL_EXTENSIONS, BUILTIN_LOCAL_TABLES)
BUILTIN_SCHEMA_BY_AREA: dict[NodeArea, SqlSchema] = {
    NodeArea.GLOBAL_POSTGRES: BUILTIN_GLOBAL_SCHEMA,
    NodeArea.MAIN_POSTGRES: BUILTIN_REGIONAL_SCHEMA,
    NodeArea.CUSTOM_POSTGRES: BUILTIN_LOCAL_SCHEMA,
}

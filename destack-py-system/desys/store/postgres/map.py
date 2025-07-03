from cachetools import cached

from destack.language import (
    CustomEntityDefinition,
    EdgeType,
    Node,
    NodeReference,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    StoreType,
    TypeCardinality,
)
from destack.language.registry import NODE_CLASS_BY_TYPE, NODE_TYPES_BY_PRIMARY_STORE_TYPE

from .core import (
    EXTENSIONS,
    PostgresColumn,
    PostgresConstraint,
    PostgresIndex,
    PostgresIndexType,
    PostgresSchema,
)
from .core import PostgresTable as PostgresTable

DESTACK_BUILTIN_TABLE_PREFIX = "destack_"
DESTACK_CUSTOM_RECORD_PREFIX = "destack_record_"
DESTACK_CUSTOM_PROPERTY_PREFIX = "custom_"


def get_table_name(node_ptr: NodeReference) -> str:
    if node_ptr.type == NodeType.RECORD:
        assert node_ptr.definition_id is not None, f"no definition_id for {node_ptr!r}"
        return f"{DESTACK_CUSTOM_RECORD_PREFIX}{node_ptr.definition_id}"
    else:
        return f"{DESTACK_BUILTIN_TABLE_PREFIX}{node_ptr.type.name.lower()}"


def map_builtin_node_to_database_table(node: type[Node]) -> PostgresTable:
    """Maps a node type into its builtin Table schema."""

    table_name = f"{DESTACK_BUILTIN_TABLE_PREFIX}{node.metatype.name.lower()}"
    columns: list[PostgresColumn] = []
    constraints: list[PostgresConstraint] = []
    indexes: list[PostgresIndex] = []
    properties: list[PropertyDeclaration] = [p for p in node.__properties__.values() if p.is_stored]
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if prop.edge_type == EdgeType.PARENT and node.__root_type__ is None:
            continue  # no parent for root nodes

        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # unravel node ptr column
            assert prop.cardinality == TypeCardinality.SCALAR, (
                f"non-scalar node reference: {prop!r}"
            )
            column = PostgresColumn(
                name=f"{prop.name}_id",
                type=PrimitiveType.UUID,
                is_nullable=not prop.is_required,
                prop=prop,
            )
            columns.append(column)
            if prop.node_has_type:
                node_type_column = PostgresColumn(
                    name=f"{prop.name}_type",
                    type=PrimitiveType.INT32,
                    is_nullable=prop.is_optional,
                    prop=prop,
                )
                columns.append(node_type_column)
            if prop.node_has_space:
                space_id_column = PostgresColumn(
                    name=f"{prop.name}_space_id",
                    type=PrimitiveType.UUID,
                    is_nullable=True,
                    prop=prop,
                )
                columns.append(space_id_column)
            if prop.node_has_definition:
                table_id_column = PostgresColumn(
                    name=f"{prop.name}_definition_id",
                    type=PrimitiveType.UUID,
                    is_nullable=True,
                    prop=prop,
                )
                columns.append(table_id_column)
        else:
            # regular column
            assert prop.primitive_type is not None, f"undetermined type for {prop!r}"
            column = PostgresColumn(
                name=prop.name,
                type=prop.primitive_type,
                is_array=prop.cardinality == TypeCardinality.LIST,
                is_nullable=prop.is_optional,
                is_primary_key=prop.name == "id",
                prop=prop,
            )
            columns.append(column)

        # unique
        if prop.is_unique:
            index = PostgresIndex(
                inner_name=f"unique_{prop.name}",
                type=PostgresIndexType.BTREE,
                columns=(prop.name,),
                is_unique=True,
            )
            indexes.append(index)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith("destack_"), f"bad idnex name: {index!r}"
        extra_index = PostgresIndex.from_index_in(
            f"space_idx_{index.name or '_'.join(index.columns)}", index
        )
        indexes.append(extra_index)

    table = PostgresTable(
        name=table_name,
        node_type=node.metatype,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


def map_custom_node_to_database_table(definition: CustomEntityDefinition) -> PostgresTable:
    """Maps a CustomNodeDefinition to its corresponding CustomNodeTable."""

    raise NotImplementedError(definition)


@cached({})
def get_builtin_schema(*store_types: StoreType) -> PostgresSchema:
    """Gets the builtin schema for the given traits."""

    node_types: tuple[NodeType, ...] = tuple(
        {
            node_type
            for store_type in store_types
            for node_type in NODE_TYPES_BY_PRIMARY_STORE_TYPE[store_type]
        }
    )
    tables: list[PostgresTable] = [
        map_builtin_node_to_database_table(NODE_CLASS_BY_TYPE[node_type])
        for node_type in node_types
    ]
    return PostgresSchema(EXTENSIONS, tuple(tables))

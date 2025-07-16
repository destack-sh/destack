from cachetools import cached

from destack.language import (
    EdgeType,
    Node,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    StoreKey,
    TypeCardinality,
)
from destack.language.registry import NODE_CLASS_BY_TYPE, NODE_TYPES_BY_PRIMARY_STORE_KEY

from .core import (
    EXTENSIONS,
    PostgresColumn,
    PostgresConstraint,
    PostgresIndex,
    PostgresIndexType,
    PostgresSchema,
)
from .core import PostgresTable as PostgresTable

POSTGRES_BUILTIN_TABLE_PREFIX = "destack_"


def map_builtin_node_to_database_table(node: type[Node]) -> PostgresTable:
    """Maps a node type into its builtin Table schema."""

    table_name = f"{POSTGRES_BUILTIN_TABLE_PREFIX}{node.metatype.id}"
    columns: list[PostgresColumn] = []
    constraints: list[PostgresConstraint] = []
    indexes: list[PostgresIndex] = []
    properties: list[PropertyDeclaration] = [p for p in node.__properties__.values() if p.is_stored]
    properties.sort(key=lambda p: p.id or -1)

    # map properties to columns, add per-column indices
    for prop in properties:
        if prop.edge_type == EdgeType.PARENT and node.metatype == NodeType.SPACE:
            continue  # no parent for root nodes
        assert isinstance(prop.id, int), f"undetermined id for {prop!r}"

        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # unravel node ptr column
            assert prop.cardinality == TypeCardinality.SCALAR, (
                f"non-scalar node reference: {prop!r}"
            )
            column = PostgresColumn(
                name=f"{prop.id}_id",
                type=PrimitiveType.UUID,
                is_nullable=not prop.is_required,
                prop=prop,
            )
            columns.append(column)
            if prop.node_is_heterogenous:
                node_type_column = PostgresColumn(
                    name=f"{prop.id}_type",
                    type=PrimitiveType.INT32,
                    is_nullable=prop.is_optional,
                    prop=prop,
                )
                columns.append(node_type_column)
            space_id_column = PostgresColumn(
                name=f"{prop.id}_space_id",
                type=PrimitiveType.UUID,
                is_nullable=True,
                prop=prop,
            )
            columns.append(space_id_column)
            if prop.node_is_extensible:
                table_id_column = PostgresColumn(
                    name=f"{prop.id}_definition_id",
                    type=PrimitiveType.UUID,
                    is_nullable=True,
                    prop=prop,
                )
                columns.append(table_id_column)
        else:
            # regular column
            assert prop.primitive_type is not None, f"undetermined type for {prop!r}"
            column = PostgresColumn(
                name=str(prop.id),
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
                inner_name=f"unique_{prop.id}",
                type=PostgresIndexType.BTREE,
                columns=(str(prop.id),),
                is_unique=True,
            )
            indexes.append(index)

    # extras
    for index in node.__indexes__:
        assert not index.name or not index.name.startswith("destack_"), f"bad index name: {index!r}"
        extra_index = PostgresIndex.from_index(index)
        indexes.append(extra_index)

    table = PostgresTable(
        name=table_name,
        node_type=node.metatype,
        columns=tuple(columns),
        constraints=tuple(constraints),
        indexes=tuple(indexes),
    )
    return table


@cached({})
def get_builtin_schema(*store_keys: StoreKey) -> PostgresSchema:
    """Gets the builtin schema for the given Stores."""

    node_types: tuple[NodeType, ...] = tuple(
        {
            node_type
            for store_key in store_keys
            for node_type in NODE_TYPES_BY_PRIMARY_STORE_KEY[store_key]
        }
    )
    tables: list[PostgresTable] = [
        map_builtin_node_to_database_table(NODE_CLASS_BY_TYPE[node_type])
        for node_type in node_types
    ]
    return PostgresSchema(EXTENSIONS, tuple(tables))

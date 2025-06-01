from bench.language import (
    Node,
    NodeConstraint,
    NodeType,
    Organization,
    PrimitiveType,
    ScalarType,
    Type,
    TypeCardinality,
    User,
    to_type,
)


def test_to_type():
    """Parse values and types into Types."""

    # scalar values
    assert to_type(1) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    assert to_type(1.0) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.FLOAT64,
    )
    assert to_type(True) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.BOOLEAN,
    )

    # collection values
    assert to_type([1, 2, 3]) == Type(
        cardinality=TypeCardinality.LIST,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    assert to_type({"a": 1, "b": 2, "c": 3}) == Type(
        cardinality=TypeCardinality.MAP,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )

    # type annotations
    assert to_type(list[int]) == Type(
        cardinality=TypeCardinality.LIST,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    assert to_type(dict[str, Node]) == Type(
        cardinality=TypeCardinality.MAP,
        scalar_type=ScalarType.NODE,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    assert to_type(User | Organization) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE,
        node_constraint=NodeConstraint(
            node_types=[NodeType.USER, NodeType.ORGANIZATION],
        ),
    )

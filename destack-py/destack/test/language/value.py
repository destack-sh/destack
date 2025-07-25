import math

from destack.language import (
    Int8,
    Node,
    NodeType,
    Organization,
    PrimitiveType,
    ScalarType,
    Type,
    TypeCardinality,
    User,
)


def test_value_to_type():
    """Parse values into Types."""

    # Int64
    assert Type.infer(1) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    # Float64
    assert Type.infer(1.3) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.FLOAT64,
    )
    # Boolean
    assert Type.infer(True) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.BOOLEAN,
    )
    # String
    assert Type.infer("hello") == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.STRING,
    )

    # tuple[Int64, Boolean]
    assert Type.infer((1, True)) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.BOOLEAN,
            ),
        ],
    )
    # tuple[String, Int64, Float64]
    assert Type.infer(("hello", 42, math.pi)) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.FLOAT64,
            ),
        ],
    )

    # lists
    # list[Int64]
    assert Type.infer([1, 2, 3]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.INT64,
        ),
    )
    # list[String]
    assert Type.infer(["a", "b", "c"]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    # list[int] (type annotation)
    assert Type.infer(list[int]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.INT64,
        ),
    )

    # maps
    # dict[String, Int64]
    assert Type.infer({"a": 1, "b": 2, "c": 3}) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.INT64,
        ),
    )
    # dict[str, Node] (type annotation)
    assert Type.infer(dict[str, Node]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.NODE],
        ),
    )

    # nested collections
    # list[tuple[Int64, String]]
    assert Type.infer([(1, "a"), (2, "b")]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.TUPLE,
            element_types=[
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.INT64,
                ),
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.STRING,
                ),
            ],
        ),
    )
    # dict[String, list[Int64]]
    assert Type.infer({"nums": [1, 2, 3], "more": [4, 5]}) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.LIST,
            value_type=Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
        ),
    )


def test_annotation_to_type():
    """Parse type annotations into Types."""

    # Int8
    assert Type.infer(Int8) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT8,
    )

    # list[str] (type annotation)
    assert Type.infer(list[str]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )

    # tuple[int, str] (type annotation)
    assert Type.infer(tuple[int, str]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
        ],
    )

    # tuple[str, int, bool] (type annotation)
    assert Type.infer(tuple[str, int, bool]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.BOOLEAN,
            ),
        ],
    )

    # dict[str, int] (type annotation)
    assert Type.infer(dict[str, int]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.INT64,
        ),
    )

    # nested annotation collections
    # list[tuple[int, str]] (type annotation)
    assert Type.infer(list[tuple[int, str]]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.TUPLE,
            element_types=[
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.INT64,
                ),
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.STRING,
                ),
            ],
        ),
    )

    # dict[str, list[int]] (type annotation)
    assert Type.infer(dict[str, list[int]]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.LIST,
            value_type=Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
        ),
    )

    # tuple[list[str], dict[str, int]] (type annotation)
    assert Type.infer(tuple[list[str], dict[str, int]]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.LIST,
                value_type=Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.STRING,
                ),
            ),
            Type(
                cardinality=TypeCardinality.MAP,
                key_type=Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.STRING,
                ),
                value_type=Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.INT64,
                ),
            ),
        ],
    )

    # list[dict[str, tuple[int, bool]]] (type annotation)
    assert Type.infer(list[dict[str, tuple[int, bool]]]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.MAP,
            key_type=Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
            value_type=Type(
                cardinality=TypeCardinality.TUPLE,
                element_types=[
                    Type(
                        cardinality=TypeCardinality.SCALAR,
                        scalar_type=ScalarType.PRIMITIVE,
                        primitive_type=PrimitiveType.INT64,
                    ),
                    Type(
                        cardinality=TypeCardinality.SCALAR,
                        scalar_type=ScalarType.PRIMITIVE,
                        primitive_type=PrimitiveType.BOOLEAN,
                    ),
                ],
            ),
        ),
    )

    # node references
    # Node
    assert Type.infer(Node) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_REFERENCE,
        node_types=[NodeType.NODE],
    )
    assert Type.infer(Node, node_as_value=True) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_VALUE,
        node_types=[NodeType.NODE],
    )
    # User | Organization (union of node types)
    assert Type.infer(User | Organization) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_REFERENCE,
        node_types=[NodeType.USER, NodeType.ORGANIZATION],
    )

    # nested node references
    # list[User] (type annotation)
    assert Type.infer(list[User]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.USER],
        ),
    )

    # dict[str, User] (type annotation)
    assert Type.infer(dict[str, User]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.USER],
        ),
    )

    # tuple[User, Organization] (type annotation)
    assert Type.infer(tuple[User, Organization]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE_REFERENCE,
                node_types=[NodeType.USER],
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE_REFERENCE,
                node_types=[NodeType.ORGANIZATION],
            ),
        ],
    )

    # unions
    # int | str (type annotation)
    assert Type.infer(int | str) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.UNION,
        union_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
        ],
    )

    # int | str | bool (type annotation)
    assert Type.infer(int | str | bool) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.UNION,
        union_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.INT64,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.BOOLEAN,
            ),
        ],
    )

    # list[int | str] (type annotation)
    assert Type.infer(list[int | str]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.UNION,
            union_types=[
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.INT64,
                ),
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.STRING,
                ),
            ],
        ),
    )

    # dict[str, int | bool] (type annotation)
    assert Type.infer(dict[str, int | bool]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.UNION,
            union_types=[
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.INT64,
                ),
                Type(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.PRIMITIVE,
                    primitive_type=PrimitiveType.BOOLEAN,
                ),
            ],
        ),
    )

    # tuple[int | str, bool] (type annotation)
    assert Type.infer(tuple[int | str, bool]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.UNION,
                union_types=[
                    Type(
                        cardinality=TypeCardinality.SCALAR,
                        scalar_type=ScalarType.PRIMITIVE,
                        primitive_type=PrimitiveType.INT64,
                    ),
                    Type(
                        cardinality=TypeCardinality.SCALAR,
                        scalar_type=ScalarType.PRIMITIVE,
                        primitive_type=PrimitiveType.STRING,
                    ),
                ],
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.BOOLEAN,
            ),
        ],
    )

    # list[User | Organization] (type annotation)
    assert Type.infer(list[User | Organization]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.USER, NodeType.ORGANIZATION],
        ),
    )

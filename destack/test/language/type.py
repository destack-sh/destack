import math

from destack import (
    Boolean,
    Float64,
    Int8,
    Int64,
    Node,
    NodeType,
    Organization,
    PrimitiveType,
    ScalarType,
    String,
    Type,
    TypeCardinality,
    UInt32,
    User,
    invert_type,
)


def test_value_to_type():
    """Parse values into Types."""

    # Int64
    assert Type.infer(1) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    assert invert_type(Type.infer(1)) == Int64

    # Float64
    assert Type.infer(1.3) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.FLOAT64,
    )
    assert invert_type(Type.infer(1.3)) == Float64

    # Boolean
    assert Type.infer(True) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.BOOLEAN,
    )
    assert invert_type(Type.infer(True)) == Boolean

    # String
    assert Type.infer("hello") == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.STRING,
    )
    assert invert_type(Type.infer("hello")) == String

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
    assert invert_type(Type.infer((1, True))) == tuple[Int64, Boolean]

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
    assert invert_type(Type.infer(("hello", 42, math.pi))) == tuple[String, Int64, Float64]

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
    assert invert_type(Type.infer([1, 2, 3])) == list[Int64]

    # list[String]
    assert Type.infer(["a", "b", "c"]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    assert invert_type(Type.infer(["a", "b", "c"])) == list[String]

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
    assert invert_type(Type.infer({"a": 1, "b": 2, "c": 3})) == dict[String, Int64]

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
    assert invert_type(Type.infer({"nums": [1, 2, 3], "more": [4, 5]})) == dict[String, list[Int64]]


def test_annotation_to_type():
    """Parse type annotations into Types."""

    # Int8
    assert Type.infer(Int8) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT8,
    )
    assert invert_type(Type.infer(Int8)) == Int8

    # list[str]
    assert Type.infer(list[str]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    assert invert_type(Type.infer(list[str])) == list[String]

    # tuple[int, str]
    assert Type.infer(tuple[UInt32, str]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.UINT32,
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PrimitiveType.STRING,
            ),
        ],
    )
    assert invert_type(Type.infer(tuple[UInt32, str])) == tuple[UInt32, String]

    # tuple[str, int, bool]
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
    assert invert_type(Type.infer(tuple[str, int, bool])) == tuple[String, Int64, Boolean]

    # dict[str, int]
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
    assert invert_type(Type.infer(dict[str, int])) == dict[String, Int64]

    # nested annotation collections
    # list[tuple[int, str]]
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
    assert invert_type(Type.infer(list[tuple[int, str]])) == list[tuple[Int64, String]]

    # dict[str, list[int]]
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
    assert invert_type(Type.infer(list[tuple[int, str]])) == list[tuple[Int64, String]]

    # tuple[list[str], dict[str, int]]
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
    assert (
        invert_type(Type.infer(tuple[list[str], dict[str, int]]))
        == tuple[list[String], dict[String, Int64]]
    )

    # list[dict[str, tuple[int, bool]]]
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
    assert invert_type(Type.infer(Node)) == Node
    # User | Organization (union of node types)
    assert Type.infer(User | Organization) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_REFERENCE,
        node_types=[NodeType.USER, NodeType.ORGANIZATION],
    )
    assert invert_type(Type.infer(User | Organization)) == User | Organization

    # nested node references
    # list[User]
    assert Type.infer(list[User]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.USER],
        ),
    )
    assert invert_type(Type.infer(list[User])) == list[User]

    # dict[str, User]
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
    assert invert_type(Type.infer(dict[str, User])) == dict[String, User]

    # tuple[User, Organization]
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
    assert invert_type(Type.infer(tuple[User, Organization])) == tuple[User, Organization]

    # list[User | Organization]
    assert Type.infer(list[User | Organization]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_REFERENCE,
            node_types=[NodeType.USER, NodeType.ORGANIZATION],
        ),
    )
    assert invert_type(Type.infer(list[User | Organization])) == list[User | Organization]

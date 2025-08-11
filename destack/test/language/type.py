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
    assert Type.of(1) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT64,
    )
    assert invert_type(Type.of(1)) == Int64

    # Float64
    assert Type.of(1.3) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.FLOAT64,
    )
    assert invert_type(Type.of(1.3)) == Float64

    # Boolean
    assert Type.of(True) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.BOOLEAN,
    )
    assert invert_type(Type.of(True)) == Boolean

    # String
    assert Type.of("hello") == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.STRING,
    )
    assert invert_type(Type.of("hello")) == String

    # tuple[Int64, Boolean]
    assert Type.of((1, True)) == Type(
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
    assert invert_type(Type.of((1, True))) == tuple[Int64, Boolean]

    # tuple[String, Int64, Float64]
    assert Type.of(("hello", 42, math.pi)) == Type(
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
    assert invert_type(Type.of(("hello", 42, math.pi))) == tuple[String, Int64, Float64]

    # lists
    # list[Int64]
    assert Type.of([1, 2, 3]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.INT64,
        ),
    )
    assert invert_type(Type.of([1, 2, 3])) == list[Int64]

    # list[String]
    assert Type.of(["a", "b", "c"]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    assert invert_type(Type.of(["a", "b", "c"])) == list[String]

    # maps
    # dict[String, Int64]
    assert Type.of({"a": 1, "b": 2, "c": 3}) == Type(
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
    assert invert_type(Type.of({"a": 1, "b": 2, "c": 3})) == dict[String, Int64]

    # nested collections
    # list[tuple[Int64, String]]
    assert Type.of([(1, "a"), (2, "b")]) == Type(
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
    assert Type.of({"nums": [1, 2, 3], "more": [4, 5]}) == Type(
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
    assert invert_type(Type.of({"nums": [1, 2, 3], "more": [4, 5]})) == dict[String, list[Int64]]


def test_annotation_to_type():
    """Parse type annotations into Types."""

    # Int8
    assert Type.of(Int8) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.PRIMITIVE,
        primitive_type=PrimitiveType.INT8,
    )
    assert invert_type(Type.of(Int8)) == Int8

    # list[str]
    assert Type.of(list[str]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
    )
    assert invert_type(Type.of(list[str])) == list[String]

    # tuple[int, str]
    assert Type.of(tuple[UInt32, str]) == Type(
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
    assert invert_type(Type.of(tuple[UInt32, str])) == tuple[UInt32, String]

    # tuple[str, int, bool]
    assert Type.of(tuple[str, int, bool]) == Type(
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
    assert invert_type(Type.of(tuple[str, int, bool])) == tuple[String, Int64, Boolean]

    # dict[str, int]
    assert Type.of(dict[str, int]) == Type(
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
    assert invert_type(Type.of(dict[str, int])) == dict[String, Int64]

    # nested annotation collections
    # list[tuple[int, str]]
    assert Type.of(list[tuple[int, str]]) == Type(
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
    assert invert_type(Type.of(list[tuple[int, str]])) == list[tuple[Int64, String]]

    # dict[str, list[int]]
    assert Type.of(dict[str, list[int]]) == Type(
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
    assert invert_type(Type.of(list[tuple[int, str]])) == list[tuple[Int64, String]]

    # tuple[list[str], dict[str, int]]
    assert Type.of(tuple[list[str], dict[str, int]]) == Type(
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
        invert_type(Type.of(tuple[list[str], dict[str, int]]))
        == tuple[list[String], dict[String, Int64]]
    )

    # list[dict[str, tuple[int, bool]]]
    assert Type.of(list[dict[str, tuple[int, bool]]]) == Type(
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
    assert Type.of(Node) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_MOMENT,
        node_types=[NodeType.NODE],
    )
    assert invert_type(Type.of(Node)) == Node
    # User | Organization (union of node types)
    assert Type.of(User | Organization) == Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_MOMENT,
        node_types=[NodeType.USER, NodeType.ORGANIZATION],
    )
    assert invert_type(Type.of(User | Organization)) == User | Organization

    # nested node references
    # list[User]
    assert Type.of(list[User]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_MOMENT,
            node_types=[NodeType.USER],
        ),
    )
    assert invert_type(Type.of(list[User])) == list[User]

    # dict[str, User]
    assert Type.of(dict[str, User]) == Type(
        cardinality=TypeCardinality.MAP,
        key_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
        ),
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_MOMENT,
            node_types=[NodeType.USER],
        ),
    )
    assert invert_type(Type.of(dict[str, User])) == dict[String, User]

    # tuple[User, Organization]
    assert Type.of(tuple[User, Organization]) == Type(
        cardinality=TypeCardinality.TUPLE,
        element_types=[
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE_MOMENT,
                node_types=[NodeType.USER],
            ),
            Type(
                cardinality=TypeCardinality.SCALAR,
                scalar_type=ScalarType.NODE_MOMENT,
                node_types=[NodeType.ORGANIZATION],
            ),
        ],
    )
    assert invert_type(Type.of(tuple[User, Organization])) == tuple[User, Organization]

    # list[User | Organization]
    assert Type.of(list[User | Organization]) == Type(
        cardinality=TypeCardinality.LIST,
        value_type=Type(
            cardinality=TypeCardinality.SCALAR,
            scalar_type=ScalarType.NODE_MOMENT,
            node_types=[NodeType.USER, NodeType.ORGANIZATION],
        ),
    )
    assert invert_type(Type.of(list[User | Organization])) == list[User | Organization]

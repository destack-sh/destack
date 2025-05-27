from bench.language import (
    ColorType,
    EnumType,
    Node,
    PrimitiveType,
    TypeIn,
    parse_type_annotation,
)


def test_parse_type_annotation():
    assert parse_type_annotation(int) == TypeIn(
        cardinality="scalar",
        scalar_type="primitive",
        type=int,
        primitive_type=PrimitiveType.INT32,
    )

    assert parse_type_annotation(list[Node]) == TypeIn(
        cardinality="list",
        scalar_type="node",
        type=list[Node],
    )

    assert parse_type_annotation(dict[str, ColorType]) == TypeIn(
        cardinality="map",
        scalar_type="enum",
        type=dict[str, ColorType],
        key_type=TypeIn(
            cardinality="scalar",
            scalar_type="primitive",
            type=str,
            primitive_type=PrimitiveType.STRING,
        ),
        enum_type=EnumType.COLOR_TYPE,
    )

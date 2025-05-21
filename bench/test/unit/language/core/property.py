from bench.language.core import TypeAnnotation, parse_type_annotation


def test_parse_type_annotation():
    # nocheckin
    assert parse_type_annotation(int) == TypeAnnotation(kind="scalar", type=int)

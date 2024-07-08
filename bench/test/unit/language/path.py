from typing import List, Tuple

import pytest

from bench.language.path import PathError, PathTokenType, parse_path, render_path

# Assuming the necessary imports and definitions are in place
# from your_module import tokenize_path, PathToken, PathTokenType, PathSyntaxError


@pytest.mark.parametrize(
    ("input_path", "expected_tokens"),
    [
        ("/", [(PathTokenType.ROOT, None)]),
        ("@bench", [(PathTokenType.BENCH, "bench")]),
        (
            "/node1/node2",
            [
                (PathTokenType.ROOT, None),
                (PathTokenType.NODE, "node1"),
                (PathTokenType.NODE, "node2"),
            ],
        ),
        (
            "./current/node",
            [
                (PathTokenType.CURRENT, None),
                (PathTokenType.NODE, "current"),
                (PathTokenType.NODE, "node"),
            ],
        ),
        ("../parent", [(PathTokenType.PARENT, None), (PathTokenType.NODE, "parent")]),
        ("#unique_node", [(PathTokenType.UNIQUE_NODE, "unique_node")]),
        ("node.property", [(PathTokenType.NODE, "node"), (PathTokenType.PROPERTY, "property")]),
        (
            "@bench/node1/#unique2/node3.property",
            [
                (PathTokenType.BENCH, "bench"),
                (PathTokenType.NODE, "node1"),
                (PathTokenType.UNIQUE_NODE, "unique2"),
                (PathTokenType.NODE, "node3"),
                (PathTokenType.PROPERTY, "property"),
            ],
        ),
    ],
)
def test_parse_path(input_path: str, expected_tokens: List[Tuple[PathTokenType, str]]):
    # parse
    path = parse_path(input_path)
    assert len(path.tokens) == len(expected_tokens)
    for token, (expected_type, expected_name) in zip(path.tokens, expected_tokens):
        assert token.type == expected_type
        assert token.name == expected_name

    # roundtrip
    rendered_path = render_path(path)
    assert rendered_path == input_path


@pytest.mark.parametrize(
    "invalid_path",
    [
        "//",
        "node1//node2",
        "node1.property1.property2",
        "node1/node2.property/invalid",
        ".../invalid",
        "node@invalid",
        "node.invalid/",
    ],
)
def test_parse_path_invalid(invalid_path: str):
    with pytest.raises(PathError):
        parse_path(invalid_path)

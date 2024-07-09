from typing import List, Tuple

import pytest
from more_itertools import first

from bench.language.bench import Bench
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.path import PathError, PathTokenType, get_node, parse_path, render_path
from bench.language.session import Session
from bench.language.view import SpaceType, ViewType

# Assuming the necessary imports and definitions are in place
# from your_module import tokenize_path, PathToken, PathTokenType, PathSyntaxError


@pytest.mark.parametrize(
    ("input_path", "expected_tokens"),
    [
        ("/", [(PathTokenType.ROOT, None)]),
        ("@bench", [(PathTokenType.BENCH, "bench")]),
        (".property", [(PathTokenType.PROPERTY, "property")]),
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
        ("^unique_node", [(PathTokenType.UNIQUE_NODE, "unique_node")]),
        (">block", [(PathTokenType.SIBLING_NODE, "block")]),
        (
            "some/>block/^unique",
            [
                (PathTokenType.NODE, "some"),
                (PathTokenType.SIBLING_NODE, "block"),
                (PathTokenType.UNIQUE_NODE, "unique"),
            ],
        ),
        ("node.property", [(PathTokenType.NODE, "node"), (PathTokenType.PROPERTY, "property")]),
        (
            "@bench/node1/^unique2/node3.property",
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
        "^^node",
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


@pytest.fixture()
def mock_package(session: Session):
    # make bench
    bench = Bench(name="testbench", slug="test")
    branch = bench.branches.create(name="Main")
    package = branch.packages.create()
    session.parent = package  # patch in the session parent
    session._graph.update(session, _force_update_parent=True)

    # make space nodes
    space1 = package.spaces.create(name="Space1", type=SpaceType.DESKTOP)
    view1 = space1.views.create(name="Primary", type=ViewType.TAB)
    view2 = view1.views.create(name="Secondary", type=ViewType.TAB)
    view12 = view1.views.create(name="Timeline1", type=ViewType.PAGE)  # noqa: F841
    view22 = view2.views.create(name="Inspect1", type=ViewType.INSPECT)  # noqa: F841
    view23 = view2.views.create(name="Logs1", type=ViewType.INSPECT)  # noqa: F841

    # make page nodes
    page1 = package.blocks.create(name="Page1", type=BlockType.PAGE, is_page=True)
    page2 = package.blocks.create(name="Page2", type=BlockType.PAGE, is_page=True)
    page11 = page1.blocks.create(name="Page11", type=BlockType.PAGE, is_page=True)
    page21 = page2.blocks.create(name="Page21", type=BlockType.PAGE, is_page=True)
    choice111 = page11.blocks.create(name="Choice111", type=BlockType.CHOICE)
    field1111 = choice111.fields.append(Field.option("Field1111"))  # noqa: F841
    field1112 = choice111.fields.append(Field.option("Field1112"))  # noqa: F841
    code2111 = page21.blocks.create(name="Code2111", type=BlockType.CODE)  # noqa: F841

    return package


@pytest.mark.parametrize(
    ("scope_name", "path", "expected_node_name"),
    [
        # from root
        ("testbench", "@testbench", "testbench"),
        ("testbench", ".", "testbench"),
        ("testbench", "..", None),
        # from top level page
        ("Page1", "..", "testbench"),
        ("Page1", "Page11/Choice111", "Choice111"),
        ("Page1", "Page11/Choice111.Field1111", "Field1111"),
        ("Page1", "Page11/./Choice111/Field1111", "Field1111"),
        ("Page1", "Page11/./Choice111/Field1111/invalid", None),
        ("Page1", "Page11/Choice111/Field1111/invalid.property", None),
        # from nested page
        ("Page11", "..", "Page1"),
        ("Page11", "../..", "testbench"),
        ("Page11", "../Page11/../../Page2/Page21/Code2111", "Code2111"),
        ("Page21", "../>Page1/Page11/Choice111", "Choice111"),
    ],
)
def test_get_node(mock_package: Bench, scope_name: str, path: str, expected_node_name: str | None):
    scope = first(n for n in mock_package._graph.nodes if getattr(n, "name", None) == scope_name)
    node = get_node(path, scope)
    if expected_node_name:
        assert node is not None, f"node not found for path '{path}' in scope '{scope}'"
        assert getattr(node, "name") == expected_node_name
    else:
        assert node is None, f"unexpected node found for path '{path}' in scope '{scope}'"

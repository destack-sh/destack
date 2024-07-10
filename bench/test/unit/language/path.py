from typing import List, Tuple

import pytest
from more_itertools import first

from bench.language.bench import Bench
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.path import PathError, PathTokenType, get_node, parse_path, render_path
from bench.language.session import Session
from bench.language.step import Step, StepType

# Assuming the necessary imports and definitions are in place
# from your_module import tokenize_path, PathToken, PathTokenType, PathSyntaxError


@pytest.mark.parametrize(
    ("input_path", "expected_tokens"),
    [
        ("/", [(PathTokenType.ROOT, None)]),
        (".", [(PathTokenType.CURRENT, None)]),
        (
            "././.",
            [
                (PathTokenType.CURRENT, None),
                (PathTokenType.CURRENT, None),
                (PathTokenType.CURRENT, None),
            ],
        ),
        ("..", [(PathTokenType.PARENT, None)]),
        ("../..", [(PathTokenType.PARENT, None), (PathTokenType.PARENT, None)]),
        ("Node", [(PathTokenType.NAMED_NODE, "Node")]),
        (">Sibling", [(PathTokenType.SIBLING_NODE, "Sibling")]),
        ("~", [(PathTokenType.CONTAINING_NODE, None)]),
        ("~Container", [(PathTokenType.CONTAINING_NODE, "Container")]),
        ("^Unique", [(PathTokenType.UNIQUE_NODE, "Unique")]),
        (".property", [(PathTokenType.PROPERTY, "property")]),
        ("@bench", [(PathTokenType.BENCH, "bench")]),
        (
            "/node1/node2",
            [
                (PathTokenType.ROOT, None),
                (PathTokenType.NAMED_NODE, "node1"),
                (PathTokenType.NAMED_NODE, "node2"),
            ],
        ),
        (
            "./current/node",
            [
                (PathTokenType.CURRENT, None),
                (PathTokenType.NAMED_NODE, "current"),
                (PathTokenType.NAMED_NODE, "node"),
            ],
        ),
        ("../parent", [(PathTokenType.PARENT, None), (PathTokenType.NAMED_NODE, "parent")]),
        ("^unique_node", [(PathTokenType.UNIQUE_NODE, "unique_node")]),
        (">block", [(PathTokenType.SIBLING_NODE, "block")]),
        (
            "some/>block/^unique",
            [
                (PathTokenType.NAMED_NODE, "some"),
                (PathTokenType.SIBLING_NODE, "block"),
                (PathTokenType.UNIQUE_NODE, "unique"),
            ],
        ),
        (
            "node.property",
            [(PathTokenType.NAMED_NODE, "node"), (PathTokenType.PROPERTY, "property")],
        ),
        (
            "@bench/node1/^unique2/node3.property",
            [
                (PathTokenType.BENCH, "bench"),
                (PathTokenType.NAMED_NODE, "node1"),
                (PathTokenType.UNIQUE_NODE, "unique2"),
                (PathTokenType.NAMED_NODE, "node3"),
                (PathTokenType.PROPERTY, "property"),
            ],
        ),
        (
            "~Container/~/../^Unique",
            [
                (PathTokenType.CONTAINING_NODE, "Container"),
                (PathTokenType.CONTAINING_NODE, None),
                (PathTokenType.PARENT, None),
                (PathTokenType.UNIQUE_NODE, "Unique"),
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
        "^",
        "$",
        ">",
        "^^node",
        "node1//node2",
        "node1.property1.property2",
        "node1/node2.property/invalid",
        ".../invalid",
        "node@invalid",
        "node1/invalid!path",
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

    # make page nodes
    page1 = package.blocks.create(name="Page1", type=BlockType.PAGE, is_page=True)
    page2 = package.blocks.create(name="Page2", type=BlockType.PAGE, is_page=True)
    page11 = page1.blocks.create(name="Page11", type=BlockType.PAGE, is_page=True)
    page21 = page2.blocks.create(name="Page21", type=BlockType.PAGE, is_page=True)
    flow111 = page11.blocks.create(name="Flow111", type=BlockType.CHOICE)
    step1111 = flow111.steps.append(Step.new(StepType.START, "Step1111"))  # noqa: F841
    field1111 = flow111.fields.append(Field.input("Field1111", bool))  # noqa: F841
    step1112 = flow111.steps.append(Step.new(StepType.START, "Step1112"))
    step11121 = step1112.steps.append(Step.new(StepType.START, "Step11121"))  # noqa: F841
    flow211 = page21.blocks.create(name="Flow211", type=BlockType.CODE)
    step2111 = flow211.steps.append(Step.new(StepType.START, "Step2111"))  # noqa: F841
    step2112 = flow211.steps.append(Step.new(StepType.START, "Step2112"))  # noqa: F841

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
        ("Page1", "Page11/Flow111", "Flow111"),
        ("Page1", "Page11/Flow111.Field1111", "Field1111"),
        ("Page1", "Page11/./Flow111/Step1111", "Step1111"),
        ("Page1", "Page11/./Flow111/Step1111/invalid", None),
        ("Page1", "Page11/Flow111/Step1111/invalid.property", None),
        # from nested
        ("Page11", "..", "Page1"),
        ("Page11", "../..", "testbench"),
        ("Page11", "../Page11/../../Page2/Page21/Flow211", "Flow211"),
        ("Page21", "../>Page1/Page11/Flow111", "Flow111"),
        # container nodes
        ("Step1111", "~", "Flow111"),
        ("Step1111", "~/~", "Page11"),
        ("Step1111", "~Page11", "Page11"),
        ("Step1111", "~Page21", None),
        ("Step1111", "~Page1", "Page1"),
        ("Step1111", "~Page11/Flow111", "Flow111"),
        ("Page21", "~Page2", "Page2"),
        ("Page21", "~", "Page2"),
        # 'unique' nodes
        ("Page1", "^Page11", "Page11"),
        ("Page1", "^Flow111", None),
        ("Page11", "^Flow111", "Flow111"),
        ("Page11", "^Step1111", None),
        ("Page1", "^Step2112", None),
        ("Flow111", "^Step1111", "Step1111"),
        ("Flow111", "^Step1112", "Step1112"),
        ("Flow111", "^Step11121", "Step11121"),
    ],
)
def test_get_node(mock_package: Bench, scope_name: str, path: str, expected_node_name: str | None):
    scope = first(n for n in mock_package._graph.nodes if getattr(n, "name", None) == scope_name)
    node = get_node(scope, path)
    if expected_node_name:
        assert node is not None, f"node not found for path '{path}' in scope '{scope}'"
        assert getattr(node, "name") == expected_node_name
    else:
        assert node is None, f"unexpected node found for path '{path}' in scope '{scope}'"

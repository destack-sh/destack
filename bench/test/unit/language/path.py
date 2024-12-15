from typing import List, Tuple, cast

import pytest
from more_itertools import first

from bench.language.bench import Bench, Package, PackageType
from bench.language.block import FlowBlock
from bench.language.const import BlockType, NodeType
from bench.language.field import Field
from bench.language.file import File, FileKind, FileType
from bench.language.flow import Step, StepType
from bench.language.path import (
    PathError,
    PathLogicError,
    PathTokenType,
    get_node,
    get_path,
    parse_path,
    render_path,
)
from bench.language.session import Session

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
        ("Node", [(PathTokenType.CHILD, "Node")]),
        ("~", [(PathTokenType.CONTAINER, None)]),
        ("~Container", [(PathTokenType.CONTAINER, "Container")]),
        ("^Unique", [(PathTokenType.UNIQUE, "Unique")]),
        (".property", [(PathTokenType.FIELD, "property")]),
        ("@bench", [(PathTokenType.BENCH, "bench")]),
        (
            "/node1/node2",
            [
                (PathTokenType.ROOT, None),
                (PathTokenType.CHILD, "node1"),
                (PathTokenType.CHILD, "node2"),
            ],
        ),
        (
            "./C_RRENT_N_DE/node",
            [
                (PathTokenType.CURRENT, None),
                (PathTokenType.CHILD, "C_RRENT_N_DE"),
                (PathTokenType.CHILD, "node"),
            ],
        ),
        ("../parent", [(PathTokenType.PARENT, None), (PathTokenType.CHILD, "parent")]),
        ("^unique_node", [(PathTokenType.UNIQUE, "unique_node")]),
        ("^Choice.Option", [(PathTokenType.UNIQUE, "Choice"), (PathTokenType.FIELD, "Option")]),
        (
            "some/~block/^unique",
            [
                (PathTokenType.CHILD, "some"),
                (PathTokenType.CONTAINER, "block"),
                (PathTokenType.UNIQUE, "unique"),
            ],
        ),
        (
            "node.property",
            [(PathTokenType.CHILD, "node"), (PathTokenType.FIELD, "property")],
        ),
        (
            "@bench/node1/^unique2/node3.property",
            [
                (PathTokenType.BENCH, "bench"),
                (PathTokenType.CHILD, "node1"),
                (PathTokenType.UNIQUE, "unique2"),
                (PathTokenType.CHILD, "node3"),
                (PathTokenType.FIELD, "property"),
            ],
        ),
        (
            "~Container/~/../^Unique",
            [
                (PathTokenType.CONTAINER, "Container"),
                (PathTokenType.CONTAINER, None),
                (PathTokenType.PARENT, None),
                (PathTokenType.UNIQUE, "Unique"),
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


@pytest.fixture
def mock_package(session: Session):
    bench = Bench(name="bench1", slug="bench")
    package = bench.packages.create(type=PackageType.ROOT, name="Main", slug="main")
    session.parent = package  # patch in the session parent
    session._graph.update(session, _force_update_parent=True)
    return package


@pytest.fixture
def mock_package_populated(session: Session):
    # make bench
    bench = Bench(name="bench1", slug="bench")
    package = bench.packages.create(type=PackageType.ROOT, name="Main", slug="main")
    session.parent = package  # patch in the session parent
    session._graph.update(session, _force_update_parent=True)

    # page nodes
    page1 = package.blocks.create(name="Page1", type=BlockType.PAGE)
    page2 = package.blocks.create(name="Page2", type=BlockType.PAGE)
    page11 = page1.blocks.create(name="Page11", type=BlockType.PAGE)
    page21 = page2.blocks.create(name="Page21", type=BlockType.PAGE)
    flow111 = cast(FlowBlock, page11.blocks.create(name="Flow111", type=BlockType.FLOW))
    step1111 = flow111.steps.append(Step.new(StepType.START, "Step1111"))  # noqa: F841
    field1111 = flow111.fields.append(Field.input("Field1111", bool))  # noqa: F841
    step1112 = flow111.steps.append(Step.new(StepType.START, "Step1112"))
    step11121 = step1112.steps.append(Step.new(StepType.START, "Step11121"))  # noqa: F841
    flow211 = cast(FlowBlock, page21.blocks.create(name="Flow211", type=BlockType.FLOW))
    step2111 = flow211.steps.append(Step.new(StepType.START, "Step2111"))  # noqa: F841
    step2112 = flow211.steps.append(Step.new(StepType.START, "Step2112"))  # noqa: F841
    step2112_t_st = flow211.steps.append(Step.new(StepType.START, "Step2112 TÖST"))  # noqa: F841
    choice212 = page21.blocks.create(  # noqa: F841
        name="Choice212",
        type=BlockType.CHOICE,
        fields=[Field.option("Option2121"), Field.option("Option2122"), Field.option("Option2123")],
    )

    return package


@pytest.mark.parametrize(
    ("scope_name", "path", "expected_node_name"),
    [
        # from root
        ("bench1", "@bench1", "bench1"),
        ("bench1", ".", "bench1"),
        ("bench1", "..", None),
        ("bench1", "@bench1/Page1", "Page1"),
        # from top level page
        ("Page1", "..", "bench1"),
        ("Page1", "Page11/Flow111", "Flow111"),
        ("Page1", "Page11/Flow111.Field1111", "Field1111"),
        ("Page1", "Page11/./Flow111/Step1111", "Step1111"),
        ("Page1", "Page11/./Flow111/Step1111/invalid", None),
        ("Page1", "Page11/Flow111/Step1111/invalid.property", None),
        # from nested
        ("Page11", "..", "Page1"),
        ("Page11", "../..", "bench1"),
        ("Page11", "../Page11/../../Page2/Page21/Flow211", "Flow211"),
        ("Page21", "../../Page1/Page11/Flow111", "Flow111"),
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
        ("Flow211", "^Step2112 TÖST", "Step2112 TÖST"),
        ("Flow211", "^Step2112_T_ST", "Step2112 TÖST"),
    ],
)
def test_get_node(
    mock_package_populated: Bench, scope_name: str, path: str, expected_node_name: str | None
):
    scope = first(
        n for n in mock_package_populated._graph.nodes if getattr(n, "name", None) == scope_name
    )
    node = get_node(scope, path)
    if expected_node_name:
        assert node is not None, f"node not found for path '{path}' in scope '{scope}'"
        assert getattr(node, "name") == expected_node_name
    else:
        assert node is None, f"unexpected node found for path '{path}' in scope '{scope}'"


@pytest.mark.parametrize(
    ("scope_name", "node_name", "expected_path"),
    [
        # to root
        ("bench1", "bench1", "@bench1"),
        ("Page21", "bench1", "@bench1"),
        ("Step2112", "bench1", "@bench1"),
        # from root
        ("bench1", "Page11", "/Page1/Page11"),
        ("bench1", "Flow111", "/Page1/Page11/Flow111"),
        ("bench1", "Field1111", "/Page1/Page11/Flow111.Field1111"),
        # inner
        ("Page1", "Page1", "."),
        ("Page1", "Page2", "/Page2"),
        ("Page1", "Page11", "Page11"),
        ("Page1", "Step1111", "Page11/Flow111/Step1111"),
        ("Page21", "Step1111", "/Page1/Page11/Flow111/Step1111"),
        ("Page11", "Page1", "~Page1"),
        ("Step1111", "Page1", "~Flow111/~Page11/~Page1"),
        ("Flow211", "Option2121", "^Choice212.Option2121"),
    ],
)
def test_get_path(
    mock_package_populated: Bench, scope_name: str, node_name: str, expected_path: str
):
    package = mock_package_populated
    scope = first(
        n
        for n in package._graph.nodes
        if getattr(n, "name", None) == scope_name and n.metatype != NodeType.BENCH
    )
    node = first(
        n
        for n in package._graph.nodes
        if getattr(n, "name", None) == node_name and n.metatype != NodeType.BENCH
    )

    # get path
    path = get_path(scope, node)
    rendered_path = render_path(path)
    assert rendered_path == expected_path

    # roundtrip
    parsed_path = parse_path(expected_path)
    assert parsed_path == path
    parsed_node = get_node(scope, expected_path)
    assert parsed_node is node


def test_shadow_node(session: Session, mock_package: Package):
    """Siblings before descendants before ancestors. See get_unique_node."""
    package = mock_package

    # shadowing nodes
    page3 = package.blocks.create(name="Page3", type=BlockType.PAGE)
    choice31 = page3.blocks.create(name="Choice31", type=BlockType.CHOICE)
    page33 = package.blocks.create(name="Page33", type=BlockType.PAGE)
    choice331 = page33.blocks.create(name="Choice331", type=BlockType.CHOICE)
    code332 = page33.blocks.create(name="Code332", type=BlockType.ACTION)
    code332_output1 = code332.fields.append(Field.output("Choice331", choice331))  # noqa: F841
    code332_output3 = code332.fields.append(Field.output("Choice31", choice31))

    assert get_node(code332, "^Choice331") is choice331  # sibling before descendants
    assert get_node(code332, "^Choice31") is code332_output3  # descendants before ancestors


def test_unnamed_node(session: Session, mock_package: Package):
    """Cannot create path to an unnamed node (like a File)."""
    package = mock_package

    assert package.bench
    file = File(
        parent=package.bench.main_drive,
        kind=FileKind.DRIVE,
        title="myfile.txt",
        type=FileType.TEXT,
        mime_type="text/plain",
        size=1024,
    )
    session._create(file)

    with pytest.raises(PathLogicError):
        get_path(package, file)

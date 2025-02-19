from typing import List, Tuple

import pytest
from more_itertools import first

from bench.language import (
    Action,
    ActionType,
    Bench,
    Choice,
    Context,
    Field,
    Flow,
    ModelFamily,
    Option,
    Package,
    PackageType,
    PathElementType,
    PathError,
    RunOptions,
    Session,
    TextOptions,
    evaluate_path,
    get_node,
    get_path,
    parse_path,
    path,
    render_path,
)


@pytest.mark.parametrize(
    ("input_path", "expected_elements"),
    [
        ("/", [(PathElementType.ROOT, None)]),
        (".", [(PathElementType.CURRENT, None)]),
        (
            "././.",
            [
                (PathElementType.CURRENT, None),
                (PathElementType.CURRENT, None),
                (PathElementType.CURRENT, None),
            ],
        ),
        ("..", [(PathElementType.PARENT, None)]),
        ("../..", [(PathElementType.PARENT, None), (PathElementType.PARENT, None)]),
        ("Node", [(PathElementType.CHILD, "Node")]),
        ("~", [(PathElementType.CONTAINER, None)]),
        ("~Container", [(PathElementType.CONTAINER, "Container")]),
        ("^Unique", [(PathElementType.CLOSEST, "Unique")]),
        (".property", [(PathElementType.ATTRIBUTE, "property")]),
        ("@bench", [(PathElementType.BENCH, "bench")]),
        (
            "/node1/node2",
            [
                (PathElementType.ROOT, None),
                (PathElementType.CHILD, "node1"),
                (PathElementType.CHILD, "node2"),
            ],
        ),
        (
            "./C_RRENT_N_DE/node",
            [
                (PathElementType.CURRENT, None),
                (PathElementType.CHILD, "C_RRENT_N_DE"),
                (PathElementType.CHILD, "node"),
            ],
        ),
        ("../parent", [(PathElementType.PARENT, None), (PathElementType.CHILD, "parent")]),
        ("^unique_node", [(PathElementType.CLOSEST, "unique_node")]),
        (
            "^Choice.Option",
            [(PathElementType.CLOSEST, "Choice"), (PathElementType.ATTRIBUTE, "Option")],
        ),
        (
            "some/~block/^unique",
            [
                (PathElementType.CHILD, "some"),
                (PathElementType.CONTAINER, "block"),
                (PathElementType.CLOSEST, "unique"),
            ],
        ),
        (
            "node.property",
            [(PathElementType.CHILD, "node"), (PathElementType.ATTRIBUTE, "property")],
        ),
        (
            "@bench/node1/^unique2/node3.property",
            [
                (PathElementType.BENCH, "bench"),
                (PathElementType.CHILD, "node1"),
                (PathElementType.CLOSEST, "unique2"),
                (PathElementType.CHILD, "node3"),
                (PathElementType.ATTRIBUTE, "property"),
            ],
        ),
        (
            "~Container/~/../^Unique",
            [
                (PathElementType.CONTAINER, "Container"),
                (PathElementType.CONTAINER, None),
                (PathElementType.PARENT, None),
                (PathElementType.CLOSEST, "Unique"),
            ],
        ),
    ],
)
def test_parse_path(input_path: str, expected_elements: List[Tuple[PathElementType, str]]):
    # parse
    path = parse_path(input_path)
    assert len(path.elements) == len(expected_elements)
    for element, (expected_type, expected_name) in zip(path.elements, expected_elements):
        assert element.type == expected_type
        assert element.name == expected_name

    # roundtrip
    rendered_path = render_path(path)
    assert rendered_path == input_path


@pytest.mark.parametrize(
    "invalid_path",
    [
        "//",
        "^",
        ">",
        ":",
        "^^node",
        "node1//node2",
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
    session.parent = bench  # patch in the session parent
    package = bench.packages.create(type=PackageType.MAIN, name="Main", slug="main")
    session._graph.update(session, _force_update_parent=True)
    return package


@pytest.fixture
def mock_package_populated(session: Session):
    # make bench
    bench = Bench(name="bench1", slug="bench1")
    session.parent = bench  # patch in the session parent
    package = bench.packages.create(type=PackageType.MAIN, name="Main", slug="main")
    session._graph.update(session, _force_update_parent=True)
    bench.main_package = package
    side_package = bench.packages.create(type=PackageType.SIDE, name="Side", slug="side")  # noqa: F841

    # page nodes
    page1 = package.pages.create(name="Page1")
    page2 = package.pages.create(name="Page2")
    page11 = page1.pages.create(name="Page11")
    page21 = page2.pages.create(name="Page21")
    flow111 = Flow.new("Flow111")
    page11.append(flow111)
    action1111 = flow111.actions.append(Action.new(ActionType.START, "Action1111"))  # noqa: F841
    field1111 = flow111.fields.append(Field.input("Field1111", bool))  # noqa: F841
    action1112 = flow111.actions.append(Action.new(ActionType.START, "Action1112"))
    action11121 = action1112.actions.append(Action.new(ActionType.START, "Action11121"))  # noqa: F841
    flow211 = Flow.new("Flow211")
    page21.append(flow211)
    action2111 = flow211.actions.append(Action.new(ActionType.START, "Action2111"))  # noqa: F841
    action2112 = flow211.actions.append(Action.new(ActionType.START, "Action2112"))  # noqa: F841
    action2112_t_st = flow211.actions.append(Action.new(ActionType.START, "Action2112 TÖST"))  # noqa: F841
    choice212 = Choice.new(
        "Choice212",
        fields=[Option.new("Option2121"), Option.new("Option2122"), Option.new("Option2123")],
    )
    page21.append(choice212)

    return package


@pytest.mark.skip("NOTE: revisit string Paths?")
@pytest.mark.parametrize(
    ("scope_name", "path", "expected_node_name"),
    [
        # from root
        ("bench1", "@bench1", "bench1"),
        ("bench1", ".", "bench1"),
        ("bench1", "..", None),
        ("bench1", "@bench1/Page1", "Page1"),
        ("bench1", "@bench1:main", "Main"),
        ("bench1", "@bench1:side", "Side"),
        # from top level page
        ("Page1", "..", "bench1"),
        ("Page1", "Page11/Flow111", "Flow111"),
        ("Page1", "Page11/Flow111.Field1111", "Field1111"),
        ("Page1", "Page11/./Flow111/Action1111", "Action1111"),
        ("Page1", "Page11/./Flow111/Action1111/invalid", None),
        ("Page1", "Page11/Flow111/Action1111/invalid.property", None),
        # from nested
        ("Page11", "..", "Page1"),
        ("Page11", "../..", "bench1"),
        ("Page11", "../Page11/../../Page2/Page21/Flow211", "Flow211"),
        ("Page21", "../../Page1/Page11/Flow111", "Flow111"),
        # container nodes
        ("Action1111", "~", "Flow111"),
        ("Action1111", "~/~", "Page11"),
        ("Action1111", "~Page11", "Page11"),
        ("Action1111", "~Page21", None),
        ("Action1111", "~Page1", "Page1"),
        ("Action1111", "~Page11/Flow111", "Flow111"),
        ("Page21", "~Page2", "Page2"),
        ("Page21", "~", "Page2"),
        # 'unique' nodes
        ("Page1", "^Page11", "Page11"),
        ("Page1", "^Flow111", None),
        ("Page11", "^Flow111", "Flow111"),
        ("Page11", "^Action1111", None),
        ("Page1", "^Action2112", None),
        ("Flow111", "^Action1111", "Action1111"),
        ("Flow111", "^Action1112", "Action1112"),
        ("Flow111", "^Action11121", "Action11121"),
        ("Flow211", "^Action2112 TÖST", "Action2112 TÖST"),
        ("Flow211", "^Action2112_T_ST", "Action2112 TÖST"),
    ],
)
def test_get_node(
    mock_package_populated: Bench, scope_name: str, path: str, expected_node_name: str | None
):
    context = Context()
    scope = first(
        n for n in mock_package_populated._graph.nodes if getattr(n, "name", None) == scope_name
    )
    node = get_node(scope, context, path)
    if expected_node_name:
        assert node is not None, f"node not found for path '{path}' in scope '{scope}'"
        assert getattr(node, "name") == expected_node_name
    else:
        assert node is None, f"unexpected node found for path '{path}' in scope '{scope}'"


@pytest.mark.skip("NOTE: revisit string Paths?")
@pytest.mark.parametrize(
    ("scope_name", "node_name", "expected_path"),
    [
        # to root
        ("bench1", "bench1", "@bench1"),
        ("Page21", "bench1", "@bench1"),
        ("Action2112", "bench1", "@bench1"),
        # from root
        ("bench1", "Page11", "@bench1/Page1/Page11"),
        ("bench1", "Flow111", "@bench1/Page1/Page11/Flow111"),
        ("bench1", "Field1111", "@bench1/Page1/Page11/Flow111.Field1111"),
        # inner
        ("Page1", "Page1", "."),
        ("Page1", "Page2", "@bench1/Page2"),
        ("Page1", "Page11", "Page11"),
        ("Page1", "Action1111", "Page11/Flow111/Action1111"),
        ("Page21", "Action1111", "@bench1/Page1/Page11/Flow111/Action1111"),
        ("Page11", "Page1", "~Page1"),
        ("Action1111", "Page1", "~Flow111/~Page11/~Page1"),
        ("Flow211", "Option2121", "^Choice212.Option2121"),
    ],
)
def test_get_path(
    mock_package_populated: Bench, scope_name: str, node_name: str, expected_path: str
):
    package = mock_package_populated
    scope = first(n for n in package._graph.nodes if getattr(n, "name", None) == scope_name)
    node = first(n for n in package._graph.nodes if getattr(n, "name", None) == node_name)
    context = Context()

    # get path
    path = get_path(scope, node)
    rendered_path = render_path(path)
    assert rendered_path == expected_path

    # roundtrip
    parsed_path = parse_path(expected_path)
    assert parsed_path == path
    parsed_node = get_node(scope, context, expected_path)
    assert parsed_node is node


def test_path_evaluate_attribute(session: Session, mock_package: Package):
    """Evaluate nested path Attributes on a Node."""
    page1 = mock_package.pages.create(name="Page1")
    flow1 = Flow.new(
        "Flow1",
        run_options=RunOptions(max_attempts=2, text_options=TextOptions(temperature=0.5)),
    )
    page1.append(flow1)
    action1 = flow1.actions.create(  # noqa: F841
        name="Action1",
        type=ActionType.CODE,
        run_options=RunOptions(model_family=ModelFamily.META_LLAMA),
    )

    # relative to scope
    p = path(
        Flow.get_property("run_options"),
        RunOptions.get_property("text_options"),
        TextOptions.get_property("temperature"),
    )
    assert evaluate_path(flow1, flow1, Context(), p) == 0.5

    # absolute node path
    p = path(
        flow1,
        Flow.get_property("run_options"),
        RunOptions.get_property("text_options"),
        TextOptions.get_property("temperature"),
    )
    assert evaluate_path(flow1, flow1, Context(), p) == 0.5

    # combind relative
    p = path(
        "Action1",
        Action.get_property("run_options"),
        RunOptions.get_property("model_family"),
    )
    assert evaluate_path(flow1, flow1, Context(), p) == ModelFamily.META_LLAMA

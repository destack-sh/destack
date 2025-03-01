import functools
import inspect
import textwrap
from typing import Any, Callable, Mapping, assert_never, cast
from uuid import UUID

from bench.language import (
    Action,
    ActionType,
    Aliasing,
    Block,
    BlockType,
    BuiltinObject,
    Choice,
    Class,
    ComputedValue,
    ComputedValueMode,
    CustomObject,
    Field,
    Flow,
    LinkType,
    Message,
    MessageType,
    Node,
    NodeReference,
    Option,
    Package,
    Page,
    PathElementType,
    Property,
    Renderer,
    RenderOptions,
    Run,
    Session,
    constraint,
    format_code,
    path,
    render_expression,
    to_type,
)
from bench.runtime.code import BUILTIN_GLOBALS, STATIC_CODE_GLOBALS


def _render_test(func: Callable[[Any, Any], Mapping[str, Any]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    def _render_and_check(
        func: Callable[[Any, Any], Mapping[str, BuiltinObject | CustomObject | Property]],
        session: Session,
        package: Package,
    ) -> None:
        """Common logic for rendering and checking rendered code matches original."""
        # (line length 96 because it's 100 - 4 for the method indent here)

        def _render(defns: Mapping[str, Any]):
            options = RenderOptions(scope=package, aliasing=Aliasing(), format=True, line_length=96)
            renderer = Renderer(options)
            statements: list[str] = []
            for obj in defns.values():
                if isinstance(obj, Node):
                    renderer.aliasing.add(obj)
            current_nodes: list[Node] = []
            for name, obj in defns.items():
                if isinstance(obj, Node):
                    current_nodes.append(obj)
                else:
                    if current_nodes:
                        statements.append(renderer.render_statement(*current_nodes))
                        current_nodes = []

                    if isinstance(obj, (BuiltinObject, Property, CustomObject)):
                        statements.append(f"{name} = {renderer.render_expression(obj)}")
                    else:
                        assert_never(obj)
            if current_nodes:
                statements.append(renderer.render_statement(*current_nodes))
            # combine & format
            rendered = "\n".join(statements)
            if options.format:
                rendered = format_code(rendered, line_length=options.line_length)
            return rendered.strip()

        original_defns = func(session, package)

        # render
        rendered = _render(original_defns)

        # should match source (minus last line)
        source = inspect.getsource(func)
        source = "\n".join(source.splitlines()[2:]).split("return")[0]  # remove return
        source = textwrap.dedent(source).strip()
        assert rendered == source

        # eval
        glbls = {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS}
        glbls_tmp = {**glbls}
        exec(rendered, glbls_tmp)
        rendered_defns = {
            name: obj
            for name, obj in glbls_tmp.items()
            if name not in glbls and name not in ("__builtins__", "__doc__", "__file__", "__name__")
        }

        # check that all definitions are equal
        identity_map: dict[UUID, NodeReference] = {}
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            if isinstance(original_obj, Node):
                assert isinstance(rendered_obj, Node)
                identity_map[original_obj.ck] = rendered_obj.to_ref()
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            if isinstance(original_obj, Property):
                assert (
                    original_obj.component == rendered_obj.component
                    and original_obj.id == rendered_obj.id
                )
            elif isinstance(original_obj, CustomObject):
                assert original_obj.equals(rendered_obj, identity_map=identity_map)
            elif isinstance(original_obj, BuiltinObject):
                assert cast(BuiltinObject, original_obj).equals(
                    rendered_obj, identity_map=identity_map
                )
            else:
                assert_never(original_obj)

        # render again from evaluated
        rendered_again = _render(rendered_defns)
        assert rendered == rendered_again

    @functools.wraps(func)
    def _inner(session: Session, package: Package):
        _render_and_check(func, session, package)
        return func(session, package)

    return _inner


@_render_test
def test_render_property(session: Session, package: Package):
    prop_1 = Node.get_property("id")
    prop_2 = Page.get_property("name")
    prop_3 = Run.get_property("outputs")
    return {"prop_1": prop_1, "prop_2": prop_2, "prop_3": prop_3}


@_render_test
def test_render_type_in(session: Session, package: Package):
    type_1 = to_type(int)
    type_2 = to_type(str)
    type_3 = Node.partial_type()
    return {"type_1": type_1, "type_2": type_2, "type_3": type_3}


@_render_test
def test_render_path(session: Session, package: Package):
    path_1 = path(PathElementType.RUN, Run.get_property("inputs"))
    path_2 = path(PathElementType.RUN, Run.get_property("inputs"))
    return {"path_1": path_1, "path_2": path_2}


@_render_test
def test_render_partial_object(session: Session, package: Package):
    PartialPage1 = Page.partial(name="PartialPage1")
    MessageType1 = Class.new(
        "MessageType1",
        Field.member("Field1", int),
        Field.member("Field2", str),
        Field.member("Field3", bool),
    )
    PatialMessage1 = Message.partial(
        MessageType.REGULAR, clazz=MessageType1, Field1=17, Field2="hello!"
    )
    return {
        "PartialPage1": PartialPage1,
        "MessageType1": MessageType1,
        "PatialMessage1": PatialMessage1,
    }


@_render_test
def test_render_bad_names(session: Session, package: Package):
    _F_1 = Field.variable("-F_1", str)
    Page_with_Spa_se = Page.new("Page with Spa se")
    return {"_F_1": _F_1, "Page_with_Spa_se": Page_with_Spa_se}


@_render_test
def test_render_choice_block(session: Session, package: Package):
    ShapeType = Choice.new(
        "ShapeType", Option.new("Circle"), Option.new("Square"), Option.new("Triangle")
    )
    return {"ShapeType": ShapeType}


@_render_test
def test_render_message_block(session: Session, package: Package):
    ShapeType = Choice.new(
        "ShapeType", Option.new("Circle"), Option.new("Square"), Option.new("Triangle")
    )
    Shape = Class.new("Shape", Field.member("kind", ShapeType), Field.member("is_cool", bool))
    return {"ShapeType": ShapeType, "Shape": Shape}


@_render_test
def test_render_field_with_constraint(session: Session, package: Package):
    Field1 = Field.input(
        "Field1", int, constraint=constraint(min_value=1.0, max_value=10.0, step_value=2.0)
    )
    return {"Field1": Field1}


@_render_test
def test_render_create_action(session: Session, package: Package):
    Action1 = Action.new(ActionType.CREATE, "Action1", node_partial=Block.partial(BlockType.PAGE))
    return {"Action1": Action1}


@_render_test
def test_render_flow_simple(session: Session, package: Package):
    Flow1 = Flow.new("Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Forward1 = Start.connect(LinkType.MANUAL, Complete, "Forward1")
    return {"Flow1": Flow1, "Start": Start, "Complete": Complete, "Forward1": Forward1}


@_render_test
def test_render_flow_computed_value(session: Session, package: Package):
    Flow1 = Flow.new(
        "Flow1",
        fields=[
            Field.input("Input1", int),
            Field.input("Input2", int),
            Field.input("Input3", int),
            Field.output("Output1", int),
            Field.output("Output2", int),
            Field.output("Output3", int),
        ],
    )
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(
        ActionType.COMPLETE,
        "Complete",
        computed_values=[
            ComputedValue.new(
                target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output1),
                source=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input1),
            ),
            ComputedValue.new(
                target=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Output2),
                source=(PathElementType.RUN, Run.get_property("inputs"), Flow1.fields.Input2),
                mode=ComputedValueMode.IF_SOURCE_SET,
            ),
        ],
    )
    Flow1.actions.extend(Start, Complete)
    Forward1 = Start.connect(LinkType.MANUAL, Complete, "Forward1")
    return {"Flow1": Flow1, "Start": Start, "Complete": Complete, "Forward1": Forward1}


#
# Other
#


def test_render_simple_choice_option_ref(session: Session, package: Package):
    """Rendered node ref in sibling scope should be simplified"""
    Page1 = package.pages.create(name="Page")
    Choice1 = Choice.new(
        "Choice1",
        Option.new("Option1"),
        Option.new("Option2"),
        Option.new("Option3"),
    )
    Page1.append(Choice1)
    rendered_option = render_expression(
        Choice1.options.Option2,
        options=RenderOptions(scope=Page1, aliasing=Aliasing()),
        as_ref=True,
    )
    assert rendered_option == "Choice1.options.Option2"

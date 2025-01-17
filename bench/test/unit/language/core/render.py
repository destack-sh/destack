import functools
import inspect
import textwrap
from typing import Any, Callable, Mapping, cast
from uuid import UUID

from bench.language import (
    Action,
    ActionType,
    Block,
    BlockType,
    BuiltinObject,
    ComputedValue,
    ComputedValueMode,
    Field,
    File,
    FileKind,
    FileType,
    Node,
    NodeReference,
    Package,
    PathElementType,
    PipeType,
    Property,
    RenderOptions,
    Run,
    Session,
    Struct,
    View,
    ViewType,
    constraint,
    md,
    path,
    render,
    render_expression,
    render_expressions,
    to_type,
)
from bench.runtime.code import BUILTIN_GLOBALS, STATIC_CODE_GLOBALS


def _render_and_check(
    func: Callable[[Any, Any], Mapping[str, BuiltinObject]],
    session: Session,
    package: Package,
    render_func: Callable,
) -> None:
    """Common logic for rendering and checking rendered code matches original."""
    # (line length 96 because it's 100 - 4 for the method indent here)
    render_options = RenderOptions(scope=package, format=True, format_line_length=96)
    original_defns = func(session, package)

    # render
    rendered = render_func(original_defns, options=render_options)

    # should match source (minus last line)
    source = inspect.getsource(func)
    source = "\n".join(source.splitlines()[2:-1])  # remove return
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
            assert original_obj == rendered_obj
        else:
            assert cast(BuiltinObject, rendered_obj).equals(original_obj, identity_map=identity_map)

    # render again from evaluated
    rendered_again = render_func(rendered_defns, options=render_options)
    assert rendered == rendered_again


def _rendered_expression(func: Callable[[Any, Any], Mapping[str, Struct]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    @functools.wraps(func)
    def _inner(session: Session, package: Package):
        _render_and_check(func, session, package, render_expressions)
        return func(session, package)

    return _inner


def _rendered_statement(func: Callable[[Any, Any], Mapping[str, BuiltinObject]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    @functools.wraps(func)
    def _inner(session: Session, package: Package):
        def render_func(defns, options):
            return render(*defns.values(), options=options)

        _render_and_check(func, session, package, render_func)
        return func(session, package)

    return _inner


#
# Structs
#


@_rendered_expression
def test_render_property(session: Session, package: Package):
    prop_1 = Node.get_property("id")
    prop_2 = Block.get_property("name")
    prop_3 = Run.get_property("outputs")
    return {"prop_1": prop_1, "prop_2": prop_2, "prop_3": prop_3}


@_rendered_expression
def test_render_type_in(session: Session, package: Package):
    type_1 = to_type(int)
    type_2 = to_type(str)
    type_3 = Node.partial_type()
    return {"type_1": type_1, "type_2": type_2, "type_3": type_3}


@_rendered_expression
def test_render_path(session: Session, package: Package):
    path_1 = path(PathElementType.RUN, Run.get_property("inputs"))
    path_2 = path(PathElementType.RUN, Run.get_property("inputs"))
    return {"path_1": path_1, "path_2": path_2}


#
# Nodes
#


@_rendered_statement
def test_render_bad_names(session: Session, package: Package):
    _F_1 = Field.variable("-F_1", str)
    Block_with_Spa_se = Block.new(BlockType.MESSAGE, "Block with Spa'se")
    return {"_F_1": _F_1, "Block_with_Spa_se": Block_with_Spa_se}


@_rendered_statement
def test_render_choice_block(session: Session, package: Package):
    ShapeType = Block.new(
        BlockType.CHOICE,
        "ShapeType",
        fields=[Field.option("Circle"), Field.option("Square"), Field.option("Triangle")],
    )
    return {"ShapeType": ShapeType}


@_rendered_statement
def test_render_message_block(session: Session, package: Package):
    ShapeType = Block.new(
        BlockType.CHOICE,
        "ShapeType",
        fields=[Field.option("Circle"), Field.option("Square"), Field.option("Triangle")],
    )
    Shape = Block.new(
        BlockType.MESSAGE,
        "Shape",
        fields=[Field.member("kind", ShapeType), Field.member("is_cool", bool)],
    )
    return {"ShapeType": ShapeType, "Shape": Shape}


@_rendered_statement
def test_render_variable_block(session: Session, package: Package):
    Variable1 = Block.new(BlockType.VARIABLE, "Variable1", value_type=to_type(int), value=1)
    return {"Variable1": Variable1}


@_rendered_statement
def test_render_view_block(session: Session, package: Package):
    View_1 = Block.new(BlockType.VIEW, "View 1")
    Button1 = View.new(ViewType.BUTTON, "Button1", node=View_1)
    View_1.views.append(Button1)
    return {"View_1": View_1, "Button1": Button1}


@_rendered_statement
def test_render_field_with_constraint(session: Session, package: Package):
    Field1 = Field.input(
        "Field1", int, constraint=constraint(min_value=1.0, max_value=10.0, step_value=2.0)
    )
    return {"Field1": Field1}


@_rendered_statement
def test_render_variable(session: Session, package: Package):
    Variable1 = Block.new(BlockType.VARIABLE, "Variable1", value_type=to_type(int), value=5)
    return {"Variable1": Variable1}


@_rendered_statement
def test_render_variable_with_file(session: Session, package: Package):
    myfile_txt = File(
        type=FileType.TEXT,
        kind=FileKind.DRIVE,
        name="myfile.txt",
        mime_type="text/plain",
        size=1024,
    )
    Variable1 = Block.new(
        BlockType.VARIABLE, "Variable1", value_type=to_type(File), value=myfile_txt
    )
    return {"myfile_txt": myfile_txt, "Variable1": Variable1}


@_rendered_statement
def test_render_create_action(session: Session, package: Package):
    Action1 = Action.new(ActionType.CREATE, "Action1", node_partial=Block.partial(BlockType.PAGE))
    return {"Action1": Action1}


@_rendered_statement
def test_render_flow_simple(session: Session, package: Package):
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Action.new(ActionType.START, "Start")
    Complete = Action.new(ActionType.COMPLETE, "Complete")
    Flow1.actions.extend(Start, Complete)
    Forward1 = Start.connect(PipeType.FORWARD, Complete, "Forward1")
    return {"Flow1": Flow1, "Start": Start, "Complete": Complete, "Forward1": Forward1}


@_rendered_statement
def test_render_flow_computed_value(session: Session, package: Package):
    Flow1 = Block.new(
        BlockType.FLOW,
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
    Forward1 = Start.connect(PipeType.FORWARD, Complete, "Forward1")
    return {"Flow1": Flow1, "Start": Start, "Complete": Complete, "Forward1": Forward1}


#
# Other
#


def test_render_page(session: Session, package: Package):
    Page = package.blocks.create(name="Page", type=BlockType.PAGE)
    Text1 = Page.blocks.append(Block.new(BlockType.TEXT, "Text1", text=md("Hello, world!")))
    rendered_page = render(Page, options=RenderOptions(scope=Page, as_page=True))
    assert Text1.name in rendered_page


def test_render_simple_choice_option_ref(session: Session, package: Package):
    """Rendered node ref in sibling scope should be simplified"""
    Page: Block = package.blocks.create(name="Page", type=BlockType.PAGE)
    Choice = Block.new(
        BlockType.CHOICE,
        "Choice",
        fields=[Field.option("Option1"), Field.option("Option2"), Field.option("Option3")],
    )
    Page = Block.new(BlockType.PAGE, "Page")
    Page.blocks.extend(Choice)
    rendered_option = render_expression(
        Choice.fields.Option2, options=RenderOptions(scope=Page), as_ref=True
    )
    assert rendered_option == "Choice.fields.Option2"

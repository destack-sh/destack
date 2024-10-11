import functools
import inspect
import textwrap
from typing import Any, Callable, Mapping, cast
from uuid import UUID

from hypothesis import assume, given

from bench.language import md
from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType, ReferenceKind, StructType
from bench.language.field import Field, to_type
from bench.language.file import File, FileKind, FileType
from bench.language.flow import Pipe, PipeType, Step, StepType
from bench.language.node import BuiltinObject, Node, NodeReference
from bench.language.render import Renderer, RenderOptions, render, render_expr
from bench.language.session import Session
from bench.language.validation import constraint
from bench.language.view import View, ViewType
from bench.runtime.compiler import BUILTIN_GLOBALS
from bench.runtime.core import STATIC_CODE_GLOBALS
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS_BY_TYPE, BUILTIN_OBJECTS_OF_EVERY_TYPE


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_render_builtin_object_expr(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    assume(obj.metatype != StructType.TEXT)  # :CrummyMarkdown
    assume(obj.metatype != StructType.TYPE_CONSTRAINT)  # coerced to TypeConstraintIn (incomparable)

    renderer = Renderer(RenderOptions(scope=shared_package))

    # impute real nodes for required node references (since they're needed for rendering)
    node_references = {}
    for o in obj._walk_struct():
        for prop in o.__node_reference_properties__.values():
            if prop.reference_kind != ReferenceKind.NODE_REGULAR:
                continue
            wired_prop = prop.reference_wired_ptr
            assert wired_prop is not None, f"no wired prop for {prop!r}"
            if not wired_prop.is_required or wired_prop.is_list or not wired_prop.reference_nodes:
                # ignore any generated references (they're not real)
                if wired_prop.is_list:
                    setattr(o, prop.name, [])
                else:
                    setattr(o, prop.name, None)
                continue
            reference_node = BUILTIN_OBJECTS_BY_TYPE[wired_prop.reference_nodes[0]]
            assert isinstance(reference_node, Node), f"expected Node, got {reference_node!r}"
            reference_alias = renderer.aliasing.add(reference_node)
            node_references[reference_alias] = reference_node
            setattr(o, prop.name, reference_node)

    # render
    rendered = renderer.render_builtin_object_expr(obj)
    glbls = {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS, **node_references}
    ret = eval(rendered, glbls)
    assert cast(BuiltinObject, ret).equals(obj)


#
# Roundtrip render statements
#


def _render_as_stmt(func: Callable[[Any, Any], Mapping[str, BuiltinObject]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    @functools.wraps(func)
    def _inner(shared_session: Session, shared_package: Package):
        # (line length 96 because it's 100 - 4 for the method indent here)
        render_options = RenderOptions(scope=shared_package, format=True, format_line_length=96)
        original_defns = func(shared_session, shared_package)

        # render
        rendered = render(*original_defns.values(), options=render_options)

        # should match source (minus last line)
        source = inspect.getsource(func)
        source = "\n".join(source.splitlines()[2:-1])  # remove return
        source = textwrap.dedent(source).strip()
        assert rendered == source

        # eval as statement
        glbls = {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS}
        glbls_tmp = {**glbls}
        exec(rendered, glbls_tmp)
        rendered_defns = {
            name: obj
            for name, obj in glbls_tmp.items()
            if name not in glbls and isinstance(obj, BuiltinObject)
        }

        # check that all definitions are equal
        identity_map: dict[UUID, NodeReference] = {}
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            if isinstance(original_obj, Node):
                assert isinstance(rendered_obj, Node)
                identity_map[original_obj.ck] = rendered_obj.to_plain_ref()
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            assert cast(BuiltinObject, rendered_obj).equals(original_obj, identity_map=identity_map)

        # render again from evaluated
        rendered_again = render(*rendered_defns.values(), options=render_options)
        assert rendered == rendered_again

    return _inner


@_render_as_stmt
def test_render_bad_names(shared_session: Session, shared_package: Package):
    _F_1 = Field.variable("-F_1", str)
    Block_with_Spa_se = Block.new(BlockType.CLASS, "Block with Spa'se")
    return {"_F_1": _F_1, "Block_with_Spa_se": Block_with_Spa_se}


@_render_as_stmt
def test_render_choice_block(shared_session: Session, shared_package: Package):
    ShapeType = Block.new(
        BlockType.CHOICE,
        "ShapeType",
        text=md("All sorts of **shapes**!"),
        fields=[Field.option("Circle"), Field.option("Square"), Field.option("Triangle")],
    )
    return {"ShapeType": ShapeType}


@_render_as_stmt
def test_render_class_block(shared_session: Session, shared_package: Package):
    ShapeType = Block.new(
        BlockType.CHOICE,
        "ShapeType",
        fields=[Field.option("Circle"), Field.option("Square"), Field.option("Triangle")],
    )
    Shape = Block.new(
        BlockType.CLASS,
        "Shape",
        fields=[Field.member("kind", ShapeType), Field.member("is_cool", bool)],
    )
    return {"ShapeType": ShapeType, "Shape": Shape}


@_render_as_stmt
def test_render_variable_block(shared_session: Session, shared_package: Package):
    Variable1 = Block.new(BlockType.VALUE, "Variable1", value_type=to_type(int), value=1)
    return {"Variable1": Variable1}


@_render_as_stmt
def test_render_view_block(shared_session: Session, shared_package: Package):
    View_1 = Block.new(BlockType.VIEW, "View 1")
    Logs1 = View.new(ViewType.LOG, "Logs1", node=View_1)
    View_1.views.append(Logs1)
    return {"View_1": View_1, "Logs1": Logs1}


@_render_as_stmt
def test_render_field_with_constraint(shared_session: Session, shared_package: Package):
    Field1 = Field.input(
        "Field1", int, constraint=constraint(min_value=1.0, max_value=10.0, step_value=2.0)
    )
    return {"Field1": Field1}


@_render_as_stmt
def test_render_variable(shared_session: Session, shared_package: Package):
    Variable1 = Block.new(BlockType.VALUE, "Variable1", value_type=to_type(int), value=5)
    return {"Variable1": Variable1}


@_render_as_stmt
def test_render_variable_with_file(shared_session: Session, shared_package: Package):
    File1 = File(
        kind=FileKind.DRIVE,
        title="myfile.txt",
        type=FileType.TEXT,
        mime_type="text/plain",
        size=1024,
    )
    Variable1 = Block.new(BlockType.VALUE, "Variable1", value_type=to_type(File), value=File1)
    return {"File1": File1, "Variable1": Variable1}


@_render_as_stmt
def test_render_flow(shared_session: Session, shared_package: Package):
    Flow1 = Block.new(BlockType.FLOW, "Flow1")
    Start = Step.new(StepType.START, "Start")
    Flow1.steps.append(Start)
    Complete = Step.new(StepType.COMPLETE, "Complete")
    Flow1.steps.append(Complete)
    Pipe1 = Pipe.new(PipeType.CONTROL_AND_DATA, "Pipe1", source=Start, target=Complete)
    Flow1.pipes.append(Pipe1)
    return {"Flow1": Flow1, "Start": Start, "Complete": Complete, "Pipe1": Pipe1}


#
# Other renderings
#


def test_render_page(shared_session: Session, shared_package: Package):
    Page = shared_package.blocks.create(name="Page", type=BlockType.PAGE)
    Text1 = Page.blocks.append(Block.new_text("Text1", "Hello, world!"))
    rendered_page = render(Page, options=RenderOptions(scope=Page, as_page=True))
    assert Text1.name in rendered_page


def test_render_simple_choice_option_ref(shared_session: Session, shared_package: Package):
    """Rendered node ref in sibling scope should be simplified"""
    Page = shared_package.blocks.create(name="Page", type=BlockType.PAGE)
    Choice = Block.new(
        BlockType.CHOICE,
        "Choice",
        fields=[Field.option("Option1"), Field.option("Option2"), Field.option("Option3")],
    )
    Function = Block.new_code("Function", "")
    Page.blocks.extend(Choice, Function)
    rendered_option = render_expr(
        Choice.fields.Option2, options=RenderOptions(scope=Function), as_ref=True
    )
    assert rendered_option == "Choice.Option2"

import functools
import inspect
import re
import textwrap
from typing import Any, Callable, Mapping, assert_never, cast

import pytest
from fastuuid import UUID

from bench.language import (
    Action,
    ActionType,
    Aliasing,
    BuiltinObject,
    Choice,
    Field,
    Flow,
    FlowEdgeType,
    Message,
    Node,
    NodeReference,
    Option,
    Package,
    Page,
    Property,
    Renderer,
    RenderOptions,
    Run,
    Schema,
    Session,
    code,
    format_code,
    render_expression,
    text,
    to_type,
)
from bench.runtime.code import BUILTIN_GLOBALS, STATIC_CODE_GLOBALS


def _render_test(func: Callable[[Any, Any], Mapping[str, Any]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    def _render_and_check(
        func: Callable[[Any, Any], Mapping[str, BuiltinObject | Property]],
        session: Session,
        package: Package,
    ) -> None:
        """Common logic for rendering and checking rendered code matches original."""

        def _render(defns: Mapping[str, Any]):
            # (line length 96 because it's 100 - 4 for the method indent here)
            options = RenderOptions(
                aliasing=Aliasing(session.supergraph), format=True, line_length=96
            )
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

                    if isinstance(obj, (BuiltinObject, Property)):
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
        source = re.sub(r'""".*?"""', "", source, flags=re.DOTALL)
        source = re.sub(r"'''.*?'''", "", source, flags=re.DOTALL)
        source = source.strip()
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


# NOTE :Broken: the multi-line string tests don't work well because of :BadCodeFormatting
#  (we should be using ruff to format the code but it doesn't have a Python API yet :c)


@pytest.mark.skip(reason=":BadCodeFormatting")
@_render_test
def test_render_text(session: Session, package: Package):
    """Text should be rendered inline :BadCodeFormatting."""
    Text1 = text("Hello, world!")
    Text2 = text("Hey, we can do `code` and **bold**!")
    Text3 = text("""\
This is a multi-line text.
We can also include **Markdown** inside multiline text.
""")
    Message1 = Message.new(
        text=text("""\
Yeah, this is a long answer.
                                     
# Heading 1
## Heading 2
...
""")
    )
    return {"Text1": Text1, "Text2": Text2, "Text3": Text3, "Message1": Message1}


@pytest.mark.skip(reason=":BadCodeFormatting")
@_render_test
def test_render_code(session: Session, package: Package):
    """Code should be rendered inline :BadCodeFormatting."""
    Code1 = code("print('Hello, world!')")
    Code2 = code("""\
def hello_world():
    print("Hello, world!")
""")
    return {"Code1": Code1, "Code2": Code2}


@_render_test
def test_render_property(session: Session, package: Package):
    """Property references should be rendered with `get_property`."""
    prop_1 = Node.property("id")
    prop_2 = Page.property("title")
    prop_3 = Run.property("outputs")
    return {"prop_1": prop_1, "prop_2": prop_2, "prop_3": prop_3}


@_render_test
def test_render_type_in(session: Session, package: Package):
    """Type in should be rendered as TypeIn with `to_type`."""
    type_1 = to_type(int)
    type_2 = to_type(str)
    return {"type_1": type_1, "type_2": type_2}


@_render_test
def test_render_class(session: Session, package: Package):
    """Message types should be rendered inline."""
    Choice1 = Choice.new(
        "Choice1", Option.new("Option1"), Option.new("Option2"), Option.new("Option3")
    )
    Class1 = Schema.new("Class1", Field.member("field1", Choice1))
    return {"Choice1": Choice1, "Class1": Class1}


@_render_test
def test_render_flow_simple(session: Session, package: Package):
    """Flows should create Links with `connect`."""
    Flow1 = Flow.new("Flow1")
    Action1 = Action.new(ActionType.START, "Action1")
    Action2 = Action.new(ActionType.END, "Action2")
    Flow1.add_children(Action1, Action2)
    Transition1 = Action1.connect(FlowEdgeType.REQUIRE, Action2, "Transition1")
    return {"Flow1": Flow1, "Action1": Action1, "Action2": Action2, "Transition1": Transition1}


def test_render_simple_choice_option_ref(session: Session, package: Package):
    """Rendered node ref in sibling scope should be simplified"""
    Page1 = Page.new("Page")
    package.add_child(Page1)
    Choice1 = Choice.new(
        "Choice1",
        Option.new("Option1"),
        Option.new("Option2"),
        Option.new("Option3"),
    )
    Page1.add_child(Choice1)
    rendered_option = render_expression(
        Choice1.child(Option, "Option2"),
        options=RenderOptions(aliasing=Aliasing(session.supergraph)),
        as_ref=True,
    )
    assert rendered_option == "Choice1.options.Option2"

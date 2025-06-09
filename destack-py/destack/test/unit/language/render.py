import functools
import inspect
import re
import textwrap
from collections.abc import Mapping
from itertools import chain
from typing import Any, Callable, assert_never

import pytest
from fastuuid import UUID

from destack.language import (
    Aliasing,
    BuiltinObjectBase,
    Message,
    Node,
    Package,
    Page,
    Property,
    Renderer,
    RenderOptions,
    Run,
    Session,
    text,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from destack.utils.code import format_code


def _render_test(func: Callable[[Any, Any], Mapping[str, Any]]):
    """Decorator to check that the function body is exactly equivalent to its (re)rendered form."""

    def _render_and_check(
        func: Callable[[Any, Any], Mapping[str, BuiltinObjectBase | Property]],
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

                    if isinstance(obj, (BuiltinObjectBase, Property)):
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
        glbls = {
            cls.__name__: cls
            for cls in chain(
                NODE_CLASS_BY_TYPE.values(),
                STRUCT_CLASS_BY_TYPE.values(),
                ENUM_CLASS_BY_TYPE.values(),
            )
        }
        glbls_tmp = {**glbls}
        exec(rendered, glbls_tmp)
        rendered_defns: dict[str, Any] = {
            name: obj
            for name, obj in glbls_tmp.items()
            if name not in glbls and name not in ("__builtins__", "__doc__", "__file__", "__name__")
        }

        # check that all definitions are equal
        identity_map: dict[UUID, UUID] = {}
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            if isinstance(original_obj, Node):
                assert isinstance(rendered_obj, Node)
                identity_map[original_obj.id] = rendered_obj.id
        for name, original_obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            if isinstance(original_obj, Property):
                assert (
                    original_obj.component == rendered_obj.component
                    and original_obj.id == rendered_obj.id
                )
            elif isinstance(original_obj, BuiltinObjectBase):
                assert original_obj.equals(rendered_obj, _identity_map=identity_map)
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
    Message1 = Message(
        text=text("""\
Yeah, this is a long answer.
                                     
# Heading 1
## Heading 2
...
""")
    )
    return {"Text1": Text1, "Text2": Text2, "Text3": Text3, "Message1": Message1}


@_render_test
def test_render_property(session: Session, package: Package):
    """Property references should be rendered with `get_property`."""
    prop_1 = Node.property("id")
    prop_2 = Page.property("title")
    prop_3 = Run.property("outputs")
    return {"prop_1": prop_1, "prop_2": prop_2, "prop_3": prop_3}

import functools
import inspect
import textwrap
from typing import Any, Callable, Mapping, cast

from hypothesis import given

from bench.language import md
from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.node import BuiltinObject
from bench.language.render import RenderOptions, render
from bench.language.session import Session
from bench.runtime.compiler import BUILTIN_GLOBALS
from bench.runtime.core import STATIC_CODE_GLOBALS
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_render_builtin_object_expr(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    rendered = render(obj, options=RenderOptions(scope=shared_package))
    glbls = {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS}
    ret = eval(rendered, glbls)
    assert cast(BuiltinObject, ret)._equals_content(obj)


def _render_as_stmt(func: Callable[[Any, Any], Mapping[str, BuiltinObject]]):
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
        for name, obj in original_defns.items():
            rendered_obj = rendered_defns[name]
            assert cast(BuiltinObject, rendered_obj)._equals_content(obj)

        # render again from evaluated
        rendered_again = render(*rendered_defns.values(), options=render_options)
        assert rendered == rendered_again

    return _inner


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


# NOTE :Test: test many more renderings

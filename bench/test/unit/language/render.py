from typing import cast

from hypothesis import given

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
def test_render_builtin_object(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    rendered = render(obj, options=RenderOptions(scope=shared_package))
    glbls = {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS}
    ret = eval(rendered, glbls)
    assert cast(BuiltinObject, ret)._equals_content(obj)


def test_render_choice_block(shared_session: Session, shared_package: Package):
    Shape = Block.new(
        BlockType.CHOICE,
        "Shape",
        fields=(Field.option("Circle"), Field.option("Square"), Field.option("Triangle")),
    )
    rendered = render(Shape, options=RenderOptions(scope=shared_package))
    print(rendered)

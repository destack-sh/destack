from typing import cast

from hypothesis import given

from bench.language import Node
from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType, NodeType, StructType
from bench.language.field import Field
from bench.language.node import BuiltinObject
from bench.language.render import render
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
    rendered = render(obj)
    exec(rendered, {**STATIC_CODE_GLOBALS, **BUILTIN_GLOBALS})
    assert cast(BuiltinObject, ret)._equals_content(obj)


def test_render_nested(session: Session, package: Package):
    # choice block
    Choice1 = Block(parent=package, type=BlockType.CHOICE, name="Choice1")
    Choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )

    # inner class
    ClassInner = Block(
        parent=package,
        type=BlockType.CLASS,
        name="ClassInner",
        fields=(Field.member("Field1", Choice1), Field.member("Field2", NodeType.BLOCK)),
    )

    # outer class
    ClassOuter = Block(
        parent=package,
        type=BlockType.CLASS,
        name="ClassOuter",
        fields=(
            Field.member("Field1", Choice1),
            Field.member("Field2", bool),
            Field.member("Field3", StructType.TEXT),
            Field.member("Field4", ClassInner),
        ),
    )

    # render
    rendered = render(Choice1, ClassInner, ClassOuter)
    glbls = {**STATIC_CODE_GLOBALS}
    exec(rendered, glbls)
    for key, value in (
        ("Choice1", Choice1),
        ("ClassInner", ClassInner),
        ("ClassOuter", ClassOuter),
    ):
        assert key in glbls
        assert cast(Node, glbls[key])._equals_content(value)

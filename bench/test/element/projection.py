from typing import cast

from hypothesis import given

from bench.language import Node
from bench.language.bench import Package
from bench.language.block import Block
from bench.language.code import format_code, run_code_eval, run_code_script
from bench.language.const import BlockType, NodeType, StructType
from bench.language.field import Field
from bench.language.node import BuiltinObject
from bench.language.projection import render_builtin_object, render_node
from bench.language.session import Session
from bench.test.element.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE
from bench.test.strategies import builtin_objects, examples


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_render_struct(obj: BuiltinObject, shared_session: Session, shared_package: Package):
    rendered = render_builtin_object(obj)
    rendered = format_code(rendered)
    run_code_eval(rendered)
    # assert cast(Struct, ret)._equals_content(obj) # TODO :Robustness :Test: assert


def test_render_nested(session: Session, package: Package):
    # choice block
    Choice1 = Block(parent=package, type=BlockType.CHOICE, name="Choice1")
    Choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )

    # inner class
    ClassInner = Block(parent=package, type=BlockType.CLASS, name="ClassInner")
    ClassInner.fields.extend(
        Field.member("Field1", Choice1), Field.member("Field2", NodeType.BLOCK)
    )

    # outer class
    ClassOuter = Block(parent=package, type=BlockType.CLASS, name="ClassOuter")
    ClassOuter.fields.extend(
        Field.member("Field1", Choice1),
        Field.member("Field2", bool),
        Field.member("Field3", StructType.TEXT),
        Field.member("Field4", ClassInner),
    )

    # render
    rendered = render_node([Choice1, ClassInner, ClassOuter])
    rendered = format_code(rendered)
    ret = run_code_script(rendered)
    for key, value in (
        ("Choice1", Choice1),
        ("ClassInner", ClassInner),
        ("ClassOuter", ClassOuter),
    ):
        assert key in ret
        assert cast(Node, ret[key])._equals_content(value)

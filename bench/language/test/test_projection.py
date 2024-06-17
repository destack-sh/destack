from typing import cast

import pytest

from bench.language import Node, Struct
from bench.language.block import Block
from bench.language.code import format_code, run_code_eval, run_code_script
from bench.language.const import BlockType, NodeType, StructType
from bench.language.field import Field
from bench.language.projection import render_node, render_struct

BUILTIN_OBJECTS = ()  # nocheckin


@pytest.mark.parametrize("obj", BUILTIN_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_render_struct(obj: Node | Struct):
    rendered = render_struct(obj)
    rendered = format_code(rendered)
    run_code_eval(rendered)
    # assert cast(Struct, ret)._equals_content(obj) # TODO :Robustness :Incomplete: assert


def test_render_nested():
    # choice block
    Choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    Choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )

    # inner class
    ClassInner = Block(type=BlockType.CLASS, name="ClassInner")
    ClassInner.fields.extend(
        Field.member("Field1", Choice1), Field.member("Field2", NodeType.BLOCK)
    )

    # outer class
    ClassOuter = Block(type=BlockType.CLASS, name="ClassOuter")
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

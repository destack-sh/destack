import pytest

from bench.language import Node, Struct
from bench.language.block import Block
from bench.language.code_ import format_code
from bench.language.const import (
    OBJECT_TYPES,
    BlockType,
    NodeType,
    PrimitiveType,
    StructType,
    TypeKind,
)
from bench.language.field import Field
from bench.language.projection import render_struct
from bench.language.setup import BENCH_CLASS_BY_TYPE
from bench.language.test.fabricator import Fabricator

fabricator = Fabricator(42)
BENCH_OBJECTS = tuple(fabricator.fabricate(BENCH_CLASS_BY_TYPE[t], ()) for t in OBJECT_TYPES)


@pytest.mark.parametrize("bench_obj", BENCH_OBJECTS, ids=lambda o: o.__class__.__name__)
def test_render_single(bench_obj: Node | Struct):
    rendered = render_struct(bench_obj)
    rendered = format_code(rendered)
    print(rendered)
    # nocheckin: assert


def test_render_nested():
    # choice block
    choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )

    # inner class
    class_inner = Block(type=BlockType.CLASS, name="ClassInner")
    class_inner.fields.extend(
        Field.member(
            name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
        ),
        Field.member(name="Field2", bench_type=NodeType.BLOCK, kind=TypeKind.NODE),
    )

    # outer class
    class_outer = Block(type=BlockType.CLASS, name="ClassOuter")
    class_outer.fields.extend(
        Field.member(
            name="Field1", kind=TypeKind.BASED_NODE, bench_type=NodeType.FIELD, base_type=choice1
        ),
        Field.member(name="Field2", kind=TypeKind.PRIMITIVE, primitive_type=PrimitiveType.BOOLEAN),
        Field.member(name="Field3", kind=TypeKind.STRUCT, bench_type=StructType.TEXT, is_list=True),
        Field.member(name="Field4", kind=TypeKind.ALIAS, base_type=class_inner),
    )

    # render
    rendered = render_struct(class_outer)
    rendered = format_code(rendered)
    print(rendered)
    # nocheckin: assert

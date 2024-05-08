from typing import cast

import pytest

from bench.language.block import Block
from bench.language.const import BlockType, NodeType, PrimitiveType, StructType
from bench.language.field import Field, FieldZone, TypeKind
from bench.language.notice import on_notice_ignore
from bench.language.text import Text
from bench.language.value import Object, pack_value, unpack_value


def test_coerce_nested_value() -> None:
    # choice block
    Choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    Choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )
    Choice1._interp_rec(None, on_notice_ignore)
    Option1 = Choice1("Option1")
    assert Option1 == Choice1.fields.Option1
    with pytest.raises(ValueError):
        Choice1("Option17")

    # inner class
    ClassInner = Block(type=BlockType.CLASS, name="ClassInner")
    ClassInner.fields.extend(
        Field.member("Field1", Choice1), Field.member("Field2", NodeType.BLOCK)
    )
    ClassInner._interp_rec(None, on_notice_ignore)
    object_inner = ClassInner(field1=Option1)
    assert object_inner.field1 == Option1

    # outer class
    ClassOuter = Block(type=BlockType.CLASS, name="ClassOuter")
    ClassOuter.fields.extend(
        Field.member("Field1", Choice1),
        Field.member("Field2", bool),
        Field.member("Field3", StructType.TEXT),
        Field.member("Field4", ClassInner),
    )
    ClassOuter._interp_rec(None, on_notice_ignore)
    object_outer = ClassOuter(
        field1=Option1, field2=False, field3=[Text.plain("hello bench!")], field4=object_inner
    )
    assert object_outer.field1 == Option1
    assert object_outer.field2 is False
    assert object_outer.field3 == [Text.plain("hello bench!")]
    assert object_outer.field4 == object_inner


def test_roundtrip_nested_value():
    # choice block
    choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    choice1.fields.create(name="Option1", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option2", zone=FieldZone.OPTION)
    choice1.fields.create(name="Option3", zone=FieldZone.OPTION)

    # inner class
    class2 = Block(type=BlockType.CLASS, name="Class2")
    class2.fields.create(
        name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
    )
    class2.fields.create(name="Field2", bench_type=NodeType.BLOCK, kind=TypeKind.NODE)

    # outer class
    class1 = Block(type=BlockType.CLASS, name="Class1")
    class1.fields.create(
        name="Field1", bench_type=NodeType.FIELD, base_type=choice1, kind=TypeKind.BASED_NODE
    )
    class1.fields.create(
        name="Field2", primitive_type=PrimitiveType.BOOLEAN, kind=TypeKind.PRIMITIVE
    )
    class1.fields.create(
        name="Field3", bench_type=StructType.TEXT, kind=TypeKind.STRUCT, is_list=True
    )
    class1.fields.create(name="Field4", base_type=class2, kind=TypeKind.ALIAS)

    # TODO :Cleanup :Test: interp/to_resolved shit should not be necessary
    #  (run this test in session? or somehow in 'tracked' mode)
    choice1._interp_rec(None, on_notice_ignore)
    class2._interp_rec(None, on_notice_ignore)
    class1._interp_rec(None, on_notice_ignore)

    # outer value
    value = cast(Object, class1())
    # TODO :Broken: pack/unpack_value does not turn node refs back into nodes
    #  (so the assertion below would fail if the next line is uncommented)
    # value.field1 = choice1.fields.Option1
    value.field2 = False
    value.field3 = [Text.plain("hello bench!")]
    value.field4 = cast(Object, class2())

    value_packed, secret_value_packed = pack_value(value, class1.as_type)
    unpacked_value = unpack_value(value_packed, secret_value_packed, class1.as_type)
    assert unpacked_value == value


# TODO :Test: auto generate :Test types & values

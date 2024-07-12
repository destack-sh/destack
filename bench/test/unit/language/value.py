from typing import cast

import pytest
from hypothesis import given

from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType, NodeType, PrimitiveType, StructType
from bench.language.field import Field, TypeKind, to_type
from bench.language.node import BuiltinObject
from bench.language.session import Session
from bench.language.text import Text
from bench.language.value import (
    ValueObject,
    pack_builtin_object_data,
    pack_value,
    unpack_builtin_object_data,
    unpack_value,
)
from bench.proto import wiring
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE


def test_coerce_nested_value(session: Session, package: Package) -> None:
    """Coerce a nested Object value."""

    # choice block
    Choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    Choice1.fields.extend(
        Field.option(name="Option1"), Field.option(name="Option2"), Field.option(name="Option3")
    )
    Option1 = Choice1("Option1")
    assert Option1 == Choice1.fields.Option1
    with pytest.raises(ValueError):
        Choice1("Option17")

    # inner class
    ClassInner = Block(type=BlockType.CLASS, name="ClassInner")
    ClassInner.fields.extend(
        Field.member("Field1", Choice1), Field.member("Field2", NodeType.BLOCK)
    )
    object_inner = ClassInner(field1=Option1)
    assert object_inner.field1 == Option1

    # outer class
    ClassOuter = Block(type=BlockType.CLASS, name="ClassOuter")
    ClassOuter.fields.extend(
        Field.member("Field1", Choice1),
        Field.member("Field2", bool),
        Field.member("Field3", Text),
        Field.member("Field4", ClassInner),
    )
    object_outer = ClassOuter(
        field1=Option1, field2=False, field3=[Text.plain("hello bench!")], field4=object_inner
    )
    assert object_outer.field1 == Option1
    assert object_outer.field2 is False
    assert object_outer.field3 == [Text.plain("hello bench!")]
    assert object_outer.field4 == object_inner


def test_roundtrip_scalar_value(session: Session, package: Package) -> None:
    """Pack/unpack a scalar value inside a (Variable) Block (which HasValues)."""

    # first set in constructor
    type_info = to_type(PrimitiveType.INT32)
    block = Block(type=BlockType.VARIABLE, name="Variable1", value_type=type_info, value=7)
    assert block.value == 7
    assert unpack_value(block.value_packed, type_info) == 7

    # set at runtime
    block.value = 42
    assert block.value == 42
    assert unpack_value(block.value_packed, type_info) == 42


def test_roundtrip_nested_value(session: Session, package: Package):
    """Pack/unpack a nested Object value."""

    # choice block
    choice1 = Block(type=BlockType.CHOICE, name="Choice1")
    choice1.fields.append(Field.option("Option1"))
    choice1.fields.append(Field.option("Option2"))
    choice1.fields.append(Field.option("Option3"))

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

    # outer value
    value = cast(ValueObject, class1())
    # TODO :Broken: value pack/unpack does not yet turn node refs back into nodes
    #  (so the assertion below would fail if the next line is uncommented)
    value.field1 = choice1.fields.Option1
    assert value.field1 is choice1.fields.Option1
    value.field2 = False
    value.field3 = [Text.plain("hello bench!")]
    value.field4 = cast(ValueObject, class2())

    class1_type = class1.to_type(as_object=True)
    value_packed = pack_value(value, class1_type)
    unpacked_value = unpack_value(value_packed, class1_type)
    assert unpacked_value == value


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_builtin_object_value(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    packed_wire_obj = wiring.pack_object(obj)
    packed_json = pack_builtin_object_data(packed_wire_obj)
    unpacked_wire_obj = unpack_builtin_object_data(packed_json)
    unpacked_obj = wiring.unpack_object(
        unpacked_wire_obj,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


# TODO :Test: auto generate :Test types & values

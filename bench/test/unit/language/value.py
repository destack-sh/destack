from typing import cast

import pytest
from hypothesis import given

from bench.language.action import ActionMode, Call, Continue
from bench.language.bench import Package
from bench.language.block import Block, ValueBlock
from bench.language.const import (
    STRUCT_TYPES,
    BlockType,
    FieldType,
    NodeType,
    ObjectKind,
    PrimitiveType,
    StructType,
)
from bench.language.field import Field, TypeInfo, TypeKind, to_type_scalar
from bench.language.node import BuiltinObject
from bench.language.session import Session
from bench.language.text import Text
from bench.language.validation import constraint, on_invalid_raise
from bench.language.value import (
    CustomObject,
    check_value,
    coerce_custom_object,
    pack_builtin_object_data,
    pack_custom_object,
    pack_value,
    sample_value,
    unpack_builtin_object_data,
    unpack_custom_object,
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
    object_inner = ClassInner(Field1=Option1)
    assert object_inner.Field1 is Option1

    # outer class
    ClassOuter = Block(type=BlockType.CLASS, name="ClassOuter")
    ClassOuter.fields.extend(
        Field.member("Field1", Choice1),
        Field.member("Field2", bool),
        Field.member("Field3", Text),
        Field.member("Field4", ClassInner),
    )
    object_outer = ClassOuter(
        Field1=Option1, Field2=False, Field3=[Text.plain("hello bench!")], Field4=object_inner
    )
    assert object_outer.Field1 is Option1
    assert object_outer.Field2 is False
    assert object_outer.Field3 == [Text.plain("hello bench!")]
    assert object_outer.Field4 == object_inner


def test_custom_object_with_builtin_properties(session: Session, package: Package) -> None:
    """Coerce, pack & unpack custom object with builtin properties."""
    Action1 = Block.new(
        BlockType.ACTION,
        "Action1",
        mode=ActionMode.CODE,
        fields=[
            Field.output("Output1", PrimitiveType.INT32),
            Field.output("Output 2 with a Space", Text),
        ],
    )

    # coerce
    Action1Output = Action1.to_type(as_object=True, field_type=FieldType.OUTPUT)
    assert Action1Output is not None
    obj = coerce_custom_object(
        ObjectKind.OUTPUT,
        Action1Output,
        {
            "Output1": 42,
            "Output 2 with a Space": Text.plain("hello bench!"),
            "call": Call(node=Action1),
        },
    )
    assert obj.call == Call(node=Action1)

    # get/set
    obj.call = None
    assert obj.call is None
    obj.continuations = [Continue(node=Action1)]
    assert obj.continuations[0].node is Action1

    # pack/unpack
    obj_packed = pack_custom_object(obj, Action1Output)
    obj_unpacked = unpack_custom_object(
        ObjectKind.OUTPUT, obj_packed, Action1Output, supergraph=session._supergraph
    )
    assert obj_unpacked.equals(obj)


def test_roundtrip_scalar_value(session: Session, package: Package) -> None:
    """Pack/unpack a scalar value inside a (Variable) Block (which HasValues)."""

    # first set in constructor
    type_info = to_type_scalar(PrimitiveType.INT32)
    block = Block.new(ValueBlock, name="Variable1", value_type=type_info, value=7)
    assert block.value == 7
    assert unpack_value(block.value_packed, type_info, wrap_scalar=True) == 7

    # set at runtime
    block.value = 42
    assert block.value == 42
    assert unpack_value(block.value_packed, type_info, wrap_scalar=True) == 42


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
        type=FieldType.MEMBER,
        name="Field1",
        bench_type=NodeType.FIELD,
        base_type=choice1,
        kind=TypeKind.BASED_NODE,
    )
    class2.fields.create(name="Field2", bench_type=NodeType.BLOCK, kind=TypeKind.NODE)

    # outer class
    class1 = Block(type=BlockType.CLASS, name="Class1")
    class1.fields.create(
        type=FieldType.MEMBER,
        name="Field1",
        bench_type=NodeType.FIELD,
        base_type=choice1,
        kind=TypeKind.BASED_NODE,
    )
    class1.fields.create(
        type=FieldType.MEMBER,
        name="Field2",
        primitive_type=PrimitiveType.INT32,
        kind=TypeKind.PRIMITIVE,
        is_list=True,
    )
    class1.fields.create(
        type=FieldType.MEMBER, name="Field3", bench_type=StructType.TEXT, kind=TypeKind.STRUCT
    )
    class1.fields.create(
        type=FieldType.MEMBER,
        name="Field4",
        base_type=class2,
        base_field_type=FieldType.MEMBER,
        kind=TypeKind.OBJECT,
    )

    # outer value
    value = cast(CustomObject, class1())
    value.Field1 = choice1.fields.Option1
    assert value.Field1 is choice1.fields.Option1
    value.Field2 = [24]
    value.Field3 = Text.plain("hello bench!")
    value.Field4 = cast(CustomObject, class2())

    class1_type = class1.to_type(as_object=True)
    assert class1_type is not None, f"{class1!r} has no type"
    value_packed = pack_value(value, class1_type, wrap_scalar=True)
    unpacked_value = unpack_value(value_packed, class1_type, wrap_scalar=True)
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
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"


# TODO :Test: auto generate :Test types & values


def test_sample_value_scalar(session: Session, package: Package):
    typ = TypeInfo.from_type(bool)
    val = sample_value(typ)
    check_value(val, typ, on_invalid_raise)


def test_sample_value_scalar_constrained(session: Session, package: Package):
    typ = TypeInfo.from_type(int, constraint=constraint(min_value=10.0, max_value=20.0))
    val = sample_value(typ)
    check_value(val, typ, on_invalid_raise)


UNGENERATABLE_STRUCT_TYPES = [
    # Edit/Change have old/new_node_packed data
    StructType.EDIT,
    StructType.EDIT_INFO,
    StructType.EDIT_OPERATION,
    StructType.CHANGE,
    # custom objects are never instantiated
    StructType.VARIABLE_OBJECT,
    StructType.MEMBER_OBJECT,
    StructType.INPUT_OBJECT,
    StructType.OUTPUT_OBJECT,
    # references have special handling
    StructType.NODE_REFERENCE,
    StructType.PROPERTY_REFERENCE,
    # contain required references
    StructType.OBJECT_MAPPING,
    StructType.FIELD_MAPPING,
    StructType.CONTINUE,
    StructType.CALL,
]


@pytest.mark.parametrize(
    "struct_type",
    [st for st in STRUCT_TYPES if st not in UNGENERATABLE_STRUCT_TYPES],
    ids=lambda t: t.bench_name,
)
def test_sample_value_struct(struct_type: StructType, session: Session, package: Package):
    typ = TypeInfo(kind=TypeKind.STRUCT, bench_type=struct_type)
    val = sample_value(typ)
    check_value(val, typ, on_invalid_raise)


def test_sample_choice_block(session: Session, package: Package):
    page = package.blocks.append(Block.new(BlockType.PAGE, "Page1"))
    choice = Block.new(
        BlockType.CHOICE,
        "Choice1",
        fields=[Field.option("Option1"), Field.option("Option2"), Field.option("Option3")],
    )
    page.blocks.append(choice)
    sampled_field = sample_value(choice.as_type)
    assert sampled_field in choice.fields

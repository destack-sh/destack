from typing import cast

import pytest
from hypothesis import given

from bench.language.action import Agency, Call, Continue
from bench.language.bench import Package
from bench.language.block import ActionBlock, Block, ValueBlock
from bench.language.code import Code
from bench.language.const import (
    STRUCT_TYPES,
    BlockType,
    FieldType,
    NodeType,
    ObjectKind,
    PartialNodeScope,
    PrimitiveType,
    StructType,
)
from bench.language.field import Field, TypeInfo, TypeKind, to_type_scalar
from bench.language.message import Message, MessageType
from bench.language.node import BuiltinObject, Node
from bench.language.session import Session
from bench.language.text import Text
from bench.language.validation import constraint, on_invalid_raise
from bench.language.value import (
    CustomObject,
    check_value,
    coerce_custom_object,
    pack_builtin_object,
    pack_builtin_object_data,
    pack_custom_object,
    pack_value,
    sample_value,
    unpack_builtin_object,
    unpack_builtin_object_data,
    unpack_custom_object,
    unpack_value,
)
from bench.proto import wiring
from bench.test.strategies import builtin_objects, examples, structs
from bench.test.unit.conftest import BUILTIN_OBJECTS, STRUCTS


def test_custom_object_with_builtin_properties(session: Session, package: Package) -> None:
    """Coerce, pack & unpack custom object with builtin properties."""
    Action1 = Block.new(
        BlockType.ACTION,
        "Action1",
        agency=Agency.CODE,
        fields=[
            Field.output("Output1", PrimitiveType.INT32),
            Field.output("Output 2 with a Space", Text),
        ],
    )

    # coerce
    Action1Output = Action1.to_type_maybe(of="value", field_type=FieldType.OUTPUT)
    assert Action1Output is not None
    obj = coerce_custom_object(
        ObjectKind.OUTPUT,
        {
            "Output1": 42,
            "Output 2 with a Space": Text.plain("hello bench!"),
            "call": Call(node=Action1),
        },
        Action1Output,
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


def test_partial_node_message(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Message node."""
    message_type = Block.new(
        BlockType.MESSAGE,
        "MyMessage",
        fields=(
            Field.member("Field1", int),
            Field.member("Field2", Block),
            Field.member("Field3", bool),
            Field.member("title", Text),
        ),
    )
    typ = TypeInfo(
        kind=TypeKind.PARTIAL_NODE,
        bench_type=NodeType.MESSAGE,
        base_type=message_type,
        partial_scope=PartialNodeScope.FULL,
    )
    obj = CustomObject.new(ObjectKind.BUILTIN, {}, typ)

    # should be init to empty/default values for Message
    assert obj.id is None
    assert obj.type is Message.get_property("type").default
    assert obj.block is None
    assert obj.Field1 is None

    # set/get values on value and properties
    obj.Field1 = 42
    obj.type = MessageType.NATIVE
    obj.title = "My New Message"
    obj.block = message_type
    assert obj.Field1 == 42
    assert obj.type == MessageType.NATIVE
    assert obj.block == message_type
    assert obj.title == "My New Message"

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(
        ObjectKind.BUILTIN, obj_packed, typ, supergraph=session._supergraph
    )
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = Message.from_partial(obj)
    assert full_obj.id is not None
    assert full_obj.type == MessageType.NATIVE
    assert full_obj.title == "My New Message"
    assert full_obj.value
    assert full_obj.value.Field1 == 42


def test_partial_node_block(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Block node with subtypes."""
    typ = TypeInfo(kind=TypeKind.PARTIAL_NODE, bench_type=NodeType.BLOCK)
    obj = Block.partial(type=BlockType.ACTION, name="Action1")

    # should be init to set/empty/default values for ActionBlock
    assert obj.type == BlockType.ACTION
    assert obj.name == "Action1"
    assert obj.agency == ActionBlock.get_property("agency").default
    assert obj.text is None

    # set/get values on properties and subnode properties
    obj.agency = Agency.CODE
    obj.text = Text.plain("hello bench!")
    obj.code = Code.from_string("print('hello bench!')")
    assert obj.agency == Agency.CODE
    assert obj.text == Text.plain("hello bench!")
    assert obj.code == Code.from_string("print('hello bench!')")

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(
        ObjectKind.BUILTIN, obj_packed, typ, supergraph=session._supergraph
    )
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = cast(ActionBlock, Block.from_partial(obj))
    assert full_obj.id is not None
    assert full_obj.agency == Agency.CODE
    assert full_obj.text == Text.plain("hello bench!")
    assert full_obj.code == Code.from_string("print('hello bench!')")


def test_partial_node_generic(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial generic node."""
    typ = TypeInfo(kind=TypeKind.PARTIAL_NODE)
    obj = CustomObject.new(ObjectKind.BUILTIN, {}, typ)

    # should be init to empty/default values for Node
    assert obj.id is None
    assert obj.parent is None
    # shouldn't have any sub-properties yet
    with pytest.raises(AttributeError):
        _ = obj.text  # doesn't exist on Node

    # set metatype, then set properties for Field
    obj.metatype = NodeType.FIELD
    obj.type = FieldType.OPTION
    obj.kind = TypeKind.LITERAL
    obj.name = "Option1"
    assert obj.name == "Option1"

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(
        ObjectKind.BUILTIN, obj_packed, typ, supergraph=session._supergraph
    )
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = cast(Field, Node.from_partial(obj))
    assert full_obj.id is not None
    assert full_obj.name == "Option1"


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

    # inner message
    message2 = Block(type=BlockType.MESSAGE, name="Message2")
    message2.fields.create(
        type=FieldType.MEMBER,
        name="Field1",
        bench_type=NodeType.FIELD,
        base_type=choice1,
        kind=TypeKind.BASED_NODE,
    )
    message2.fields.create(name="Field2", bench_type=NodeType.BLOCK, kind=TypeKind.NODE)

    # outer message
    message1 = Block(type=BlockType.MESSAGE, name="Message1")
    message1.fields.create(
        type=FieldType.MEMBER,
        name="Field1",
        bench_type=NodeType.FIELD,
        base_type=choice1,
        kind=TypeKind.BASED_NODE,
    )
    message1.fields.create(
        type=FieldType.MEMBER,
        name="Field2",
        primitive_type=PrimitiveType.INT32,
        kind=TypeKind.PRIMITIVE,
        is_list=True,
    )
    message1.fields.create(
        type=FieldType.MEMBER, name="Field3", bench_type=StructType.TEXT, kind=TypeKind.STRUCT
    )
    message1.fields.create(
        type=FieldType.MEMBER,
        name="Field4",
        base_type=message2,
        base_field_type=FieldType.MEMBER,
        kind=TypeKind.CUSTOM_OBJECT,
    )

    # outer value
    value = CustomObject.new(ObjectKind.MEMBER, {}, message1.to_type(of="value"))
    value.Field1 = choice1.fields.Option1
    assert value.Field1 is choice1.fields.Option1
    value.Field2 = [24]
    value.Field3 = Text.plain("hello bench!")
    value.Field4 = CustomObject.new(ObjectKind.MEMBER, {}, message2.to_type(of="value"))

    value_packed = pack_value(value, message1.to_type(of="value"), wrap_scalar=True)
    unpacked_value = unpack_value(value_packed, message1.to_type(of="value"), wrap_scalar=True)
    assert unpacked_value == value


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
def test_roundtrip_builtin_object_value_data(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    packed_wire_obj = wiring.pack_builtin_object(obj)
    packed_json = pack_builtin_object_data(packed_wire_obj)
    unpacked_wire_obj = unpack_builtin_object_data(packed_json)
    unpacked_obj = wiring.unpack_builtin_object(
        unpacked_wire_obj,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=structs)
@examples([{"obj": obj} for obj in STRUCTS])
def test_roundtrip_builtin_object_value(
    obj: BuiltinObject, shared_session: Session, shared_package: Package
):
    packed_json = pack_builtin_object(obj)
    unpacked_obj = unpack_builtin_object(
        packed_json, session=shared_session, supergraph=shared_session._supergraph
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"


# TODO :Test: auto generate :Test types & values

#
# Sampling
#

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
    # nodes have special handling
    StructType.NODE_REFERENCE,
    StructType.PROPERTY_REFERENCE,
    # contain required references
    StructType.OBJECT_MAPPING,
    StructType.FIELD_MAPPING,
    StructType.CONTINUE,
    StructType.CALL,
]


def test_sample_value_scalar(session: Session, package: Package):
    typ = TypeInfo.from_type(bool)
    val = sample_value(typ)
    check_value(val, typ, on_invalid_raise)


def test_sample_value_scalar_constrained(session: Session, package: Package):
    typ = TypeInfo.from_type(int, constraint=constraint(min_value=10.0, max_value=20.0))
    val = sample_value(typ)
    check_value(val, typ, on_invalid_raise)


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
    typ = choice.to_type_maybe(of="instance")
    assert typ is not None, f"{choice!r} has no type"
    sampled_field = sample_value(typ)
    assert sampled_field in choice.fields

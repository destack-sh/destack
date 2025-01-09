from typing import cast

import pytest
from hypothesis import HealthCheck, given, settings

from bench.language import (
    DEFAULT_CHECK_OPTIONS,
    STRUCT_TYPES,
    Action,
    ActionType,
    Block,
    BlockType,
    BuiltinObject,
    Code,
    CreateAction,
    CustomObject,
    DuplicateAction,
    Field,
    FieldType,
    Message,
    MessageType,
    Node,
    NodeType,
    Package,
    PrimitiveType,
    Record,
    Session,
    StructType,
    Text,
    Type,
    TypeKind,
    VariableBlock,
    check_value,
    coerce_custom_object_scalar,
    constraint,
    on_invalid_raise,
    pack_builtin_object,
    pack_builtin_object_data,
    pack_custom_object,
    pack_value,
    sample_value,
    to_type_scalar,
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
    Flow1 = Block.new(
        BlockType.FLOW,
        "Flow1",
        fields=[
            Field.output("Output1", PrimitiveType.INT32),
            Field.output("Output 2 with a Space", Text),
        ],
    )

    # coerce
    Flow1Output = Flow1.to_type_maybe(of="value", field_types=[FieldType.OUTPUT])
    assert Flow1Output is not None
    obj = coerce_custom_object_scalar(
        {
            "Output1": 42,
            "Output 2 with a Space": Text.plain("hello bench!"),
        },
        Flow1Output,
    )

    # pack/unpack
    obj_packed = pack_custom_object(obj, Flow1Output)
    obj_unpacked = unpack_custom_object(obj_packed, Flow1Output, supergraph=session._supergraph)
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
    typ = Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=NodeType.MESSAGE, base_type=message_type)
    obj = CustomObject.new({}, typ)

    # should be init to empty/default values for Message
    assert obj.id is None
    assert obj.type is Message.get_property("type").default
    assert obj.block is None
    assert obj.Field1 is None

    # set/get values on value and properties
    obj.Field1 = 42
    obj.type = MessageType.INTERNAL
    obj.title = "My New Message"
    obj.block = message_type
    assert obj.Field1 == 42
    assert obj.type == MessageType.INTERNAL
    assert obj.block == message_type
    assert obj.title == "My New Message"

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = Message.from_partial(obj)
    assert full_obj.id is not None
    assert full_obj.type == MessageType.INTERNAL
    assert full_obj.title == "My New Message"
    assert full_obj.value
    assert full_obj.value.Field1 == 42


def test_partial_node_block(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Block node with subtypes."""
    typ = Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=NodeType.ACTION)
    obj = Action.partial(type=ActionType.DUPLICATE, name="Action1")

    # should be init to set/empty/default values for ActionBlock
    assert obj.type == ActionType.DUPLICATE
    assert obj.name == "Action1"
    assert obj.node is None
    assert obj.is_shallow is False

    # set/get values on properties and subnode properties
    obj.is_shallow = True
    obj.text = Text.plain("hello bench!")
    obj.code = Code.from_string("print('hello bench!')")
    assert obj.is_shallow is True
    assert obj.text == Text.plain("hello bench!")
    assert obj.code == Code.from_string("print('hello bench!')")

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = cast(DuplicateAction, Action.from_partial(obj))
    assert full_obj.id is not None
    assert full_obj.type == ActionType.DUPLICATE
    assert full_obj.is_shallow is True
    assert full_obj.text == Text.plain("hello bench!")
    assert full_obj.code == Code.from_string("print('hello bench!')")


def test_partial_node_generic(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial generic node."""
    typ = Type(kind=TypeKind.PARTIAL_OBJECT)
    obj = CustomObject.new({}, typ)

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
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = cast(Field, Node.from_partial(obj))
    assert full_obj.id is not None
    assert full_obj.name == "Option1"


def test_partial_node_with_nested_value_packed(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial node with a nested value packed property (CreateAction)."""
    obj = Action.partial(type=ActionType.CREATE)
    typ = obj._type

    # should be init to given/empty values
    assert obj.id is None
    assert obj.parent is None
    assert obj.type == ActionType.CREATE
    assert obj.node_partial is None

    # set/get nested value
    node_partial = Record.partial(block=Block.new(BlockType.DATABASE, "Database1"))
    obj.node_partial = node_partial
    assert obj.node_partial == node_partial

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj.equals(obj_unpacked)
    assert obj_unpacked.node_partial is not node_partial  # should be a different object instance

    # turn into full node
    full_obj = cast(CreateAction, Action.from_partial(obj, name="CreateAction1"))
    assert full_obj.id is not None
    assert full_obj.name == "CreateAction1"
    assert full_obj.type == ActionType.CREATE
    assert full_obj.node_partial == node_partial


def test_roundtrip_scalar_value(session: Session, package: Package) -> None:
    """Pack/unpack a scalar value inside a (Variable) Block (which HasValues)."""

    # first set in constructor
    type_info = to_type_scalar(PrimitiveType.INT32)
    block = Block.new(VariableBlock, name="Variable1", value_type=type_info, value=7)
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
        base_field_types=[FieldType.MEMBER],
        kind=TypeKind.CUSTOM_OBJECT,
    )

    # outer value
    value = CustomObject.new({}, message1.to_type(of="value"))
    value.Field1 = choice1.fields.Option1
    assert value.Field1 is choice1.fields.Option1
    value.Field2 = [24]
    value.Field3 = Text.plain("hello bench!")
    value.Field4 = CustomObject.new({}, message2.to_type(of="value"))

    value_packed = pack_value(value, message1.to_type(of="value"), wrap_scalar=True)
    unpacked_value = unpack_value(value_packed, message1.to_type(of="value"), wrap_scalar=True)
    assert unpacked_value == value


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object_value_data(
    obj: BuiltinObject, session: Session, package: Package
):
    packed_wire_obj = wiring.pack_builtin_object(obj)
    packed_json = pack_builtin_object_data(packed_wire_obj)
    unpacked_wire_obj = unpack_builtin_object_data(packed_json)
    unpacked_obj = wiring.unpack_builtin_object(
        unpacked_wire_obj,
        supergraph=session._supergraph,
        graph=session._graph,
        session=session,
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=structs)
@examples([{"obj": obj} for obj in STRUCTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object_value(obj: BuiltinObject, session: Session, package: Package):
    packed_json = pack_builtin_object(obj)
    unpacked_obj = unpack_builtin_object(
        packed_json, session=session, supergraph=session._supergraph
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"


# TODO :Test: auto generate :Test types & values

#
# Sampling
#

UNGENERATABLE_STRUCT_TYPES = [
    # Edit/Change have old/new_node_packed data
    StructType.EDIT,
    StructType.EDIT_OPERATION,
    StructType.CHANGE,
    # nodes have special handling
    StructType.NODE_REFERENCE,
    StructType.PROPERTY_REFERENCE,
    # contain required references
    StructType.OBJECT_MAPPING,
    StructType.CALL,
]


def test_sample_value_scalar(session: Session, package: Package):
    typ = Type.from_type(bool)
    val = sample_value(typ)
    check_value(val, typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise)


def test_sample_value_scalar_constrained(session: Session, package: Package):
    typ = Type.from_type(int, constraint=constraint(min_value=10.0, max_value=20.0))
    val = sample_value(typ)
    check_value(val, typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise)


@pytest.mark.parametrize(
    "struct_type",
    [st for st in STRUCT_TYPES if st not in UNGENERATABLE_STRUCT_TYPES],
    ids=lambda t: t.bench_name,
)
def test_sample_value_struct(struct_type: StructType, session: Session, package: Package):
    typ = Type(kind=TypeKind.STRUCT, bench_type=struct_type)
    val = sample_value(typ)
    check_value(val, typ, options=DEFAULT_CHECK_OPTIONS, invalid=on_invalid_raise)


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

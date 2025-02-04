import math
from datetime import datetime
from typing import cast

import pytest
import pytz
from hypothesis import HealthCheck, given, settings

from bench.language import (
    Action,
    ActionType,
    Block,
    BuiltinObject,
    Class,
    Code,
    CreateAction,
    CustomObject,
    Database,
    DuplicateAction,
    Field,
    FieldType,
    Flow,
    Message,
    MessageType,
    Node,
    NodeType,
    Package,
    PrimitiveType,
    Record,
    Session,
    Text,
    Type,
    TypeKind,
    coerce_custom_object_scalar,
    pack_builtin_object,
    pack_builtin_object_data,
    pack_custom_object,
    unpack_builtin_object,
    unpack_builtin_object_data,
    unpack_custom_object,
)
from bench.proto import wiring
from bench.test.strategies import builtin_objects, examples, structs
from bench.test.unit.conftest import BUILTIN_OBJECTS, STRUCTS


def test_custom_object_with_builtin_properties(session: Session, package: Package) -> None:
    """Coerce, pack & unpack custom object with builtin properties."""
    Flow1 = Flow.new(
        "Flow1",
        Field.output("Output1", PrimitiveType.INT32),
        Field.output("Output 2 with a Space", Text),
    )

    # coerce
    Flow1Output = Flow1.to_type_maybe(field_types=[FieldType.OUTPUT])
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
    message_type = Class.new(
        "MyMessage",
        Field.member("Field1", int),
        Field.member("Field2", Block),
        Field.member("Field3", bool),
        Field.member("Field4", datetime),
    )
    typ = Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=NodeType.MESSAGE, base_type=message_type)
    obj = CustomObject.new({}, typ)

    # should be init to empty/default values for Message
    assert obj.id is None
    assert obj.type is Message.get_property("type").default
    assert obj.block is None
    assert obj.Field1 is None
    assert obj.Field4 is None

    # set/get values on value and properties
    obj.Field1 = 42
    obj.title = "My New Message"
    obj.block = message_type
    obj.Field4 = datetime(2024, 1, 1, tzinfo=pytz.utc)
    assert obj.Field1 == 42
    assert obj.block == message_type
    assert obj.title == "My New Message"
    assert obj.Field4 == datetime(2024, 1, 1, tzinfo=pytz.utc)

    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = Message.from_partial(obj)
    assert full_obj.id is not None
    assert full_obj.title == "My New Message"
    assert full_obj.value
    assert full_obj.value.Field1 == 42
    assert full_obj.value.Field4 == datetime(2024, 1, 1, tzinfo=pytz.utc)


def test_partial_node_message_extraneous_property(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Message node with extraneous kwargs (should error)."""
    message_type = Class.new("MyMessage", Field.member("Field1", int))
    _ = Message.partial(type=MessageType.TEXT, block=message_type, Field1=42)
    with pytest.raises(ValueError):
        _ = Message.partial(
            type=MessageType.TEXT, block=message_type, Field1=42, my_extraneous_something="value"
        )


def test_partial_node_action(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Action node with subtypes."""
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
    node_partial = Record.partial(database=Database.new("Database1"))
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


def test_partial_node_coerce(session: Session, package: Package) -> None:
    """Coerce a partial node with a subtype value."""
    obj = Action.partial()
    obj.type = ActionType.TYPE
    obj.string = "Hello World!"
    obj_coerced = coerce_custom_object_scalar({**obj}, obj._type)
    assert obj_coerced == obj


def test_unpack_custom_object(session: Session, package: Package) -> None:
    """Unpack a custom object with a nested value."""

    Action1 = Action.new(
        ActionType.CODE,
        "Action1",
        fields=[
            Field.input("Input1", int),
            Field.input("Input2", float),
            Field.input("Input3", str),
            Field.output("Output1", int),
        ],
    )
    input_type = Action1.input_type_field_only
    assert input_type is not None, f"no input_type for {Action1!r}"
    obj = coerce_custom_object_scalar(
        {
            "Input1": 42,
            "Input2": math.pi,
            "Input3": "hello bench!",
        },
        input_type,
    )
    assert obj.Input1 == 42

    obj_unpacked = {**obj}
    assert obj_unpacked == {
        "Input1": 42,
        "Input2": math.pi,
        "Input3": "hello bench!",
    }


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

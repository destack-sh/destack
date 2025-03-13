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
    Channel,
    Class,
    CustomObject,
    Field,
    FieldType,
    Flow,
    Message,
    MessageType,
    Node,
    NodeType,
    Package,
    PrimitiveType,
    Session,
    Text,
    Type,
    TypeKind,
    coerce_custom_object_scalar,
    pack_builtin_object,
    pack_builtin_object_data,
    pack_custom_object,
    text_line,
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
    message_type = Class.new(
        "MyMessage",
        Field.member("Field1", int),
        Field.member("Field2", Block),
        Field.member("Field3", bool),
        Field.member("Field4", datetime),
    )
    Channel1 = Channel.new("Channel1")
    typ = Type(kind=TypeKind.PARTIAL_OBJECT, bench_type=NodeType.MESSAGE, base_type=message_type)
    obj = CustomObject.new({}, typ)

    # should be init to empty/default values for Message
    assert obj.id is None
    assert obj.type is Message.get_property("type").default
    assert obj.clazz is None
    assert obj.Field1 is None
    assert obj.Field4 is None
    assert obj.channel is None
    # set/get values on value and properties
    obj.Field1 = 42
    obj.title = text_line("My New Message")
    obj.clazz = message_type
    obj.Field4 = datetime(2024, 1, 1, tzinfo=pytz.utc)
    obj.channel = Channel1
    assert obj.Field1 == 42
    assert obj.clazz == message_type
    assert obj.title == text_line("My New Message")
    assert obj.Field4 == datetime(2024, 1, 1, tzinfo=pytz.utc)
    assert obj.channel == Channel1
    # pack/unpack
    obj_packed = pack_custom_object(obj, typ)
    obj_unpacked = unpack_custom_object(obj_packed, typ, supergraph=session._supergraph)
    assert obj_unpacked.equals(obj)

    # turn into full node
    full_obj = Message.from_partial(obj)
    assert full_obj.id is not None
    assert full_obj.title == text_line("My New Message")
    assert full_obj.value
    assert full_obj.value.Field1 == 42
    assert full_obj.value.Field4 == datetime(2024, 1, 1, tzinfo=pytz.utc)


def test_partial_node_message_extraneous_property(session: Session, package: Package) -> None:
    """Create, update, pack/unpack a partial Message node with extraneous kwargs (should error)."""
    message_type = Class.new("MyMessage", Field.member("Field1", int))
    _ = Message.partial(type=MessageType.REGULAR, clazz=message_type, Field1=42)
    with pytest.raises(ValueError):
        _ = Message.partial(
            type=MessageType.REGULAR, block=message_type, Field1=42, my_extraneous_something="value"
        )


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
    obj.type = FieldType.INPUT
    obj.kind = TypeKind.PRIMITIVE
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
    input_type = Action1.input_type
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

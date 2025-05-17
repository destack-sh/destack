import math

from hypothesis import HealthCheck, given, settings

from bench.language import (
    Action,
    ActionType,
    BuiltinObject,
    Field,
    FieldType,
    Flow,
    Package,
    PrimitiveType,
    Session,
    Text,
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

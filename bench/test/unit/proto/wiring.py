from fastuuid import uuid4
from hypothesis import HealthCheck, given, settings

from bench.language import BuiltinObject, NodeReference, NodeType, Session
from bench.proto import AnyObjectData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS


def test_roundtrip_node_reference_bytes():
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference(
        node_type=NodeType.PAGE, id=uuid4(), ck=uuid4(), bench_id=uuid4(), base_id=uuid4()
    )
    node_ref_data = node_ref.to_proto()
    assert node_ref._proto is node_ref_data  # cached
    assert node_ref.to_proto() is node_ref_data  # cached
    node_ref_data_bytes = node_ref_data.SerializeToString()
    unpacked_node_ref_data = type(node_ref_data)()
    unpacked_node_ref_data.ParseFromString(node_ref_data_bytes)
    unpacked_node_ref = NodeReference.__unpack_proto__(unpacked_node_ref_data)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"
    assert unpacked_node_ref._proto is unpacked_node_ref_data  # cached
    assert unpacked_node_ref.to_proto() is unpacked_node_ref_data  # cached


def test_roundtrip_node_reference_value():
    """Pack and unpack a NodeReference."""
    node_ref = NodeReference()


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object_bytes(obj: BuiltinObject[AnyObjectData], session: Session):
    packed_obj_data: AnyObjectData = obj.to_proto()
    packed_bytes = packed_obj_data.SerializeToString()
    unpacked_obj_data = type(packed_obj_data)()
    unpacked_obj_data.ParseFromString(packed_bytes)
    unpacked_obj = obj.__unpack_proto__(unpacked_obj_data)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"

from fastuuid import uuid4
from hypothesis import HealthCheck, given, settings

from bench.language import BuiltinObject, NodeReference, NodeType, Session, User
from bench.proto import AnyObjectData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS


def test_roundtrip_node_reference_proto():
    """Pack and unpack a NodeReference as proto."""
    node_ref = NodeReference(
        node_type=NodeType.PAGE, id=uuid4(), ck=uuid4(), bench_id=uuid4(), base_id=uuid4()
    )
    node_ref_data = node_ref.to_proto()
    assert node_ref.to_proto() is node_ref_data  # cached (frozen Struct)
    node_ref_data_bytes = node_ref_data.SerializeToString()
    unpacked_node_ref_data = type(node_ref_data)()
    unpacked_node_ref_data.ParseFromString(node_ref_data_bytes)
    unpacked_node_ref = NodeReference.from_proto(unpacked_node_ref_data)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"
    assert unpacked_node_ref.to_proto() is unpacked_node_ref_data  # cached (frozen Struct)


def test_roundtrip_node_reference_value():
    """Pack and unpack a NodeReference as value."""
    node_ref = NodeReference(
        node_type=NodeType.PAGE, id=uuid4(), ck=uuid4(), bench_id=uuid4(), base_id=uuid4()
    )
    node_ref_value = node_ref.to_value()
    unpacked_node_ref = NodeReference.from_value(node_ref_value)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"


def test_roundtrip_user_proto():
    """Pack and unpack a User as proto."""
    user = User()
    user_data = user.to_proto()
    assert user.to_proto() is user_data  # cached (frozen Struct)


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

from hypothesis import HealthCheck, given, settings

from bench.language import BuiltinObject, Session
from bench.proto import wiring
from bench.proto.wire import AnyObjectData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object_bytes(obj: BuiltinObject[AnyObjectData], session: Session):
    packed_obj_data: AnyObjectData = wiring.pack_builtin_object(obj)
    packed_bytes = packed_obj_data.SerializeToString()
    unpacked_obj_data = type(packed_obj_data)()
    unpacked_obj_data.ParseFromString(packed_bytes)
    unpacked_obj = wiring.unpack_builtin_object(
        unpacked_obj_data,
        supergraph=session._supergraph,
        graph=session._graph,
        session=session,
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"

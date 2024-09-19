from hypothesis import given

from bench.language import BuiltinObject, Session
from bench.proto import wiring
from bench.proto.wire import AnyNodeData, AnyStructData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_builtin_object_bytes(
    obj: BuiltinObject[AnyNodeData | AnyStructData], shared_session: Session
):
    packed_obj_data: AnyNodeData | AnyStructData = wiring.pack_object(obj)
    packed_bytes = packed_obj_data.SerializeToString()
    unpacked_obj_data = type(packed_obj_data)()
    unpacked_obj_data.ParseFromString(packed_bytes)
    unpacked_obj = wiring.unpack_object(
        unpacked_obj_data,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"

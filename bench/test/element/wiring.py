from hypothesis import given

from bench.language import BuiltinObject, Session
from bench.proto import wiring
from bench.test.element.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE
from bench.test.strategies import builtin_objects, examples


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_bytes(obj: BuiltinObject, shared_session: Session):
    packed_obj_data = wiring.pack_object(obj)
    packed_bytes = bytes(packed_obj_data)
    unpacked_obj_data = type(packed_obj_data)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_object(
        unpacked_obj_data,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_json(obj: BuiltinObject, shared_session: Session):
    packed_obj_data = wiring.pack_object(obj)
    packed_json = packed_obj_data.to_json(indent=2)
    unpacked_obj_data = type(packed_obj_data)().from_json(packed_json)
    unpacked_obj = wiring.unpack_object(
        unpacked_obj_data,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_copy(obj: BuiltinObject, shared_session: Session):
    packed_obj_data = wiring.pack_object(obj)
    copied_obj_data = wiring.copy_struct(packed_obj_data)
    unpacked_obj = wiring.unpack_object(
        copied_obj_data,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"
    # (we want to check both assertions but the first is easier to debug)
    assert copied_obj_data == packed_obj_data, f"{copied_obj_data!r} != {packed_obj_data!r}"

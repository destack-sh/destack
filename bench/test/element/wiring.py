from hypothesis import given

from bench.language import BuiltinObject, Session
from bench.proto import wiring
from bench.test.element.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE
from bench.test.strategies import builtin_objects, examples


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_bytes(obj: BuiltinObject, shared_session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    packed_bytes = bytes(packed_wire_obj)
    unpacked_wire_obj = type(packed_wire_obj)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_object(
        unpacked_wire_obj,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_json(obj: BuiltinObject, shared_session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    packed_json = packed_wire_obj.to_json(indent=2)
    unpacked_wire_obj = type(packed_wire_obj)().from_json(packed_json)
    unpacked_obj = wiring.unpack_object(
        unpacked_wire_obj,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_wire_copy(obj: BuiltinObject, shared_session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    copied_obj = wiring.copy_struct(packed_wire_obj)
    assert copied_obj == packed_wire_obj, f"{copied_obj!r} != {packed_wire_obj!r}"
    unpacked_obj = wiring.unpack_object(
        copied_obj,
        supergraph=shared_session._supergraph,
        graph=shared_session._graph,
        session=shared_session,
    )
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"

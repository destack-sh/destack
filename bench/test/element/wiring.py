from hypothesis import given

from bench.language import BuiltinObject, Session
from bench.proto import wiring
from bench.test.strategies import builtin_objects


@given(obj=builtin_objects())
def test_roundtrip_bytes(obj: BuiltinObject, session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    packed_bytes = bytes(packed_wire_obj)
    unpacked_wire_obj = type(packed_wire_obj)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_object(unpacked_wire_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
def test_roundtrip_json(obj: BuiltinObject, session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    packed_json = packed_wire_obj.to_json(indent=2)
    unpacked_wire_obj = type(packed_wire_obj)().from_json(packed_json)
    unpacked_obj = wiring.unpack_object(unpacked_wire_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
def test_roundtrip_copy(obj: BuiltinObject, session: Session):
    packed_wire_obj = wiring.pack_object(obj)
    copied_obj = wiring.copy_struct(packed_wire_obj)
    assert copied_obj == packed_wire_obj, f"{copied_obj!r} != {packed_wire_obj!r}"
    unpacked_obj = wiring.unpack_object(copied_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"

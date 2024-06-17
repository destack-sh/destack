from hypothesis import given

from bench.language import BuiltinObject
from bench.language.const import StructType
from bench.language.test.strategies import builtin_objects, from_object_type
from bench.proto import wiring


@given(value=from_object_type(StructType.POLICY))
def test_nocheckin_identity(value: BuiltinObject):
    assert value._equals_content(value), f"{value!r} != {value!r}"


@given(obj=builtin_objects())
def test_roundtrip_bytes(obj: BuiltinObject):
    packed_wire_obj = wiring.pack_object(obj)
    packed_bytes = bytes(packed_wire_obj)
    unpacked_wire_obj = type(packed_wire_obj)().parse(packed_bytes)
    unpacked_obj = wiring.unpack_object(unpacked_wire_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
def test_roundtrip_json(obj: BuiltinObject):
    packed_wire_obj = wiring.pack_object(obj)
    packed_json = packed_wire_obj.to_json(indent=2)
    unpacked_wire_obj = type(packed_wire_obj)().from_json(packed_json)
    unpacked_obj = wiring.unpack_object(unpacked_wire_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"


@given(obj=builtin_objects())
def test_roundtrip_copy(obj: BuiltinObject):
    packed_wire_obj = wiring.pack_object(obj)
    copied_obj = wiring.copy_struct(packed_wire_obj)
    assert copied_obj == packed_wire_obj, f"{copied_obj!r} != {packed_wire_obj!r}"
    unpacked_obj = wiring.unpack_object(copied_obj, supergraph=None)
    assert unpacked_obj._equals_content(obj), f"{unpacked_obj!r} != {obj!r}"

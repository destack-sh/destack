from datetime import datetime, timedelta

import pytz
from betterproto import _Duration as ProtoDuration
from betterproto import _Timestamp as ProtoTimestamp
from hypothesis import given
from hypothesis import strategies as st

from bench.language import BuiltinObject, Session
from bench.language.const import PrimitiveType
from bench.language.value import MAX_VALUE_BY_PRIMITIVE_TYPE, MIN_VALUE_BY_PRIMITIVE_TYPE
from bench.proto import wiring
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS_OF_EVERY_TYPE


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS_OF_EVERY_TYPE])
def test_roundtrip_builtin_object_bytes(obj: BuiltinObject, shared_session: Session):
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
def test_roundtrip_builtin_object_json(obj: BuiltinObject, shared_session: Session):
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
def test_roundtrip_builtin_object_copy(obj: BuiltinObject, shared_session: Session):
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


# NOTE :Test: we manually test time values since they are converted into proto-specific structures
#  with different precision and timezone handling


@given(
    value=st.timedeltas(
        min_value=MIN_VALUE_BY_PRIMITIVE_TYPE[PrimitiveType.INTERVAL],
        max_value=MAX_VALUE_BY_PRIMITIVE_TYPE[PrimitiveType.INTERVAL],
    )
)
def test_roundtrip_timedelta(value: timedelta):
    packed_value_data = ProtoDuration.from_timedelta(value)
    unpacked_value = packed_value_data.to_timedelta()
    assert unpacked_value == value, f"{unpacked_value!r} != {value!r}"


@given(value=st.datetimes(timezones=st.just(pytz.utc)))
def test_roundtrip_datetime(value: datetime):
    packed_value_data = ProtoTimestamp.from_datetime(value)
    unpacked_value = packed_value_data.to_datetime()
    assert unpacked_value == value, f"{unpacked_value!r} != {value!r}"

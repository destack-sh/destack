import json

from bench.language import (
    Aggregation,
    AggregationType,
    BuiltinObjectBase,
    Cursor,
    JoinType,
    Message,
    NodeReference,
    NodeType,
    Query,
    Session,
    Thread,
    User,
    UserStatus,
    attribute_ref,
    join,
)
from bench.proto import AnyObjectData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS
from fastuuid import uuid4
from hypothesis import HealthCheck, given, settings


def test_roundtrip_node_reference():
    """Pack and unpack a NodeReference as proto and value."""
    node_ref = NodeReference(
        node_type=NodeType.PAGE, id=uuid4(), bench_id=uuid4(), definition_id=uuid4()
    )

    # proto
    node_ref_data = node_ref.to_proto()
    assert node_ref.to_proto() is node_ref_data  # cached (frozen Struct)
    node_ref_data_bytes = node_ref_data.SerializeToString()
    unpacked_node_ref_data = type(node_ref_data)()
    unpacked_node_ref_data.ParseFromString(node_ref_data_bytes)
    unpacked_node_ref = NodeReference.from_proto(unpacked_node_ref_data)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"
    assert unpacked_node_ref.to_proto() is unpacked_node_ref_data  # cached (frozen Struct)

    # value
    node_ref_value = node_ref.to_value()
    node_ref_value_str = json.dumps(node_ref_value, indent=2)
    print(node_ref_value_str)  # noqa: T201
    unpacked_node_ref_value = json.loads(node_ref_value_str)
    unpacked_node_ref = NodeReference.from_value(unpacked_node_ref_value)
    assert unpacked_node_ref.equals(node_ref), f"{unpacked_node_ref!r} != {node_ref!r}"


def test_roundtrip_query_proto(session: Session):
    """Pack and unpack a Query as proto."""
    query = Thread.search(
        sort=[Thread.property("created_at").asc()],
        limit=25,
        count=True,
        Cursor=Cursor.get(
            join=join(JoinType.LEFT, on=Cursor.property("owned_by").eq(5)),
            UnreadCount=Message.aggregate(
                where=Message.property("read_at").greater_than(
                    attribute_ref("Cursor.last_read_at")
                ),
                aggregation=Aggregation(type=AggregationType.COUNT),
            ),
        ),
    )

    # proto
    query_data = query.to_proto()
    query_data_bytes = query_data.SerializeToString()
    unpacked_query_data = type(query_data)()
    unpacked_query_data.ParseFromString(query_data_bytes)
    unpacked_query = Query.from_proto(unpacked_query_data)
    assert unpacked_query.equals(query), f"{unpacked_query!r} != {query!r}"

    # value
    query_value = query.to_value()
    query_value_str = json.dumps(query_value, indent=2)
    print(query_value_str)  # noqa: T201
    unpacked_query_value = json.loads(query_value_str)
    unpacked_query = Query.from_value(unpacked_query_value)
    assert unpacked_query.equals(query), f"{unpacked_query!r} != {query!r}"


def test_roundtrip_user_proto(session: Session):
    """Pack and unpack a User as proto."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Florian",
        slug="florian",
        bench_ptr=NodeReference(id=uuid4(), node_type=NodeType.BENCH),
    )

    # proto
    user_data = user.to_proto()
    user_data_bytes = user_data.SerializeToString()
    unpacked_user_data = type(user_data)()
    unpacked_user_data.ParseFromString(user_data_bytes)
    unpacked_user = User.from_proto(unpacked_user_data)
    assert unpacked_user.equals(user), f"{unpacked_user!r} != {user!r}"

    # value
    user_value = user.to_value()
    user_value_str = json.dumps(user_value, indent=2)
    print(user_value_str)  # noqa: T201
    unpacked_user_value = json.loads(user_value_str)
    unpacked_user = User.from_value(unpacked_user_value)
    assert unpacked_user.equals(user), f"{unpacked_user!r} != {user!r}"


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_roundtrip_builtin_object(obj: BuiltinObjectBase[AnyObjectData], session: Session):
    # proto
    packed_obj_data: AnyObjectData = obj.to_proto()
    packed_bytes = packed_obj_data.SerializeToString()
    unpacked_obj_data = type(packed_obj_data)()
    unpacked_obj_data.ParseFromString(packed_bytes)
    unpacked_obj = obj.__unpack_proto__(unpacked_obj_data)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"

    # value
    packed_obj_value = obj.to_value()
    packed_obj_value_str = json.dumps(packed_obj_value, indent=2)
    unpacked_obj_value = json.loads(packed_obj_value_str)
    unpacked_obj = obj.from_value(unpacked_obj_value)
    assert unpacked_obj.equals(obj), f"{unpacked_obj!r} != {obj!r}"

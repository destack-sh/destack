from hypothesis import HealthCheck, given, settings

from bench.language import BuiltinObjectBase, Session, Thread
from bench.pb2 import AnyObjectData
from bench.test.strategies import builtin_objects, examples
from bench.test.unit.conftest import BUILTIN_OBJECTS


def test_repr_query():
    query = Thread.search(sort=[Thread.property("created_at").asc()], limit=25, count=True)
    print(repr(query))  # noqa: T201


@given(obj=builtin_objects())
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_repr_builtin_object(obj: BuiltinObjectBase[AnyObjectData], session: Session):
    print(repr(obj))  # noqa: T201

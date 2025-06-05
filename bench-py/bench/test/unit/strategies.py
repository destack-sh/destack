from hypothesis import HealthCheck, Phase, given, settings

from bench.language import BuiltinObjectBase, Session
from bench.test.strategies import builtin_objects


@given(obj=builtin_objects())
@settings(phases=(Phase.generate,), suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_generate_builtin_objects(obj: BuiltinObjectBase, session: Session):
    assert True  # the 'test' is in the fixture

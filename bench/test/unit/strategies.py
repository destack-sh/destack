from hypothesis import HealthCheck, Phase, given, settings

from bench.language import BuiltinObject, Session
from bench.test.strategies import builtin_objects

#
# NOTE: this file is about *testing* the hypothesis strategies,
#  not about *defining* them (see bench/test/stategies.py for that)
#


@given(obj=builtin_objects())
@settings(phases=(Phase.generate,), suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_generate_builtin_objects(obj: BuiltinObject, session: Session):
    assert True  # the 'test' is in the fixture

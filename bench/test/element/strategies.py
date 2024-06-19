# NOTE: this file is about *testing* the strategies (not defining them)
#  see bench/test/strategies.py for the actual, shared strategies


from hypothesis import given

from bench.language.node import BuiltinObject
from bench.language.session import Session
from bench.test.strategies import builtin_objects


@given(obj=builtin_objects())
def test_generate_builtin_objects(obj: BuiltinObject, shared_session: Session):
    assert True

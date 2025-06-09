import pytest
from hypothesis import HealthCheck, given, settings

from destack.language import (
    BuiltinObjectBase,
    Folder,
    Session,
)
from destack.test.strategies import examples, structs
from destack.test.unit.conftest import BUILTIN_OBJECTS


@given(obj=structs)
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_builtin_object_clone(obj: BuiltinObjectBase, session: Session):
    obj_clone = obj.clone()
    assert obj_clone.equals(obj)


def test_create_circular_node_ancestry(session: Session, package: Folder):
    """Create a circular node ancestry. Should fail."""
    Folder1 = Folder(name="Folder1")
    with pytest.raises(ValueError):
        Folder1.add_child(Folder1)
    package.add_child(Folder1)

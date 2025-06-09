import pytest
from hypothesis import HealthCheck, given, settings

from destack.language import (
    BuiltinObjectBase,
    Field,
    Package,
    Page,
    Schema,
    Session,
    title,
)
from destack.test.simulation.core import Simulation
from destack.test.simulation.workload import RuntimeLambdaWorkload
from destack.test.strategies import examples, structs
from destack.test.unit.conftest import BUILTIN_OBJECTS, simulated_runtime


@given(obj=structs)
@examples([{"obj": obj} for obj in BUILTIN_OBJECTS])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_builtin_object_clone(obj: BuiltinObjectBase, session: Session):
    obj_clone = obj.clone()
    assert obj_clone.equals(obj)


@simulated_runtime()
async def test_add_detached_subtree(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    schema = Schema(name="Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        schema.add_child(Field(name=letter))
    runtime.page().add_child(schema)
    await runtime.commit()


@simulated_runtime()
async def test_clone(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Clone a Node subtree."""
    schema = Schema(name="Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        schema.add_child(Field(name=letter))
    runtime.page().add_child(schema)
    await runtime.commit()

    schema_clone = schema.clone()
    for field, field_clone in zip(schema.get_children(Field), schema_clone.get_children(Field)):
        assert field is not field_clone
        assert field.id != field_clone.id
        assert field.equals(field_clone)
    await runtime.commit()


def test_create_circular_node_ancestry(session: Session, package: Package):
    """Create a circular node ancestry. Should fail."""
    Page1 = Page(title=title("Page1"))
    with pytest.raises(ValueError):
        Page1.add_child(Page1)
    package.add_child(Page1)

import pytest
from hypothesis import HealthCheck, given, settings

from bench.language import (
    Action,
    ActionType,
    Block,
    BuiltinObject,
    Field,
    Flow,
    Package,
    Page,
    Schema,
    Session,
    Table,
    Text,
)
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.strategies import structs
from bench.test.unit.conftest import simulated_runtime


@given(obj=structs)
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
def test_builtin_object_clone(obj: BuiltinObject, session: Session):
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


@simulated_runtime()
async def test_clone_with_cross_references(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Clone consistency test with references."""
    schema = Schema(name="Letter")
    schema.add_children(Field(name="A"), Field(name="B"))
    flow = Flow(name="Flow")
    action = Action(
        type=ActionType.CODE,
        name="Action",
    )
    action.add_children(
        Field(name="Text", type=Text),
        Field(name="Schema", type=schema),
    )
    flow.add_child(action)
    table = Table(name="Table")
    table.add_children(Field(name="Text", type=Text))
    page = runtime.page()
    schema_block = page.add_child(schema)
    flow_block = page.add_child(flow)
    _ = page.add_child(table)
    await runtime.commit()

    # cloning an inline node should be consistent with its definition counterpart
    schema_block_clone = schema_block.clone()
    assert schema_block_clone.node is not schema
    assert schema_block_clone.get_node_as(Schema).definition is schema_block_clone
    # other way around
    flow_clone = flow.clone()
    assert flow_clone.definition is not None
    assert flow_clone.definition is not flow_block
    assert flow_clone.definition.get_node_as(Flow) is flow_clone

    # references should be consistent within new subtree
    page_clone = page.clone()
    flow_clone = page_clone.child(Block, "Flow").get_node_as(Flow)
    assert flow_clone is not None
    schema_clone = page_clone.child(Block, "Letter").get_node_as(Schema)
    assert schema_clone is not None
    action_clone = flow_clone.child(Action, "Action")
    assert action_clone is not None
    assert action_clone.child(Field, "Schema").base_type == schema_clone
    table_clone = page_clone.child(Block, "Table").get_node_as(Table)
    assert table_clone is not None
    await runtime.commit()


@simulated_runtime()
async def test_instance(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Instance a subtree."""
    schema = Schema(name="Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        schema.add_child(Field(name=letter))
    runtime.page().add_child(schema)
    await runtime.commit()

    schema_instance = schema.instance()
    assert schema_instance.equals(schema)
    assert schema_instance.id != schema.id
    assert schema_instance.template_id == schema.id
    for field, field_instance in zip(
        schema.get_children(Field), schema_instance.get_children(Field)
    ):
        assert field_instance.id != field.id
        assert field_instance.ck == field.ck
        assert field_instance.template_id == field.id
        assert field.equals(field_instance)
    await runtime.commit()


@simulated_runtime()
async def test_instance_with_cross_references(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Instance a subtree with cross references."""
    schema = Schema(name="Letter")
    schema.add_children(Field(name="A"), Field(name="B"))
    flow = Flow(name="Flow")
    action = Action(type=ActionType.CODE, name="Action")
    action.add_children(
        Field(name="Text", type=Text),
        Field(name="Schema", type=schema),
    )
    flow.add_child(action)
    table = Table(name="Table")
    table.add_children(Field(name="Text", type=Text))
    page = runtime.page()
    _ = page.add_child(schema)
    _ = page.add_child(flow)
    _ = page.add_child(table)
    await runtime.commit()

    # references should be consistent within new subtree
    page_instance = page.instance()
    flow_instance = page_instance.child(Block, "Flow").get_node_as(Flow)
    assert flow_instance is not None
    assert flow_instance is not flow
    assert flow_instance.template is flow
    schema_instance = page_instance.child(Block, "Letter").get_node_as(Schema)
    assert schema_instance is not None
    assert schema_instance is not schema
    assert schema_instance.template is schema
    action_instance = flow_instance.child(Action, "Action")
    assert action_instance is not None
    assert action_instance is not action
    assert action_instance.template is action
    assert action_instance.child(Field, "Schema").base_type == schema_instance
    table_instance = page_instance.child(Block, "Table").get_node_as(Table)
    assert table_instance is not None
    assert table_instance is not table
    assert table_instance.template is table
    await runtime.commit()


@simulated_runtime()
async def test_move_subtree(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Move Nodes between parents (within a Package)."""
    Page1 = runtime.page("Page1")
    Page2 = runtime.page("Page2")
    Block1 = Page1.add_child(Schema(name="Block1"))
    Block1.add_children(Field(name="A"), Field(name="B"))
    Block2 = Page1.add_child(Flow(name="Block2"))
    Block2.add_children(Field(name="Text", type=Text), Field(name="Schema", type=Block1))
    Block3 = Page1.add_child(Table(name="Block3"))
    Block3.add_children(Field(name="Text", type=Text))
    await runtime.commit()

    # can't just append directly
    with pytest.raises(ValueError):
        Page2.add_child(Block3)

    # move Block1 to Page2
    Block1.move(to=Page2)
    await runtime.commit()
    assert Page1.get_children(Block) == [Block2, Block3]
    assert Page2.get_children(Block) == [Block1]

    # move Block1 back to Page1
    Block1.move(to=Page1)
    await runtime.commit()
    assert Page1.get_children(Block) == [Block2, Block3, Block1]
    assert Page2.get_children(Block) == []

    # move all blocks to Page2
    Block1.move(to=Page2)
    Block2.move(to=Page2)
    Block3.move(to=Page2)
    await runtime.commit()
    assert Page1.get_children(Block) == []
    assert Page2.get_children(Block) == [Block1, Block2, Block3]


def test_create_circular_node_ancestry(session: Session, package: Package):
    """Create a circular node ancestry. Should fail."""
    Page1 = Page.new("Page1")
    with pytest.raises(ValueError):
        Page1.add_child(Page1)
    package.add_child(Page1)

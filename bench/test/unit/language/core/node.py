from typing import cast

import pytest
from hypothesis import HealthCheck, given, settings

from bench.language import (
    Action,
    ActionType,
    Bench,
    Block,
    BlockType,
    BuiltinObject,
    Client,
    ClientType,
    Code,
    Database,
    Field,
    Flow,
    Machine,
    Message,
    NodeReference,
    NodeType,
    Package,
    PackageType,
    Page,
    Schema,
    Session,
    Table,
    Text,
    TextLine,
    Thread,
)
from bench.proto import unpack_builtin_object
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.strategies import structs
from bench.test.unit.conftest import simulated_runtime


def test_init_with_non_existing_property(session: "Session"):
    with pytest.raises(AttributeError):
        _ = Code(_non_existing_property="wadabadaboo")  # type: ignore
    with pytest.raises(AttributeError):
        _ = Bench(slug="test", name="Test", _non_existing_property="wadabadaboo")  # type: ignore
    with pytest.raises(AttributeError):
        _ = Block.new(BlockType.PARAGRAPH, name="Test", _non_existing_property="wadabadaboo")  # type: ignore


def test_get_set_non_existing_property(session: "Session"):
    """Should raise properly"""
    node = Bench(slug="test", name="Test")
    with pytest.raises(AttributeError):
        node.wadabadaboo = "wadabadaboo"  # type: ignore
    with pytest.raises(AttributeError):
        _ = node.wadabadaboo  # type: ignore


def test_node_subtype_pack_unpack(session: "Session"):
    block = Block.new(BlockType.PARAGRAPH, line=TextLine.plain("Hello!"))
    # pack/unpack wiring
    block_data = block._to_data()
    unpacked_block = cast(
        Block, unpack_builtin_object(block_data, expect=Block, supergraph=session.supergraph)
    )
    assert unpacked_block.equals(block)
    assert unpacked_block.line is not None and unpacked_block.line.spans[0].content == "Hello!"


def test_node_pointers_consistency(session: "Session"):
    """Pointers should include the relevant bench/base/base_bench references."""
    bench_a = Bench(slug="testa", name="testb")
    assert bench_a.to_ref().equals(
        NodeReference(node_type=NodeType.BENCH, id=bench_a.id, ck=bench_a.ck, bench_id=bench_a.id)
    )
    package_a = Package(type=PackageType.OPEN, name="Main", slug="main")
    bench_a.add_child(package_a)
    bench_a.database = Database(name="Database")
    bench_a.add_child(bench_a.database)

    # sub bench, above package pointers
    assert package_a.bench_id == bench_a.id
    assert package_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.PACKAGE, id=package_a.id, ck=package_a.ck, bench_id=bench_a.id
        )
    )
    assert package_a.parent_ptr
    assert package_a.parent_ptr.id == bench_a.id

    # sub bench nested pointers
    machine_a = Machine(parent=package_a, name="Main")
    assert machine_a.bench_id == bench_a.id
    client_a = Client(
        parent=bench_a,
        seen_at=session.oracle.utc(),
        type=ClientType.MOBILE,
        name="Testificate's iPhone",
    )
    assert client_a.bench_id == bench_a.id
    assert client_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.CLIENT, id=client_a.id, ck=client_a.ck, bench_id=bench_a.id
        )
    )
    assert client_a.parent_ptr
    assert client_a.parent_ptr.bench_id == bench_a.id

    # sub package nested pointers
    package_a = Package(type=PackageType.OPEN, name="Main B", slug="main-b")
    bench_a.add_child(package_a)
    assert package_a.bench_id == bench_a.id
    page_a_1 = Page()
    package_a.add_child(page_a_1)
    assert page_a_1.bench_id == bench_a.id
    assert page_a_1.to_ref().equals(
        NodeReference(node_type=NodeType.PAGE, id=page_a_1.id, ck=page_a_1.ck, bench_id=bench_a.id)
    )

    # based pointers
    thread_a = Thread.new("Thread1")
    page_a_1.add_child(thread_a)
    message_a = Message(parent=thread_a)
    assert message_a.bench_id == bench_a.id
    assert message_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_a.id,
            ck=message_a.ck,
            bench_id=bench_a.id,
            base_id=thread_a.id,
        )
    )

    # refs pointing to different bench
    bench_b = Bench(slug="testb", name="testb")
    package_b = Package(type=PackageType.OPEN, name="Main B", slug="main-b")
    bench_b.add_child(package_b)
    bench_b.database = Database(name="Database")
    bench_b.add_child(bench_b.database)
    page_b = Page()
    package_b.add_child(page_b)
    assert page_b.bench_id == bench_b.id
    thread_b = Thread.new("Thread1")
    page_b.add_child(thread_b)
    message_b = Message(parent=thread_b)
    assert message_b.bench_id == bench_b.id
    assert message_b.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_b.id,
            ck=message_b.ck,
            bench_id=bench_b.id,
            base_id=thread_b.id,
        )
    )
    assert message_b.parent_ptr
    assert message_b.parent_ptr.bench_id == bench_b.id


#
# Trees
#


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
    schema = Schema.new("Letter", fields=[Field.new("A"), Field.new("B")])
    flow = Flow.new("Flow")
    action = Action.new(
        ActionType.CODE,
        "Action",
        fields=[Field.input("Text", Text), Field.output("Schema", schema)],
    )
    flow.add_child(action)
    table = Table.new("Table", fields=[Field.member("Text", Text)])
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
    schema = Schema.new("Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        schema.add_child(Field.new(letter))
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
    schema = Schema.new("Letter", fields=[Field.new("A"), Field.new("B")])
    flow = Flow.new("Flow")
    action = Action.new(
        ActionType.CODE,
        "Action",
        fields=[Field.input("Text", Text), Field.output("Schema", schema)],
    )
    flow.add_child(action)
    table = Table.new("Table", fields=[Field.member("Text", Text)])
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
    Block1 = Page1.add_child(Schema.new("Block1", fields=[Field.new("A"), Field.new("B")]))
    Block2 = Page1.add_child(
        Flow.new("Block2", fields=[Field.input("Text", Text), Field.output("Schema", Block1)])
    )
    Block3 = Page1.add_child(Table.new("Block3", fields=[Field.member("Text", Text)]))
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

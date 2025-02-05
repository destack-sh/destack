from itertools import chain
from typing import cast

import pytest
from hypothesis import HealthCheck, given, settings

from bench.language import (
    NODE_CLASSES,
    STRUCT_CLASSES,
    Action,
    ActionType,
    Bench,
    Block,
    BlockType,
    BuiltinObject,
    Choice,
    Class,
    Client,
    ClientType,
    Code,
    Database,
    DuplicateAction,
    Field,
    Flow,
    Machine,
    Message,
    NodeReference,
    NodeType,
    Package,
    PackageType,
    Page,
    Property,
    Session,
    Text,
    md,
)
from bench.proto import unpack_builtin_object
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.strategies import structs
from bench.test.unit.conftest import simulated_runtime


def test_builtin_object_properties_are_available():
    """Slightly tautological sanity check to check our introspected properties"""
    for cls in chain(STRUCT_CLASSES, NODE_CLASSES):
        for prop in cls.__properties__.values():
            if prop.is_introspectable:
                attr = getattr(cls, prop.name)
                assert type(attr) is Property, f"{prop!r}->{attr!r} is not a Property"


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


def test_node_subtype_property_access(session: "Session"):
    # subtype -> regular property
    Text1 = Block.new(BlockType.PARAGRAPH, text=md("Hello!"))
    assert Text1.text is not None and Text1.text.to_markdown() == "Hello!"
    Text1.text = md("Hello, world!")
    assert Text1.text is not None and Text1.text.to_markdown() == "Hello, world!"

    # subtype -> node ref property
    Duplicate1 = Action.new(DuplicateAction, "BlockAction1", node=Text1)
    assert Duplicate1.node == Text1
    assert Duplicate1.node_ptr == Text1.to_ref()
    Duplicate1.is_shallow = True
    assert Duplicate1.is_shallow


def test_node_subtype_property_reference(session: "Session"):
    Text1 = Block.new(BlockType.PARAGRAPH, text=md("Hello!"))
    Duplicate1 = Action.new(DuplicateAction, "BlockAction1", node=Text1)
    node_prop: Property = Duplicate1.get_property("node")
    assert node_prop.to_ref().resolve_or_error() is node_prop


def test_node_subtype_pack_unpack(session: "Session"):
    block = Block.new(BlockType.PARAGRAPH, text=md("Hello!"))
    # pack/unpack wiring
    block_data = block._to_data()
    unpacked_block = cast(
        Block, unpack_builtin_object(block_data, expect=Block, supergraph=session._supergraph)
    )
    assert unpacked_block.equals(block)
    assert unpacked_block.text is not None and unpacked_block.text.to_markdown() == "Hello!"


def test_node_pointers_consistency(session: "Session"):
    """Pointers should include the relevant bench/base/base_bench references."""
    bench_a = Bench(slug="testa", name="testb")
    assert bench_a.to_ref().equals(
        NodeReference(node_type=NodeType.BENCH, id=bench_a.id, ck=bench_a.ck, bench_id=bench_a.id)
    )
    bench_a.main_store = bench_a.stores.create(name="Store")

    # sub bench, above package pointers
    package_a = bench_a.packages.create(type=PackageType.ROOT, name="Main", slug="main")
    assert package_a.bench_id == bench_a.id
    assert package_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.PACKAGE, id=package_a.id, ck=package_a.ck, bench_id=bench_a.id
        )
    )
    assert package_a.parent_ptr
    assert package_a.parent_ptr.id == bench_a.id

    # sub bench nested pointers
    machine_a = Machine(parent=bench_a, name="Main")
    assert machine_a.bench_id == bench_a.id
    client_a = Client(
        parent=bench_a,
        seen_at=session._oracle.utc(),
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
    package_a = bench_a.packages.create(type=PackageType.ROOT, name="Main B", slug="main-b")
    assert package_a.bench_id == bench_a.id
    page_a_1 = package_a.pages.create()
    assert page_a_1.bench_id == bench_a.id
    assert page_a_1.to_ref().equals(
        NodeReference(node_type=NodeType.PAGE, id=page_a_1.id, ck=page_a_1.ck, bench_id=bench_a.id)
    )

    # based pointers
    class_a_1 = Class.new("Class1")
    page_a_1.append(class_a_1)
    message_a = Message(parent=bench_a, clazz=class_a_1)
    assert message_a.bench_id == bench_a.id
    assert message_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_a.id,
            ck=message_a.ck,
            bench_id=bench_a.id,
            base_ck=class_a_1.ck,
            base_bench_id=bench_a.id,
        )
    )

    # refs pointing to different bench
    bench_b = Bench(slug="testb", name="testb")
    bench_b.main_store = bench_b.stores.create(name="Store")
    package_b = bench_b.packages.create(type=PackageType.ROOT, name="Main B", slug="main-b")
    page_b = package_b.pages.create()
    assert page_b.bench_id == bench_b.id
    class_b_1 = Class.new("Class1")
    page_b.append(class_b_1)
    message_b = Message(parent=bench_b, clazz=class_b_1)
    assert message_b.bench_id == bench_b.id
    assert message_b.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_b.id,
            ck=message_b.ck,
            bench_id=bench_b.id,
            base_ck=class_b_1.ck,
            base_bench_id=bench_b.id,
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
    choice = Choice.new("Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    runtime.page().append(choice)
    await runtime.commit()


@simulated_runtime()
async def test_clone_subtree(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Clone a Node subtree."""
    choice = Choice.new("Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    runtime.page().append(choice)
    await runtime.commit()

    choice_clone = choice.clone()
    assert choice_clone.equals(choice)
    for field, field_clone in zip(choice.fields, choice_clone.fields):
        assert field_clone.equals(field)
    await runtime.commit()


@simulated_runtime()
async def test_clone_consistency(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Clone consistency test with references."""
    choice = Choice.new("Letter", fields=[Field.option("A"), Field.option("B")])
    flow = Flow.new("Flow")
    action = Action.new(
        ActionType.CODE,
        "Action",
        fields=[Field.input("Text", Text), Field.output("Choice", choice)],
    )
    flow.append(action)
    page1 = runtime.page()
    page1.append(choice)
    page1.append(flow)
    await runtime.commit()

    # references should be consistent within new subtree
    page_clone = page1.clone()
    flow_clone = page_clone.blocks.Flow.node_as(Flow)
    assert flow_clone is not None
    choice_clone = page_clone.blocks.Letter.node_as(Choice)
    assert choice_clone is not None
    action_clone = flow_clone.actions.Action
    assert action_clone is not None
    assert action_clone.fields.Choice.base_type == choice_clone
    await runtime.commit()


@simulated_runtime()
async def test_move_subtree(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Move Nodes between parents (within a Package)."""
    Page1 = runtime.page("Page1")
    Page2 = runtime.page("Page2")
    Block1 = Page1.append(Choice.new("Block1", fields=[Field.option("A"), Field.option("B")]))
    Block2 = Page1.append(
        Flow.new("Block2", Field.input("Text", Text), Field.output("Choice", Block1))
    )
    Block3 = Page1.append(Database.new("Block3", fields=[Field.member("Text", Text)]))
    await runtime.commit()

    # can't just append directly
    with pytest.raises(ValueError):
        Page2.append(Block3)

    # move Block1 to Page2
    Block1.move(to=Page2)
    await runtime.commit()
    assert Page1.blocks == [Block2, Block3]
    assert Page2.blocks == [Block1]

    # move Block1 back to Page1
    Block1.move(to=Page1)
    await runtime.commit()
    assert Page1.blocks == [Block2, Block3, Block1]
    assert Page2.blocks == []

    # move all blocks to Page2
    Block1.move(to=Page2)
    Block2.move(to=Page2)
    Block3.move(to=Page2)
    await runtime.commit()
    assert Page1.blocks == []
    assert Page2.blocks == [Block1, Block2, Block3]


def test_create_circular_node_ancestry(session: Session, package: Package):
    """Create a circular node ancestry. Should fail."""
    Page1 = Page.new("Page1")
    with pytest.raises(ValueError):
        Page1.append(Page1)
    package.append(Page1)

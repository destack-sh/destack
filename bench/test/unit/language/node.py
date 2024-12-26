from itertools import chain
from typing import cast

import pytest
from hypothesis import HealthCheck, given, settings

from bench.language import Bench, Message, NodeReference, Property, Server
from bench.language.action import Action, ActionType, DuplicateAction
from bench.language.bench import Client, PackageType
from bench.language.block import Block, ValueBlock
from bench.language.code import Code
from bench.language.const import BlockType, ClientType, NodeType
from bench.language.field import Field, to_type
from bench.language.node import BuiltinObject
from bench.language.registry import NODE_CLASSES, STRUCT_CLASSES
from bench.language.session import Session
from bench.language.text import Text, md
from bench.proto.wiring import unpack_builtin_object
from bench.test.strategies import structs
from bench.test.unit.conftest import RuntimeHandle


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
        _ = Block.new(BlockType.TEXT, name="Test", _non_existing_property="wadabadaboo")  # type: ignore


def test_get_set_non_existing_property(session: "Session"):
    """Should raise properly"""
    node = Bench(slug="test", name="Test")
    with pytest.raises(AttributeError):
        node.wadabadaboo = "wadabadaboo"  # type: ignore
    with pytest.raises(AttributeError):
        _ = node.wadabadaboo  # type: ignore


def test_node_passthrough(session: "Session"):
    bench = Bench(slug="test", name="Test")
    package = bench.packages.create(type=PackageType.ROOT, name="Main", slug="main")

    WeatherCondition = package.blocks.create(
        type=BlockType.CHOICE,
        name="WeatherCondition",
        fields=[
            Field.option("Sunny"),
            Field.option("Rainy"),
            Field.option("Cloudy"),
            Field.option("Snowy"),
        ],
    )
    assert WeatherCondition.fields.Sunny is WeatherCondition.fields.get("Sunny")
    assert WeatherCondition.Sunny is WeatherCondition.fields.get("Sunny")  # type: ignore


def test_node_subtype_property_access(session: "Session"):
    # subtype -> regular property
    Text1 = Block.new(BlockType.TEXT, "Text1", text=md("Hello!"))
    assert Text1.text is not None and Text1.text.to_markdown() == "Hello!"
    Text1.text = md("Hello, world!")
    assert Text1.text is not None and Text1.text.to_markdown() == "Hello, world!"

    # subtype -> node ref property
    BlockAction1 = Action.new(DuplicateAction, "BlockAction1", node=Text1)
    assert BlockAction1.node == Text1
    assert BlockAction1.node_ptr == Text1.to_ref()

    # subtype -> value packed property
    Value1 = Block.new(ValueBlock, "Value1", value_type=to_type(int), value=42)
    assert Value1.value_type == to_type(int)
    assert Value1.value == 42
    Value1.value = 43
    assert Value1.value == 43
    assert Value1.value_type
    assert Value1.value_packed
    assert Value1.value_packed[Value1.value_type.identity_key] == 43


def test_node_subtype_pack_unpack(session: "Session"):
    block = Block.new(BlockType.TEXT, "Text1", text=md("Hello!"))
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
    bench_a.main_server = server_a = bench_a.servers.create(name="Server")
    bench_a.main_store = bench_a.stores.create(name="Store")
    bench_a.main_drive = bench_a.drives.create(name="Drive")

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
    server_a: Server = Server(parent=bench_a, name="Main")
    assert server_a.bench_id == bench_a.id
    client_a = Client(
        parent=bench_a,
        seen_at=session._oracle.utc(),
        type=ClientType.BENCH_MOBILE,
        title="Testificate's iPhone",
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
    block_a_1 = package_a.blocks.create(type=BlockType.VIEW)
    assert block_a_1.bench_id == bench_a.id
    assert block_a_1.to_ref().equals(
        NodeReference(
            node_type=NodeType.BLOCK, id=block_a_1.id, ck=block_a_1.ck, bench_id=bench_a.id
        )
    )
    block_a_2 = package_a.blocks.create(type=BlockType.ROLE)
    block_a_1.roles = [block_a_2]

    # based pointers
    message_a = Message(parent=bench_a, origin=block_a_1, block=block_a_1)
    assert message_a.bench_id == bench_a.id
    assert message_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_a.id,
            ck=message_a.ck,
            bench_id=bench_a.id,
            base_ck=block_a_1.ck,
            base_bench_id=bench_a.id,
        )
    )

    # refs pointing to different bench
    bench_b = Bench(slug="testb", name="testb")
    bench_b.main_server = bench_b.servers.create(name="Server")
    bench_b.main_store = bench_b.stores.create(name="Store")
    bench_b.main_drive = bench_b.drives.create(name="Drive")
    package_b = bench_b.packages.create(type=PackageType.ROOT, name="Main B", slug="main-b")
    block_b = package_b.blocks.create(type=BlockType.PAGE, roles=[block_a_1])
    assert block_b.bench_id == bench_b.id
    assert block_b.roles
    assert block_b.roles[0].bench_id == bench_a.id
    message_b = Message(parent=bench_b, origin=block_a_1, block=block_a_1)
    assert message_b.bench_id == bench_b.id
    assert message_b.to_ref().equals(
        NodeReference(
            node_type=NodeType.MESSAGE,
            id=message_b.id,
            ck=message_b.ck,
            bench_id=bench_b.id,
            base_ck=block_a_1.ck,
            base_bench_id=bench_a.id,
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


async def test_add_detached_subtree(hosted_runtime: RuntimeHandle):
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    hosted_runtime.page().blocks.append(choice)
    await hosted_runtime.commit()


async def test_clone_subtree(hosted_runtime: RuntimeHandle):
    """Clone a Node subtree."""
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    hosted_runtime.page().blocks.append(choice)
    await hosted_runtime.commit()

    choice_clone = choice.clone()
    assert choice_clone.equals(choice)
    for field, field_clone in zip(choice.fields, choice_clone.fields):
        assert field_clone.equals(field)
    await hosted_runtime.commit()


async def test_clone_consistency(hosted_runtime: RuntimeHandle):
    """Clone consistency test with references."""
    choice = Block.new(BlockType.CHOICE, "Letter", fields=[Field.option("A"), Field.option("B")])
    action = Action.new(
        ActionType.CODE,
        "Action",
        fields=[Field.input("Text", Text), Field.output("Choice", choice)],
    )
    hosted_runtime.page().append(choice)
    hosted_runtime.page().append(action)
    await hosted_runtime.commit()

    # references should be consistent within new subtree
    page_clone = hosted_runtime.page().clone()
    choice_clone = page_clone.blocks.get("Letter")
    assert choice_clone is not None
    action_clone = page_clone.blocks.get("Action")
    assert action_clone is not None
    assert action_clone.fields.Choice.base_type == choice_clone
    await hosted_runtime.commit()


async def test_move_subtree(hosted_runtime: RuntimeHandle):
    """Move Nodes between parents (within a Package)."""
    Page1 = hosted_runtime.page("Page1")
    Page2 = hosted_runtime.page("Page2")
    Block1 = Page1.blocks.append(
        Block.new(BlockType.CHOICE, "Block1", fields=[Field.option("A"), Field.option("B")])
    )
    Block2 = Page1.blocks.append(
        Block.new(
            BlockType.FLOW,
            "Block2",
            fields=[Field.input("Text", Text), Field.output("Choice", Block1)],
        )
    )
    Block3 = Page1.blocks.append(
        Block.new(BlockType.DATABASE, "Block3", fields=[Field.member("Text", Text)])
    )
    await hosted_runtime.commit()

    # can't just append directly
    with pytest.raises(ValueError):
        Page2.append(Block3)

    # move Block1 to Page2
    Block1.move(to=Page2)
    await hosted_runtime.commit()
    assert Page1.blocks == [Block2, Block3]
    assert Page2.blocks == [Block1]

    # move Block1 back to Page1
    Block1.move(to=Page1)
    await hosted_runtime.commit()
    assert Page1.blocks == [Block2, Block3, Block1]
    assert Page2.blocks == []

    # move all blocks to Page2
    Block1.move(to=Page2)
    Block2.move(to=Page2)
    Block3.move(to=Page2)
    await hosted_runtime.commit()
    assert Page1.blocks == []
    assert Page2.blocks == [Block1, Block2, Block3]


@pytest.mark.skip("NOTE :Robustness: check circular node ancestry")
async def test_create_circular_node_ancestry(hosted_runtime: RuntimeHandle):
    """Create a circular node ancestry. Should fail."""
    # ...

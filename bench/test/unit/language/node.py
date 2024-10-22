from itertools import chain
from typing import cast

import pytest
from hypothesis import given

from bench.language import Bench, NodeReference, Property, Server, Signal
from bench.language.bench import Client, Package
from bench.language.block import Block, ViewBlock
from bench.language.code import Code
from bench.language.const import BlockType, ClientType, NodeType
from bench.language.field import Field
from bench.language.file import File, FileKind, FileReference, FileType
from bench.language.node import BuiltinObject
from bench.language.session import Session
from bench.language.setup import NODE_CLASSES, STRUCT_CLASSES
from bench.language.view import View, ViewType
from bench.proto.wiring import unpack_object
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
    branch = bench.branches.create(name="main")
    package = branch.packages.create()

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
    branch_a = bench_a.branches.create(name="main a")
    assert branch_a.bench_id == bench_a.id
    assert branch_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.BRANCH, id=branch_a.id, ck=branch_a.ck, bench_id=bench_a.id
        )
    )
    assert branch_a.parent_ptr
    assert branch_a.parent_ptr.id == bench_a.id

    # sub bench nested pointers
    server_a: Server = Server(parent=bench_a, name="Main")
    assert server_a.bench_id == bench_a.id
    client_a = Client(
        parent=server_a,
        seen_at=session._oracle.utc(),
        type=ClientType.BENCH_MOBILE,
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
    branch_a = bench_a.branches.create(name="main a")
    package_a = branch_a.packages.create()
    assert package_a.bench_id == bench_a.id
    block_a_1 = cast(ViewBlock, package_a.blocks.create(type=BlockType.VIEW))
    assert block_a_1.bench_id == bench_a.id
    assert block_a_1.to_ref().equals(
        NodeReference(
            node_type=NodeType.BLOCK, id=block_a_1.id, ck=block_a_1.ck, bench_id=bench_a.id
        )
    )
    block_a_2 = package_a.blocks.create(type=BlockType.ROLE)
    block_a_1.roles = [block_a_2]

    # based pointers
    signal_a = Signal(parent=package_a, block=block_a_1)
    assert signal_a.bench_id == bench_a.id
    assert signal_a.to_ref().equals(
        NodeReference(
            node_type=NodeType.SIGNAL,
            id=signal_a.id,
            ck=signal_a.ck,
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
    branch_b = bench_b.branches.create(name="main b")
    package_b = branch_b.packages.create()
    block_b = package_b.blocks.create(type=BlockType.CODE, roles=[block_a_1])
    assert block_b.bench_id == bench_b.id
    assert block_b.roles
    assert block_b.roles[0].bench_id == bench_a.id
    signal_b = Signal(parent=package_b, block=block_a_1)
    assert signal_b.bench_id == bench_b.id
    assert signal_b.to_ref().equals(
        NodeReference(
            node_type=NodeType.SIGNAL,
            id=signal_b.id,
            ck=signal_b.ck,
            bench_id=bench_b.id,
            base_ck=block_a_1.ck,
            base_bench_id=bench_a.id,
        )
    )
    assert signal_b.parent_ptr
    assert signal_b.parent_ptr.bench_id == bench_b.id

    # rich references
    file1 = File(
        kind=FileKind.DRIVE,
        title="File1",
        size=0,
        type=FileType.TEXT,
        mime_type="text/plain",
    )
    file1_ref = file1.to_ref()
    assert isinstance(file1_ref, FileReference)
    view_a_1_1 = block_a_1.views.append(View.new(ViewType.COLOR, "Color1"))
    view_a_1_1.node = file1
    assert isinstance(view_a_1_1.node_ptr, FileReference)
    assert view_a_1_1.node == file1


@given(obj=structs)
def test_builtin_object_clone(obj: BuiltinObject, shared_session: Session):
    obj_clone = obj.clone()
    assert obj_clone.equals(obj)


async def test_add_detached_subtree(local_runtime: RuntimeHandle):
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    local_runtime.page().blocks.append(choice)
    await local_runtime.commit()


async def test_clone_subtree(local_runtime: RuntimeHandle):
    choice = Block.new(BlockType.CHOICE, "Letter")
    for i in range(0, 26):
        letter = chr(65 + i)
        choice.fields.append(Field.option(letter))
    local_runtime.page().blocks.append(choice)
    await local_runtime.commit()

    choice_clone = choice.clone()
    assert choice_clone.equals(choice)
    for field, field_clone in zip(choice.fields, choice_clone.fields):
        assert field_clone.equals(field)
    await local_runtime.commit()


def test_roundtrip_rich_reference(shared_session: Session, shared_package: Package):
    # plain reference
    block = Block.new_text("Text1", "Hello, world!")
    view = View.new(ViewType.PAGE, "Page1", node=block)
    view_data = view._to_data()
    unpacked_view = unpack_object(
        view_data, supergraph=shared_session._supergraph, session=shared_session
    )
    assert unpacked_view.equals(view)

    # file reference (rich)
    file = File(
        parent=shared_package.bench.main_drive,
        kind=FileKind.DRIVE,
        title="myfile.txt",
        type=FileType.TEXT,
        mime_type="text/plain",
        size=1024,
    )
    view.node = file
    view_data = view._to_data()
    unpacked_view = unpack_object(
        view_data, supergraph=shared_session._supergraph, session=shared_session
    )
    assert unpacked_view.equals(view)


@pytest.mark.skip("NOTE :Robustness: check circular node ancestry")
async def test_create_circular_node_ancestry(hosted_runtime: RuntimeHandle):
    """Create a circular node ancestry. Should fail."""
    # page->block1->block2
    block1 = Block.new(BlockType.CODE, "Code1")
    hosted_runtime.page().blocks.append(block1)
    block2 = Block.new(BlockType.CODE, "Code2")
    block1.blocks.append(block2)
    await hosted_runtime.session.commit()

    # now force block2->block1
    # ...

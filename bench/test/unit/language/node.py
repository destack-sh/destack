from itertools import chain

import pytest
from hypothesis import given

from bench.language import Bench, NodeReference, Property, Server, Signal
from bench.language.bench import Client, ServerProfile
from bench.language.const import BlockType, ClientType, NodeType
from bench.language.field import Field
from bench.language.file import File
from bench.language.node import BuiltinObject
from bench.language.session import Session
from bench.language.setup import NODE_CLASSES, STRUCT_CLASSES
from bench.language.view import View, ViewType
from bench.test.strategies import structs


def test_struct_regular_properties_are_available():
    """Slightly tautological sanity check to check our introspected properties"""
    for cls in chain(STRUCT_CLASSES, NODE_CLASSES):
        for prop in cls.__properties__.values():
            if prop.is_introspectable:
                attr = getattr(cls, prop.name)
                assert isinstance(attr, Property), f"{prop!r}->{attr!r} is not a Property"


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
    assert bench_a.to_ref()._equals_content(
        NodeReference(type=NodeType.BENCH, id=bench_a.id, ck=bench_a.ck, bench_id=bench_a.id)
    )
    bench_a.main_server = server_a = bench_a.servers.create(
        name="Server", profile=ServerProfile.TINY
    )
    bench_a.main_store = bench_a.stores.create(name="Store")
    bench_a.main_drive = bench_a.drives.create(name="Drive")

    # sub bench, above package pointers
    branch_a = bench_a.branches.create(name="main a")
    assert branch_a.bench_id == bench_a.id
    assert branch_a.to_ref()._equals_content(
        NodeReference(type=NodeType.BRANCH, id=branch_a.id, ck=branch_a.ck, bench_id=bench_a.id)
    )
    assert branch_a.parent_ptr
    assert branch_a.parent_ptr.id == bench_a.id

    # sub bench nested pointers
    server_a: Server = Server(parent=bench_a, name="Main", profile=ServerProfile.TINY)
    assert server_a.bench_id == bench_a.id
    client_a = Client(
        parent=server_a,
        seen_at=session._oracle.utc(),
        type=ClientType.BENCH_MOBILE,
        name="Testificate's iPhone",
    )
    assert client_a.bench_id == bench_a.id
    assert client_a.to_ref()._equals_content(
        NodeReference(type=NodeType.CLIENT, id=client_a.id, ck=client_a.ck, bench_id=bench_a.id)
    )
    assert client_a.parent_ptr
    assert client_a.parent_ptr.bench_id == bench_a.id

    # sub package nested pointers
    branch_a = bench_a.branches.create(name="main a")
    package_a = branch_a.packages.create()
    assert package_a.bench_id == bench_a.id
    block_a_1 = package_a.blocks.create(type=BlockType.CODE)
    assert block_a_1.bench_id == bench_a.id
    assert block_a_1.to_ref()._equals_content(
        NodeReference(type=NodeType.BLOCK, id=block_a_1.id, ck=block_a_1.ck, bench_id=bench_a.id)
    )
    block_a_2 = package_a.blocks.create(type=BlockType.CODE)
    block_a_1.bases = [block_a_2]

    # based pointers
    signal_a = Signal(parent=package_a, type=block_a_1)
    assert signal_a.bench_id == bench_a.id
    assert signal_a.to_ref()._equals_content(
        NodeReference(
            type=NodeType.SIGNAL,
            id=signal_a.id,
            ck=signal_a.ck,
            bench_id=bench_a.id,
            base_ck=block_a_1.ck,
            base_bench_id=bench_a.id,
        )
    )

    # refs pointing to different bench
    bench_b = Bench(slug="testb", name="testb")
    bench_b.main_server = bench_b.servers.create(name="Server", profile=ServerProfile.TINY)
    bench_b.main_store = bench_b.stores.create(name="Store")
    bench_b.main_drive = bench_b.drives.create(name="Drive")
    branch_b = bench_b.branches.create(name="main b")
    package_b = branch_b.packages.create()
    block_b = package_b.blocks.create(type=BlockType.CODE, bases=[block_a_1])
    assert block_b.bench_id == bench_b.id
    assert block_b.bases
    assert block_b.bases[0].bench_id == bench_a.id
    signal_b = Signal(parent=package_b, type=block_a_1)
    assert signal_b.bench_id == bench_b.id
    assert signal_b.to_ref()._equals_content(
        NodeReference(
            type=NodeType.SIGNAL,
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
    file1 = File(title="File1", size=0, mime_type="text/plain")
    view_a_1_1 = block_a_1.views.append(View.new(ViewType.COLOR, "Color1"))
    view_a_1_1.node = file1
    assert view_a_1_1.node == file1


@given(obj=structs)
def test_struct_clone(obj: BuiltinObject, shared_session):
    obj_clone = obj.clone()
    assert obj_clone._equals_content(obj)

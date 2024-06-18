from itertools import chain

import pytest

from bench.language import Bench, Environment, NodeReference, Property, Server, Signal
from bench.language.bench import Client, ServerProfile
from bench.language.const import BlockType, ClientType, NodeType
from bench.language.session import Session
from bench.language.setup import NODE_CLASSES, STRUCT_CLASSES
from bench.utils.oracle import get_oracle


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


def test_node_pointers_consistency(session: "Session"):
    """Pointers should include the relevant bench/base/base_bench references."""
    bench_a = Bench(slug="test_a", name="test_b")
    assert bench_a.to_ref()._equals_content(
        NodeReference(type=NodeType.BENCH, id=bench_a.id, ck=bench_a.ck, bench_id=bench_a.id)
    )
    server_a = bench_a.servers.create(name="Server", profile=ServerProfile.TINY)
    store_a = bench_a.stores.create(name="Store")
    drive_a = bench_a.drives.create(name="Drive")

    # sub bench, above package pointers
    environment_a = bench_a.environments.create(
        name="Production A", server=server_a, store=store_a, drive=drive_a
    )
    assert environment_a.bench_id == bench_a.id
    assert environment_a.to_ref()._equals_content(
        NodeReference(
            type=NodeType.ENVIRONMENT, id=environment_a.id, ck=environment_a.ck, bench_id=bench_a.id
        )
    )
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
        seen_at=get_oracle().utc(),
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
    package_a = branch_a.packages.create(environment=environment_a)
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
    bench_b = Bench(slug="test_b", name="test_b")
    server_b = bench_b.servers.create(name="Server", profile=ServerProfile.TINY)
    store_b = bench_b.stores.create(name="Store")
    drive_b = bench_b.drives.create(name="Drive")
    environment_b = Environment(
        parent=bench_b, name="Production B", server=server_b, store=store_b, drive=drive_b
    )
    branch_b = bench_b.branches.create(name="main b")
    package_b = branch_b.packages.create(environment=environment_b)
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

from itertools import chain

import pytest

from bench.language import Bench, Environment, NodeReference, Property, Server, Signal
from bench.language.const import BlockType, InterpStatus, NodeType
from bench.language.setup import NODE_CLASSES, STRUCT_CLASSES
from bench.language.test.fabricator import Fabricator
from bench.language.user import Client


def test_struct_regular_properties_are_available():
    """Slightly tautological sanity check to check our introspected properties"""
    for cls in chain(STRUCT_CLASSES, NODE_CLASSES):
        for prop in cls.__properties__.values():
            if prop.is_introspectable:
                attr = getattr(cls, prop.name)
                assert isinstance(attr, Property), f"{prop!r}->{attr!r} is not a Property"


def test_get_set_non_existing_property(fabricator: "Fabricator"):
    """Should raise properly"""
    client = fabricator.fabricate(Client)
    client._status = InterpStatus.TRACKED
    with pytest.raises(AttributeError):
        client.wadabadaboo = "wadabadaboo"  # type: ignore
    with pytest.raises(AttributeError):
        _ = client.wadabadaboo  # type: ignore


async def test_node_pointers_consistency(fabricator: "Fabricator"):
    """Pointers should include the relevant bench/base/base_bench references."""
    bench_a = fabricator.fabricate(Bench, slug="test_a", name="test_b")
    assert bench_a.to_ref().equals_content(
        NodeReference(type=NodeType.BENCH, id=bench_a.id, bench_id=bench_a.id)
    )

    # sub bench, above package pointers
    environment_a = Environment(parent=bench_a, name="Production A")
    assert environment_a.bench_id == bench_a.id
    assert environment_a.to_ref().equals_content(
        NodeReference(type=NodeType.ENVIRONMENT, id=environment_a.id, bench_id=bench_a.id)
    )
    branch_a = bench_a.branches.create(name="main a")
    assert branch_a.bench_id == bench_a.id
    assert branch_a.to_ref().equals_content(
        NodeReference(type=NodeType.BRANCH, id=branch_a.id, bench_id=bench_a.id)
    )
    assert branch_a.parent_ptr
    assert branch_a.parent_ptr.id == bench_a.id

    # sub bench nested pointers
    server_a: Server = fabricator.fabricate(Server, parent=bench_a, name="Main")
    assert server_a.bench_id == bench_a.id
    client_a = fabricator.fabricate(Client, parent=server_a, name="Testificate's iPhone")
    assert client_a.bench_id == bench_a.id
    assert client_a.to_ref().equals_content(
        NodeReference(type=NodeType.CLIENT, id=client_a.id, bench_id=bench_a.id)
    )
    assert client_a.parent_ptr
    assert client_a.parent_ptr.bench_id == bench_a.id

    # sub package nested pointers
    package_a = bench_a.packages.create(environment=environment_a)
    assert package_a.bench_id == bench_a.id
    block_a_1 = package_a.blocks.create(type=BlockType.CODE)
    assert block_a_1.bench_id == bench_a.id
    assert block_a_1.to_ref().equals_content(
        NodeReference(type=NodeType.BLOCK, ck=block_a_1.ck, id=block_a_1.id, bench_id=bench_a.id)
    )
    block_a_2 = package_a.blocks.create(type=BlockType.CODE)
    block_a_1.bases = [block_a_2]

    # based pointers
    signal_a = Signal(parent=package_a, type=block_a_1)
    assert signal_a.bench_id == bench_a.id
    assert signal_a.to_ref().equals_content(
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
    bench_b = fabricator.fabricate(Bench, slug="test_b", name="test_b")
    environment_b = Environment(parent=bench_b, name="Production B")
    package_b = bench_b.packages.create(environment=environment_b)
    block_b = package_b.blocks.create(type=BlockType.CODE, bases=[block_a_1])
    assert block_b.bench_id == bench_b.id
    assert block_b.bases
    assert block_b.bases[0].bench_id == bench_a.id
    signal_b = Signal(parent=package_b, type=block_a_1)
    assert signal_b.bench_id == bench_b.id
    assert signal_b.to_ref().equals_content(
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

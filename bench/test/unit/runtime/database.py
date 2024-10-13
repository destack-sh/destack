import pytest
from grpclib import GRPCError, Status

from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.session import Session
from bench.test.unit.conftest import RuntimeHandle


def test_create_record_kwargs(session: Session):
    """Create a Record with keyword arguments into value."""
    Database1 = Block.new(
        BlockType.DATABASE,
        "Database1",
        fields=[
            Field.member("Name", str),
            Field.member("Age", int),
            Field.member("Aliases", str, is_list=True),
        ],
    )
    Record1 = Database1.records.create()
    assert Record1.Name is None  # type: ignore
    assert Record1.Age is None  # type: ignore
    assert Record1.Aliases is None  # type: ignore

    Record2 = Database1.records.create(Name="Record2", Aliases=["R3", "R4"])
    assert Record2.Name == "Record2"  # type: ignore
    assert Record2.Age is None  # type: ignore
    assert Record2.Aliases == ["R3", "R4"]  # type: ignore
    with pytest.raises(AttributeError):
        _ = Record2.NonExistent  # type: ignore


async def test_create_empty_database_block(hosted_runtime: RuntimeHandle):
    """Create a blank database and query it."""
    Database1 = Block.new(BlockType.DATABASE, "Database1")
    hosted_runtime.page().blocks.append(Database1)

    # cannot access database before committing it
    with pytest.raises(GRPCError) as e:  # :BadRemoteErrors
        _ = await Database1.records.search()
        assert e.value.status == Status.FAILED_PRECONDITION

    # commit to create database
    await hosted_runtime.session.commit()

    # should be empty
    records = await Database1.records.search()
    assert records == []


async def test_create_database_and_records_simultaneously(hosted_runtime: RuntimeHandle):
    """Create a database and records within it at the same time."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database1.records.create(Name="Record2")
    await hosted_runtime.session.commit()

    records = await Database1.records.search()
    assert records == [Record1, Record2]

from datetime import date, datetime, time, timedelta
from uuid import UUID

import pytest
from grpclib import GRPCError, Status

from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.text import Text, md
from bench.language.value import sample_value
from bench.test.unit.conftest import RuntimeHandle
from bench.utils.string import Casing, to_casing


def test_create_record_kwargs(local_runtime: RuntimeHandle):
    """Create a Record with keyword arguments (into value)."""
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
    assert Record2.value_packed is not None
    assert Record2.value_packed[Database1.fields.Name.storage_key] == "Record2"


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
    """Create a database and records within it in the same transaction/commit."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database1.records.create(Name="Record2")
    await hosted_runtime.session.commit()

    records = await Database1.records.search()
    assert records == [Record1, Record2]
    assert records[0].Name == "Record1"  # type: ignore
    assert records[1].Name == "Record2"  # type: ignore


async def test_update_record(hosted_runtime: RuntimeHandle):
    """Update a record with a simple Field and query it."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    hosted_runtime.page().blocks.append(Database1)

    # create & query
    Record1 = Database1.records.create(Name="Record1")
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records[0].Name == "Record1"  # type: ignore

    # update & query
    Record1.Name = "Record1.1"  # type: ignore
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records[0].Name == "Record1.1"  # type: ignore


async def test_update_database_and_record(hosted_runtime: RuntimeHandle):
    """Updates a database and records within and across transactions."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Id", int)])
    hosted_runtime.page().blocks.append(Database1)

    # create records with value for every field type
    cached_records = []
    for i, sample_type in enumerate(
        (
            str,
            int,
            float,
            datetime,
            date,
            timedelta,
            time,
            UUID,
        )
    ):
        for is_list in (False, True):
            field_name = f"{to_casing(sample_type.__name__, Casing.CAMEL)}{'s' if is_list else ''}"
            field = Database1.fields.append(Field.member(field_name, sample_type, is_list=is_list))
            sample_field_value = sample_value(field)
            record = Database1.records.create(Id=i, **{field_name: sample_field_value})
            cached_records.append(record)
            await hosted_runtime.session.commit()

    # query
    stored_records = await Database1.records.order_by(Database1.fields.Id).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)

    # update
    for field, record in zip(Database1.fields, stored_records):
        sample_field_value = sample_value(field)
        setattr(record, field.name, sample_field_value)
    await hosted_runtime.session.commit()

    # query again
    stored_records = await Database1.records.order_by(Database1.fields.Id).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)


async def test_create_record_with_ptrs(hosted_runtime: RuntimeHandle):
    """Create a Database with pointer fields (scalar and list)."""
    Database1 = Block.new(
        BlockType.DATABASE,
        "Database1",
        fields=[Field.member("Block", Block), Field.member("Blocks", Block, is_list=True)],
    )
    hosted_runtime.page().blocks.append(Database1)
    await hosted_runtime.session.commit()

    Record1 = Database1.records.create(Block=Database1, Blocks=[Database1])
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record1]


# nocheckin: default sort for databases remote list?


async def test_move_database(hosted_runtime: RuntimeHandle):
    """Move a database between parents while creating a Record."""
    # create database in Page1
    Page1 = hosted_runtime.page("Page1")
    Page2 = hosted_runtime.page("Page2")
    Database1 = Page1.blocks.append(Block.new(BlockType.DATABASE, "Database1"))
    Record1 = Database1.records.create(title="Record1")
    await hosted_runtime.session.commit()

    # query database in Page1
    records = await Database1.records.search()
    assert records == [Record1]

    # move database to Page2
    Database1.move(to=Page2)
    Record2 = Database1.records.create(title="Record2")
    await hosted_runtime.session.commit()

    # query database in Page2
    records = await Database1.records.search()
    assert records == [Record1, Record2]


async def test_delete_restore_database(hosted_runtime: RuntimeHandle):
    """Delete a database, querying it shouldn't work. Restore, and it should work again."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    Record1 = Database1.records.create(Name="Record1")
    hosted_runtime.page().blocks.append(Database1)
    await hosted_runtime.session.commit()

    # delete
    Database1.delete()
    await hosted_runtime.session.commit()
    with pytest.raises(GRPCError) as e:  # :BadRemoteErrors
        _ = await Database1.records.search()
        assert e.value.status == Status.NOT_FOUND

    # restore
    Database1.restore()
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record1]


async def test_delete_restore_record(hosted_runtime: RuntimeHandle):
    """Deleting a Record should remove it from default view, restoring should re-add it."""
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database1.records.create(Name="Record2")
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record1, Record2]

    # delete
    Record1.delete()
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record2]

    # insert
    Record3 = Database1.records.create(Name="Record3")
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record3, Record2]

    # delete
    Record2.delete()
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record3]

    # restore
    Record1.restore()
    await hosted_runtime.session.commit()
    records = await Database1.records.search()
    assert records == [Record1, Record3]


async def test_delete_restore_database_field(hosted_runtime: RuntimeHandle):
    """Delete and restore a Field in a Database."""
    Field1 = Field.member("Field1", str)
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field1])
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Field1="Record1")
    await hosted_runtime.session.commit()

    # delete
    Field1.delete()
    await hosted_runtime.session.commit()
    with pytest.raises(AttributeError):  # can't update anymore (Field1 is gone)
        Record1.Field1 = "Value1"  # type: ignore

    # restore
    Field1.restore()
    await hosted_runtime.session.commit()
    Record1.Field1 = "Value1"  # type: ignore
    await hosted_runtime.session.commit()


async def test_morph_database_field_type(hosted_runtime: RuntimeHandle):
    """
    Update a Fields type and set/get its values.
    The values belonging to different types should be preserved (should map to different columns).
    """
    # initial str is_list=False
    Field1 = Field.member("Field1", str, is_list=False)
    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field1])
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Field1="Record1")
    await hosted_runtime.session.commit()

    # morph str is_list=False -> str is_list=True
    Field1.is_list = True
    Record1.Field1 = ["Record1.1", "Record1.2"]  # type: ignore
    await hosted_runtime.session.commit()

    # morph str is_list=True -> int is_list=False
    Field1.morph_to(int, is_list=False)
    Record1.Field1 = 42  # type: ignore
    await hosted_runtime.session.commit()

    # morph back to str is_list=True (should still have old value)
    Field1.morph_to(str, is_list=True)
    await hosted_runtime.session.commit()
    Record1 = await Database1.records.get(Record1.to_ref())
    assert Record1.Field1 == ["Record1.1", "Record1.2"]  # type: ignore

    # morph back to str is_list=False (should still have old value)
    Field1.morph_to(str, is_list=False)
    await hosted_runtime.session.commit()
    Record1 = await Database1.records.get(Record1.to_ref())
    assert Record1.Field1 == "Record1"  # type: ignore


async def test_record_recursive_reference(hosted_runtime: RuntimeHandle):
    """Create a Record with a recursive reference to itself."""
    Database1 = Block.new(BlockType.DATABASE, "Database1")
    Database1.fields.append(Field.member("Record", Database1))
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(title="Record1")
    Record1.Record = Record1  # type: ignore
    await hosted_runtime.session.commit()
    assert Record1.Record == Record1  # type: ignore


async def test_search_record(hosted_runtime: RuntimeHandle):
    """Insert, update and query Records with various filters."""
    Database1 = Block.new(
        BlockType.DATABASE,
        "Database1",
        fields=[
            Field.member("Name", str),
            Field.member("Age", int),
            Field.member("Description", Text),
        ],
    )
    hosted_runtime.page().blocks.append(Database1)
    Record1 = Database1.records.create(Name="Alice", Age=30, Description=md("Alice is a *person*."))
    Record2 = Database1.records.create(Name="Bob", Age=40, Description=md("Bob is a *goat*."))
    Record3 = Database1.records.create(
        Name="Charlie", Age=50, Description=md("Charlie is a *cat*.")
    )
    await hosted_runtime.session.commit()

    # get by ref
    Result = await Database1.records.get(Record1.to_ref())
    assert Result == Record1

    # get by id
    Result = await Database1.records.get(id=Record2.id)
    assert Result == Record2

    # get by custom column
    Result = await Database1.records.get(Database1.fields.Age == 40)  # type: ignore
    assert Result == Record2

    # get by custom column
    Result = await Database1.records.get(Name="Charlie")
    assert Result == Record3

    # filter by custom column
    Result = await Database1.records.search(Database1.fields.Age >= 40)
    assert Result == [Record2, Record3]

    # order by custom column
    Result = await Database1.records.order_by(Database1.fields.Age).search()
    assert Result == [Record1, Record2, Record3]


async def test_database_isolation(hosted_runtime: RuntimeHandle):
    """Create two databases and ensure they don't interfere with each other."""

    Database1 = Block.new(BlockType.DATABASE, "Database1", fields=[Field.member("Name", str)])
    Database2 = Block.new(BlockType.DATABASE, "Database2", fields=[Field.member("Name", str)])
    hosted_runtime.page().blocks.extend(Database1, Database2)

    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database2.records.create(Name="Record2")
    await hosted_runtime.session.commit()

    records1 = await Database1.records.search()
    assert records1 == [Record1]
    records2 = await Database2.records.search()
    assert records2 == [Record2]

    Record3 = Database1.records.create(Name="Record3")
    Record4 = Database2.records.create(Name="Record4")
    await hosted_runtime.session.commit()

    records1 = await Database1.records.search()
    assert records1 == [Record3, Record1]
    records2 = await Database2.records.search()
    assert records2 == [Record4, Record2]

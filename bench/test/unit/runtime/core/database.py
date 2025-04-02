from datetime import date, datetime, time, timedelta
from random import Random
from typing import Any
from uuid import UUID

import pytest
import pytz
from grpclib import GRPCError

from bench.language import (
    Block,
    Database,
    Field,
    FileType,
    IsType,
    PrimitiveType,
    Text,
    TypeKind,
    text,
)
from bench.language.core.query import NodeNotFoundError
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.utils.string import Casing, to_casing


class SampleGenerator:
    def __init__(self, random: Random):
        self.random = random

    def generate(self, typ: IsType) -> Any:
        if not typ.is_list:
            return self.generate_scalar(typ)
        else:
            return [self.generate_scalar(typ) for _ in range(self.random.randint(1, 3))]

    def generate_scalar(self, typ: IsType) -> Any:
        if typ.kind == TypeKind.PRIMITIVE:
            assert typ.primitive_type is not None, f"primitive type is None for {typ!r}"
            if typ.primitive_type == PrimitiveType.STRING:
                return self.random.choice(("a", "b", "c"))
            elif typ.primitive_type.is_int:
                return self.random.randint(0, 100)
            elif typ.primitive_type.is_float:
                return self.random.uniform(0, 100)
            elif typ.primitive_type == PrimitiveType.BOOLEAN:
                return self.random.choice([True, False])
            elif typ.primitive_type == PrimitiveType.BYTES:
                return self.random.randbytes(10)
            elif typ.primitive_type == PrimitiveType.UUID:
                return UUID(int=self.random.randint(0, 0xFFFFFFFFFFFFFFFF))
            elif typ.primitive_type == PrimitiveType.JSON:
                return {"a": "b", "c": "d"}
            elif typ.primitive_type == PrimitiveType.DATETIME:
                return datetime.now(pytz.utc)
            elif typ.primitive_type == PrimitiveType.DATE:
                return datetime.now(pytz.utc).date()
            elif typ.primitive_type == PrimitiveType.TIME:
                return datetime.now(pytz.utc).time()
            elif typ.primitive_type == PrimitiveType.DURATION:
                return timedelta(seconds=self.random.randint(0, 1000000))

        raise RuntimeError(f"unsupported type {typ.primitive_type!r}")


@simulated_runtime()
async def test_create_record_kwargs(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    """Create a Record with keyword arguments (into value)."""
    Database1 = Database.new(
        "Database1",
        Field.member("name", Field.get_property("name")),  # internal property
        Field.member("Name", str),
        Field.member("Age", int),
        Field.member("Aliases", str, is_list=True),
    )
    runtime.page().append(Database1)
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


@simulated_runtime()
async def test_create_empty_database_block(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a blank database and query it."""
    Database1 = Database.new("Database1")
    runtime.page().append(Database1)

    # cannot access database before committing it
    with simulation.raises(NodeNotFoundError, GRPCError):
        _ = await Database1.records.search()

    # commit to create database
    await runtime.commit()

    # should be empty
    records = await Database1.records.search()
    assert records == []


@simulated_runtime()
async def test_create_database_and_records_simultaneously(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Create a database and records within it in the same transaction/commit."""
    Database1 = Database.new("Database1", Field.member("Name", str))
    runtime.page().append(Database1)
    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database1.records.create(Name="Record2")
    await runtime.commit()

    records = await Database1.records.search()
    assert records == [Record1, Record2]
    assert records[0].Name == "Record1"  # type: ignore
    assert records[1].Name == "Record2"  # type: ignore


@simulated_runtime()
async def test_update_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Update a record with a simple Field and query it."""
    Database1 = Database.new(
        "Database1",
        Field.member("name", Field.get_property("name")),  # internal property
        Field.member("Name", str),
    )
    runtime.page().append(Database1)

    # create & query
    Record1 = Database1.records.create(Name="Record1")
    await runtime.commit()
    records = await Database1.records.search()
    assert records[0].Name == "Record1"  # type: ignore

    # update & query
    Record1.Name = "Record1.1"  # type: ignore
    await runtime.commit()
    records = await Database1.records.search()
    assert records[0].Name == "Record1.1"  # type: ignore


@simulated_runtime()
async def test_update_database_and_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Updates a database and records within and across transactions."""
    Database1 = Database.new("Database1", Field.member("Id", int))
    runtime.page().append(Database1)
    sampler = SampleGenerator(Random(0))

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
            sample_field_value = sampler.generate(field)
            record = Database1.records.create(Id=i, **{field_name: sample_field_value})
            cached_records.append(record)
            await runtime.commit()

    # query
    stored_records = await Database1.records.order_by(Database1.fields.Id).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)

    # update
    for field, record in zip(Database1.fields, stored_records):
        assert field.name is not None, f"field {field!r} has no name"
        sample_field_value = sampler.generate(field)
        setattr(record, field.name, sample_field_value)
    await runtime.commit()

    # query again
    stored_records = await Database1.records.order_by(Database1.fields.Id).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)


@simulated_runtime()
async def test_create_record_with_ptrs(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Database with pointer fields (scalar and list)."""
    Database1 = Database.new(
        "Database1",
        Field.member("Block", Block),
        Field.member("Blocks", Block, is_list=True),
    )
    runtime.page().append(Database1)
    await runtime.commit()

    Record1 = Database1.records.create(Block=Database1, Blocks=[Database1])
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record1]


@simulated_runtime()
async def test_move_database(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Move a database between parents while creating a Record."""
    # create database in Page1
    Page1 = runtime.page("Page1")
    Page2 = runtime.page("Page2")
    Database1 = Database.new(
        "Database1",
        Field.member("Alias", str),
        Field.member("Image", FileType.IMAGE),
    )
    Page1.append(Database1)
    Record1 = Database1.records.create(name="Record1", Alias="1")
    await runtime.commit()

    # query database in Page1
    records = await Database1.records.search()
    assert records == [Record1]

    # move database to Page2
    Database1.move(to=Page2)
    Record2 = Database1.records.create(name="Record2", Alias="2")
    await runtime.commit()

    # query database in Page2
    records = await Database1.records.search()
    assert records == [Record1, Record2]

    # delete Record1
    Record1.delete()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record2]


@simulated_runtime()
async def test_delete_restore_database(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Delete a database, querying it shouldn't work. Restore, and it should work again."""
    Database1 = Database.new("Database1", Field.member("Name", str))
    Record1 = Database1.records.create(Name="Record1")
    runtime.page().append(Database1)
    await runtime.commit()

    # delete
    Database1.delete()
    await runtime.commit()
    with simulation.raises(NodeNotFoundError, GRPCError):
        _ = await Database1.records.search()

    # restore
    Database1.restore()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record1]


@simulated_runtime()
async def test_delete_restore_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Deleting a Record should remove it from default view, restoring should re-add it."""
    Database1 = Database.new("Database1", Field.member("Name", str))
    runtime.page().append(Database1)
    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database1.records.create(Name="Record2")
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record1, Record2]

    # delete
    Record1.delete()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record2]

    # insert
    Record3 = Database1.records.create(Name="Record3")
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record2, Record3]

    # delete
    Record2.delete()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record3]

    # restore
    Record1.restore()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == [Record1, Record3]


@simulated_runtime()
async def test_delete_restore_database_field(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Delete and restore a Field in a Database."""
    Field1 = Field.member("Field1", str)
    Database1 = Database.new("Database1", Field1)
    runtime.page().append(Database1)
    Record1 = Database1.records.create(Field1="Record1")
    await runtime.commit()

    # delete
    Field1.delete()
    await runtime.commit()
    with pytest.raises(AttributeError):  # can't update anymore (Field1 is gone)
        Record1.Field1 = "Value1"  # type: ignore

    # restore
    Field1.restore()
    await runtime.commit()
    Record1.Field1 = "Value1"  # type: ignore
    await runtime.commit()


@simulated_runtime()
async def test_morph_database_field_type(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """
    Update a Fields type and set/get its values.
    The values belonging to different types should be preserved (should map to different columns).
    """
    # initial str is_list=False
    Field1 = Field.member("Field1", str, is_list=False)
    Field2 = Field.member("Field2", bool, is_list=False)
    Database1 = Database.new("Database1", Field1, Field2)
    runtime.page().append(Database1)
    Record1 = Database1.records.create(Field1="Record1", Field2=True)
    await runtime.commit()

    # morph str is_list=False -> str is_list=True
    Field1.is_list = True
    Record1.Field1 = ["Record1.1", "Record1.2"]  # type: ignore
    await runtime.commit()

    # morph str is_list=True -> int is_list=False
    Field1.morph_to(int, is_list=False)
    Record1.Field1 = 42  # type: ignore
    await runtime.commit()

    # morph back to str is_list=True (should still have old value)
    Field1.morph_to(str, is_list=True)
    await runtime.commit()
    Record1 = await Database1.records.get(Record1.to_ref())
    assert Record1.Field1 == ["Record1.1", "Record1.2"]  # type: ignore

    # morph back to str is_list=False (should still have old value)
    Field1.morph_to(str, is_list=False)
    await runtime.commit()
    Record1 = await Database1.records.get(Record1.to_ref())
    assert Record1.Field1 == "Record1"  # type: ignore
    assert Record1.Field2 is True  # type: ignore
    # delete record
    Record1.delete()
    await runtime.commit()
    records = await Database1.records.search()
    assert records == []


@simulated_runtime()
async def test_record_recursive_reference(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Record with a recursive reference to itself."""
    Database1 = Database.new("Database1")
    Database1.fields.append(Field.member("Record", Database1))
    runtime.page().append(Database1)
    Record1 = Database1.records.create(name="Record1")
    Record1.Record = Record1  # type: ignore
    assert Record1.Record == Record1  # type: ignore
    await runtime.commit()

    Record1 = await Database1.records.get(Record1.to_ref())
    assert Record1.Record == Record1  # type: ignore


@simulated_runtime()
async def test_search_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Insert, update and query Records with various filters."""
    Database1 = Database.new(
        "Database1",
        Field.member("Name", str),
        Field.member("Name", str),
        Field.member("Age", int),
        Field.member("Description", Text),
    )
    runtime.page().append(Database1)
    Record1 = Database1.records.create(
        Name="Alice", Age=30, Description=text("Alice is a *person*.")
    )
    Record2 = Database1.records.create(Name="Bob", Age=40, Description=text("Bob is a *goat*."))
    Record3 = Database1.records.create(
        Name="Charlie", Age=50, Description=text("Charlie is a *cat*.")
    )
    await runtime.commit()

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


@simulated_runtime()
async def test_database_isolation(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create two databases and ensure they don't interfere with each other."""

    Database1 = Database.new("Database1", Field.member("Name", str))
    Database2 = Database.new("Database2", Field.member("Name", str))
    runtime.page().extend(Database1, Database2)

    Record1 = Database1.records.create(Name="Record1")
    Record2 = Database2.records.create(Name="Record2")
    await runtime.commit()

    records1 = await Database1.records.search()
    assert records1 == [Record1]
    records2 = await Database2.records.search()
    assert records2 == [Record2]

    Record3 = Database1.records.create(Name="Record3")
    Record4 = Database2.records.create(Name="Record4")
    await runtime.commit()

    records1 = await Database1.records.search()
    assert records1 == [Record1, Record3]
    records2 = await Database2.records.search()
    assert records2 == [Record2, Record4]

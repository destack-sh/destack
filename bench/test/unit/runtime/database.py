from datetime import date, datetime, time, timedelta
from random import Random
from typing import Any

import pytest
from fastuuid import UUID
from grpclib import GRPCError

from bench.language import (
    PRIMITIVE_TYPE_BY_PY_TYPE,
    CustomNodeDefinition,
    Field,
    NodeType,
    PrimitiveType,
    ScalarType,
    TypeBase,
    TypeCardinality,
    text,
)
from bench.language.core.const import StructType
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime
from bench.utils.string import Casing, to_casing


class SampleGenerator:
    def __init__(self, random: Random):
        self.random = random

    def generate(self, typ: TypeBase) -> Any:
        raise NotImplementedError


@simulated_runtime()
async def test_create_record_kwargs(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    """Create a Record with keyword arguments (into value)."""
    Table1 = CustomNodeDefinition(name="Table1")
    Table1.add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
        Field(name="Age", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.INT64),
        Field(
            name="Aliases",
            scalar_type=ScalarType.PRIMITIVE,
            primitive_type=PrimitiveType.STRING,
            cardinality=TypeCardinality.LIST,
        ),
    )
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create()
    assert Record1.Name is None  # type: ignore
    assert Record1.Age is None  # type: ignore
    assert Record1.Aliases is None  # type: ignore

    Record2 = Table1.records.create(Name="Record2", Aliases=["R3", "R4"])
    assert Record2.Name == "Record2"  # type: ignore
    assert Record2.Age is None  # type: ignore
    assert Record2.Aliases == ["R3", "R4"]  # type: ignore
    with pytest.raises(AttributeError):
        _ = Record2.NonExistent  # type: ignore
    assert Record2.value_packed is not None
    assert Record2.value_packed[Table1.child(Field, "Name").storage_key] == "Record2"


@simulated_runtime()
async def test_create_empty_table_block(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a blank table and query it."""
    Table1 = CustomNodeDefinition(name="Table1")
    runtime.page().add_child(Table1)

    # cannot access table before committing it
    with simulation.raises(GRPCError):
        _ = await Table1.records.search()

    # commit to create table
    await runtime.commit()

    # should be empty
    records = await Table1.records.search()
    assert records == []


@simulated_runtime()
async def test_create_table_and_records_simultaneously(
    simulation: Simulation, runtime: RuntimeLambdaWorkload
):
    """Create a table and records within it in the same transaction/commit."""
    Table1 = CustomNodeDefinition(name="Table1")
    Table1.add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
    )
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(Name="Record1")
    Record2 = Table1.records.create(Name="Record2")
    await runtime.commit()

    records = await Table1.records.search()
    assert records == [Record1, Record2]
    assert records[0].Name == "Record1"  # type: ignore
    assert records[1].Name == "Record2"  # type: ignore


@simulated_runtime()
async def test_update_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Update a record with a simple Field and query it."""
    Table1 = CustomNodeDefinition(name="Table1")
    Table1.add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
    )
    runtime.page().add_child(Table1)

    # create & query
    Record1 = Table1.records.create(Name="Record1")
    await runtime.commit()
    records = await Table1.records.search()
    assert records[0].Name == "Record1"  # type: ignore

    # update & query
    Record1.Name = "Record1.1"  # type: ignore
    await runtime.commit()
    records = await Table1.records.search()
    assert records[0].Name == "Record1.1"  # type: ignore


@simulated_runtime()
async def test_update_table_and_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Updates a table and records within and across transactions."""
    Table1 = CustomNodeDefinition(name="Table1")
    Table1.add_children(
        Field(name="Id", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.INT64),
    )
    runtime.page().add_child(Table1)
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
            field = Field(
                name=field_name,
                scalar_type=ScalarType.PRIMITIVE,
                primitive_type=PRIMITIVE_TYPE_BY_PY_TYPE[sample_type],
                cardinality=TypeCardinality.LIST if is_list else TypeCardinality.SCALAR,
            )
            Table1.add_child(field)
            sample_field_value = sampler.generate(field)
            record = Table1.records.create(Id=i, **{field_name: sample_field_value})
            cached_records.append(record)
            await runtime.commit()

    # query
    stored_records = await Table1.records.order_by(Table1.child(Field, "Id")).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)

    # update
    for field, record in zip(Table1.get_children(Field), stored_records):
        assert field.name is not None, f"field {field!r} has no name"
        sample_field_value = sampler.generate(field)
        setattr(record, field.name, sample_field_value)
    await runtime.commit()

    # query again
    stored_records = await Table1.records.order_by(Table1.child(Field, "Id")).search()
    for stored_record, cached_record in zip(stored_records, cached_records):
        assert stored_record.equals(cached_record)


@simulated_runtime()
async def test_create_record_with_ptrs(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Table with pointer fields (scalar and list)."""
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Block", scalar_type=ScalarType.NODE, node_type=NodeType.BLOCK),
        Field(
            name="Blocks",
            scalar_type=ScalarType.NODE,
            node_type=NodeType.BLOCK,
            cardinality=TypeCardinality.LIST,
        ),
    )
    runtime.page().add_child(Table1)
    await runtime.commit()

    Record1 = Table1.records.create(Block=Table1, Blocks=[Table1])
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record1]


@simulated_runtime()
async def test_move_table(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Move a table between parents while creating a Record."""
    # create table in Page1
    Page1 = runtime.page("Page1")
    Page2 = runtime.page("Page2")
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Alias", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
        Field(name="Image", scalar_type=ScalarType.NODE, node_type=NodeType.FILE),
    )
    Page1.add_child(Table1)
    Record1 = Table1.records.create(name="Record1", Alias="1")
    await runtime.commit()

    # query table in Page1
    records = await Table1.records.search()
    assert records == [Record1]

    # move table to Page2
    Table1.move_to(parent=Page2)
    Record2 = Table1.records.create(name="Record2", Alias="2")
    await runtime.commit()

    # query table in Page2
    records = await Table1.records.search()
    assert records == [Record1, Record2]

    # delete Record1
    Record1.delete()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record2]


@simulated_runtime()
async def test_delete_restore_table(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Delete a table, querying it shouldn't work. Restore, and it should work again."""
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
    )
    Record1 = Table1.records.create(Name="Record1")
    runtime.page().add_child(Table1)
    await runtime.commit()

    # delete
    Table1.delete()
    await runtime.commit()
    with simulation.raises(GRPCError):
        _ = await Table1.records.search()

    # restore
    Table1.restore()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record1]


@simulated_runtime()
async def test_delete_restore_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Deleting a Record should remove it from default view, restoring should re-add it."""
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
    )
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(Name="Record1")
    Record2 = Table1.records.create(Name="Record2")
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record1, Record2]

    # delete
    Record1.delete()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record2]

    # insert
    Record3 = Table1.records.create(Name="Record3")
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record2, Record3]

    # delete
    Record2.delete()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record3]

    # restore
    Record1.restore()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == [Record1, Record3]


@simulated_runtime()
async def test_delete_restore_table_field(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Delete and restore a Field in a Table."""
    Field1 = Field(
        name="Field1", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING
    )
    Table1 = CustomNodeDefinition(name="Table1").add_children(Field1)
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(Field1="Record1")
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
async def test_morph_table_field_type(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """
    Update a Fields type and set/get its values.
    The values belonging to different types should be preserved (should map to different columns).
    """
    # initial str is_list=False
    Field1 = Field(
        name="Field1", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING
    )
    Field2 = Field(
        name="Field2", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.BOOLEAN
    )
    Table1 = CustomNodeDefinition(name="Table1").add_children(Field1, Field2)
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(Field1="Record1", Field2=True)
    await runtime.commit()

    # morph str is_list=False -> str is_list=True
    Field1.cardinality = TypeCardinality.LIST
    Record1.Field1 = ["Record1.1", "Record1.2"]  # type: ignore
    await runtime.commit()

    # morph str is_list=True -> int is_list=False
    Field1.primitive_type = PrimitiveType.INT64
    Field1.cardinality = TypeCardinality.SCALAR
    Record1.Field1 = 42  # type: ignore
    await runtime.commit()

    # morph back to str is_list=True (should still have old value)
    Field1.primitive_type = PrimitiveType.STRING
    Field1.cardinality = TypeCardinality.LIST
    await runtime.commit()
    Record1 = await Table1.records.get(Record1.to_ref())
    assert Record1.Field1 == ["Record1.1", "Record1.2"]  # type: ignore

    # morph back to str is_list=False (should still have old value)
    Field1.primitive_type = PrimitiveType.STRING
    Field1.cardinality = TypeCardinality.SCALAR
    await runtime.commit()
    Record1 = await Table1.records.get(Record1.to_ref())
    assert Record1.Field1 == "Record1"  # type: ignore
    assert Record1.Field2 is True  # type: ignore
    # delete record
    Record1.delete()
    await runtime.commit()
    records = await Table1.records.search()
    assert records == []


@simulated_runtime()
async def test_record_recursive_reference(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create a Record with a recursive reference to itself."""
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Record", scalar_type=ScalarType.NODE, node_type=NodeType.CUSTOM_NODE_INSTANCE)
    )
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(name="Record1")
    Record1.Record = Record1  # type: ignore
    assert Record1.Record == Record1  # type: ignore
    await runtime.commit()

    Record1 = await Table1.records.get(Record1.to_ref())
    assert Record1.Record == Record1  # type: ignore


@simulated_runtime()
async def test_search_record(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Insert, update and query Records with various filters."""
    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING),
        Field(name="Age", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.INT64),
        Field(name="Description", scalar_type=ScalarType.STRUCT, struct_type=StructType.TEXT),
    )
    runtime.page().add_child(Table1)
    Record1 = Table1.records.create(Name="Alice", Age=30, Description=text("Alice is a *person*."))
    Record2 = Table1.records.create(Name="Bob", Age=40, Description=text("Bob is a *goat*."))
    Record3 = Table1.records.create(Name="Charlie", Age=50, Description=text("Charlie is a *cat*."))
    await runtime.commit()

    # get by ref
    Result = await Table1.records.get(Record1.to_ref())
    assert Result == Record1

    # get by id
    Result = await Table1.records.get(id=Record2.id)
    assert Result == Record2

    # get by custom column
    Result = await Table1.records.get(Table1.child(Field, "Age") == 40)  # type: ignore
    assert Result == Record2

    # get by custom column
    Result = await Table1.records.get(Name="Charlie")
    assert Result == Record3

    # filter by custom column
    Result = await Table1.records.search(Table1.child(Field, "Age").gte(40))
    assert Result == [Record2, Record3]

    # order by custom column
    Result = await Table1.records.order_by(Table1.child(Field, "Age")).search()
    assert Result == [Record1, Record2, Record3]


@simulated_runtime()
async def test_table_isolation(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    """Create two tables and ensure they don't interfere with each other."""

    Table1 = CustomNodeDefinition(name="Table1").add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING)
    )
    Table2 = CustomNodeDefinition(name="Table2").add_children(
        Field(name="Name", scalar_type=ScalarType.PRIMITIVE, primitive_type=PrimitiveType.STRING)
    )
    runtime.page().add_children(Table1, Table2)

    Record1 = Table1.records.create(Name="Record1")
    Record2 = Table2.records.create(Name="Record2")
    await runtime.commit()

    records1 = await Table1.records.search()
    assert records1 == [Record1]
    records2 = await Table2.records.search()
    assert records2 == [Record2]

    Record3 = Table1.records.create(Name="Record3")
    Record4 = Table2.records.create(Name="Record4")
    await runtime.commit()

    records1 = await Table1.records.search()
    assert records1 == [Record1, Record3]
    records2 = await Table2.records.search()
    assert records2 == [Record2, Record4]

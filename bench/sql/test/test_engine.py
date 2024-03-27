import random
import string
from typing import Callable, Mapping

import psycopg
import pytest
from psycopg import sql

from bench.conftest import test_session
from bench.language import (
    Bench,
    Block,
    BlockType,
    Client,
    NodeReference,
    PrimitiveType,
    Region,
    Server,
    ServerProfile,
    StoreEngineType,
    StoreKind,
)
from bench.language.const import NodeType
from bench.language.test.fabricator import Fabricator
from bench.sql.core import Column, Table
from bench.sql.engine import (
    RowIn,
    _pg_adapt_row,
    _pg_adapt_rows,
    pg_delete,
    pg_insert,
    pg_select,
    pg_update_constant,
    pg_update_variable,
    pg_upsert,
)
from bench.sql.migration import force_create_tables

_TEST_TYPES = (
    PrimitiveType.BOOLEAN,
    PrimitiveType.INT32,
    PrimitiveType.FLOAT32,
    PrimitiveType.STRING,
    PrimitiveType.JSON,
    PrimitiveType.BYTES,
)
_MINI_REGULAR_TABLE = Table(
    "_test_mini_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        Column("foo", PrimitiveType.STRING),
        Column("foo_n", PrimitiveType.STRING, is_nullable=True),
    ),
)
_REGULAR_TABLE = Table(
    "_test_regular_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        *(Column(f"regular_{t.name.lower()}", t) for t in _TEST_TYPES),
        *(Column(f"regular_{t.name.lower()}_n", t, is_nullable=True) for t in _TEST_TYPES),
        *(Column(f"regular_{t.name.lower()}_a", t, is_array=True) for t in _TEST_TYPES),
    ),
)
_MINI_ENCRYPTED_TABLE = Table(
    "_test_mini_encrypted_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        Column("secret", PrimitiveType.STRING, is_encrypted=True),
        Column("secret_n", PrimitiveType.STRING, is_encrypted=True, is_nullable=True),
    ),
)
_ENCRYPTED_TABLE = Table(
    "_test_encrypted_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        *(Column(f"secret_{t.name.lower()}", t, is_encrypted=True) for t in _TEST_TYPES),
        *(
            Column(f"secret_{t.name.lower()}_n", t, is_encrypted=True, is_nullable=True)
            for t in _TEST_TYPES
        ),
    ),
)
_TEST_TABLES = [_MINI_REGULAR_TABLE, _REGULAR_TABLE, _MINI_ENCRYPTED_TABLE, _ENCRYPTED_TABLE]

COLUMN_VALUE_GENERATORS: Mapping[PrimitiveType, Callable[[], any]] = {
    PrimitiveType.BOOLEAN: lambda: random.choice((True, False)),
    PrimitiveType.INT32: lambda: random.randint(0, 2**31 - 1),
    PrimitiveType.FLOAT32: lambda: random.random().__round__(3),  # for comparison stability
    PrimitiveType.STRING: lambda: "".join(random.choices(string.ascii_letters, k=10)),
    PrimitiveType.JSON: lambda: {"foo": "bar", "nested": [1, 2, 3]},
    PrimitiveType.BYTES: lambda: random.randbytes(20),
}


@pytest.fixture(autouse=True, scope="module")
async def test_tables():
    from bench.sql.client import get_pg_connection_str, pg_cursor
    from bench.system.client import GLOBAL_STORE

    async with pg_cursor(get_pg_connection_str(GLOBAL_STORE, "test")) as cur:
        await force_create_tables(cur, _TEST_TABLES)


@pytest.mark.parametrize("table", _TEST_TABLES, ids=lambda t: t.name)
async def test_crud_rows(test_cur: psycopg.AsyncCursor, table: Table):
    from bench.sql.client import _current_pg_crypto_key

    random.seed(42)
    _current_pg_crypto_key.set(random.randbytes(32).hex())

    def _generate_row(id: int) -> Mapping[str, any]:
        row = {"id": id}
        for column in table.columns:
            if column.is_primary_key:
                continue
            elif column.is_nullable and random.random() < 0.5:
                row[column.name] = None
            elif column.is_array:
                row[column.name] = [
                    COLUMN_VALUE_GENERATORS[column.underlying_type]() for _ in range(0, 3)
                ]
            else:
                row[column.name] = COLUMN_VALUE_GENERATORS[column.underlying_type]()
        return row

    # insert
    initial_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(0, 3))
    target_rows = list(initial_rows)
    db_rows = await pg_insert(
        test_cur, table, _pg_adapt_rows(table, initial_rows), returning=table.columns
    )
    assert db_rows == target_rows
    db_rows = await pg_select(test_cur, table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # upsert
    upsert_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(2, 5))
    await pg_upsert(test_cur, table, _pg_adapt_rows(table, upsert_rows))
    target_rows = target_rows[:2] + list(upsert_rows)
    db_rows = await pg_select(test_cur, table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with dynamic values
    update_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(1, 4))
    target_rows = target_rows[:1] + list(update_rows) + target_rows[4:]
    db_rows = await pg_update_variable(
        cur=test_cur,
        table=table,
        dynamic_columns=table.columns,
        dynamic_values=_pg_adapt_rows(table, update_rows),
        returning=table.columns,
    )
    db_rows.sort(key=lambda r: r["id"])
    assert db_rows == target_rows[1:4]
    db_rows = await pg_select(test_cur, table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with fixed values
    static_value = {
        column.name: COLUMN_VALUE_GENERATORS[column.underlying_type]()
        for i, column in enumerate(table.columns)
        if not column.is_primary_key and not column.is_array and i % 2 == 0
    }
    target_rows = [{**row, **static_value} for row in target_rows]
    db_rows = await pg_update_constant(
        test_cur, table, static_value=_pg_adapt_row(table, static_value), returning=table.columns
    )
    db_rows.sort(key=lambda r: r["id"])
    assert db_rows == target_rows
    db_rows = await pg_select(test_cur, table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # delete
    await pg_delete(test_cur, table, where=sql.SQL("id > 3"))
    target_rows = target_rows[:4]
    db_rows = await pg_select(test_cur, table, order_by=sql.SQL("id"))
    db_rows.sort(key=lambda r: r["id"])
    assert db_rows == target_rows


# TODO :Test: test node-level operations more generally (maybe as part of graph testing)


async def test_crud_node_pointers(fabricator: "Fabricator"):
    """Ensures that node pointers (parent, regular, ancestor) roundtrip correctly"""
    # write
    async with test_session() as session:
        bench_a: Bench = Bench(
            slug="test_a", name="test_b", region=Region.GLOBAL, encryption_key="yo"
        )
        session.create(bench_a)
        await session.flush()
        server_a = bench_a.servers.create(name="Production A", profile=ServerProfile.TINY)
        store_a = bench_a.stores.create(
            name="Production A", kind=StoreKind.RELATIONAL, engine=StoreEngineType.POSTGRES
        )
        drive_a = bench_a.drives.create(name="Production A")
        client_a = server_a.clients.create(name="Testificate's iPhone")
        environment_a = bench_a.environments.create(
            name="main a", server=server_a, store=store_a, search=store_a, drive=drive_a
        )
        bench_a.branches.create(name="main a")
        package_a = bench_a.packages.create(environment=environment_a)
        block_a_1 = package_a.blocks.create(type=BlockType.CODE_ROUTINE)
        block_a_1.fields.create(name="foo")
        await session.commit()

    # read back
    async with test_session() as session:
        server_a = await Server.include_ancestors().get(id=server_a.id)
        assert server_a.parent_ptr.equals_content(bench_a.to_ref())
        assert server_a.bench_id == bench_a.id
        assert server_a.to_ref().equals_content(
            NodeReference(type=NodeType.SERVER, id=server_a.id, bench_id=bench_a.id)
        )

        client_a = await Client.include_ancestors().get(id=client_a.id)
        assert client_a.bench_id == bench_a.id
        assert client_a.to_ref().equals_content(
            NodeReference(type=NodeType.CLIENT, id=client_a.id, bench_id=bench_a.id)
        )

        block_a_1 = await Block.include_ancestors().get(id=block_a_1.id)
        assert block_a_1.bench_id == bench_a.id
        assert block_a_1.to_ref().equals_content(
            NodeReference(
                type=NodeType.BLOCK, id=block_a_1.id, ck=block_a_1.ck, bench_id=bench_a.id
            )
        )

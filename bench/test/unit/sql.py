# TODO :Robustness :Cleanup: type-check sql engine

import random
import string
from typing import Any, Callable, Mapping, cast

import psycopg
import pytest
from psycopg import sql

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
    Store,
)
from bench.language.const import ClientType, EnumType, NodeType, UserStatus
from bench.language.field import TypeKind
from bench.language.query import NodeNotFoundError
from bench.language.session import Session
from bench.language.user import User
from bench.sql.client import pg_store_connection
from bench.sql.core import GLOBAL_EXTENSIONS, Column, ObjectKind, Schema, Table
from bench.sql.engine import (
    GLOBAL_SCHEMA,
    LOCAL_SCHEMA,
    RowIn,
    _pg_adapt_row,
    _pg_adapt_rows,
    pg_delete,
    pg_insert,
    pg_select,
    pg_update_static,
    pg_update_variable,
    pg_upsert,
)
from bench.sql.migration import (
    force_create_schema,
    generate_sql_migration_ops,
    introspect_sql_schema,
    read_migrations_from_fs,
    sql_migrate,
)
from bench.utils.func import generate_encryption_key
from bench.utils.oracle import REAL_ORACLE

#
# Simple SQL engine only tests
#

TEST_PRIMITIVE_TYPES = (
    PrimitiveType.BOOLEAN,
    PrimitiveType.INT32,
    PrimitiveType.FLOAT32,
    PrimitiveType.STRING,
    PrimitiveType.JSON,
    PrimitiveType.BYTES,
)
MINI_REGULAR_TABLE = Table(
    "_test_mini_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        Column("foo", PrimitiveType.STRING),
        Column("foo_n", PrimitiveType.STRING, is_nullable=True),
    ),
)
REGULAR_TABLE = Table(
    "_test_regular_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        *(Column(f"regular_{t.name.lower()}", t) for t in TEST_PRIMITIVE_TYPES),
        *(Column(f"regular_{t.name.lower()}_n", t, is_nullable=True) for t in TEST_PRIMITIVE_TYPES),
        *(Column(f"regular_{t.name.lower()}_a", t, is_array=True) for t in TEST_PRIMITIVE_TYPES),
    ),
)
MINI_ENCRYPTED_TABLE = Table(
    "_test_mini_encrypted_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        Column("secret", PrimitiveType.STRING, is_encrypted=True),
        Column("secret_n", PrimitiveType.STRING, is_encrypted=True, is_nullable=True),
    ),
)
ENCRYPTED_TABLE = Table(
    "_test_encrypted_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        *(Column(f"secret_{t.name.lower()}", t, is_encrypted=True) for t in TEST_PRIMITIVE_TYPES),
        *(
            Column(f"secret_{t.name.lower()}_n", t, is_encrypted=True, is_nullable=True)
            for t in TEST_PRIMITIVE_TYPES
        ),
    ),
)
TEST_TABLES = (MINI_REGULAR_TABLE, REGULAR_TABLE, MINI_ENCRYPTED_TABLE, ENCRYPTED_TABLE)
TEST_SCHEMA = Schema(extensions=GLOBAL_EXTENSIONS, tables=TEST_TABLES)

# NOTE :Test: convert sql test values to hypothesis strategies?
COLUMN_VALUE_GENERATORS: Mapping[PrimitiveType, Callable[[], Any]] = {
    PrimitiveType.BOOLEAN: lambda: random.choice((True, False)),
    PrimitiveType.INT32: lambda: random.randint(0, 2**31 - 1),
    PrimitiveType.FLOAT32: lambda: random.random().__round__(3),  # for comparison stability
    PrimitiveType.STRING: lambda: "".join(random.choices(string.ascii_letters, k=10)),
    PrimitiveType.JSON: lambda: {"foo": "bar", "nested": [1, 2, 3]},
    PrimitiveType.BYTES: lambda: random.randbytes(20),
}


@pytest.fixture()
async def blank_cur(blank_store: Store):
    async with pg_store_connection(blank_store, autocommit=True) as cur:
        yield cur


@pytest.fixture()
async def test_cur(blank_store: Store):
    # creates test schema
    async with pg_store_connection(blank_store, autocommit=True) as cur:
        await force_create_schema(cur, TEST_SCHEMA)
        await cur.connection.commit()

    async with pg_store_connection(blank_store) as cur:
        yield cur


@pytest.mark.parametrize("table", TEST_TABLES, ids=lambda t: t.name)
async def test_crud_rows(test_cur: psycopg.AsyncCursor, table: Table):
    from bench.sql.client import _force_pg_crypto_key

    random.seed(42)
    _force_pg_crypto_key.set(random.randbytes(32).hex())

    def _generate_row(id: int) -> Mapping[str, Any]:
        row: dict[str, Any] = {"id": id}
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
        cur=test_cur, table=table, rows=_pg_adapt_rows(table, initial_rows), returning=table.columns
    )
    assert db_rows == target_rows
    db_rows = await pg_select(cur=test_cur, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # upsert
    upsert_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(2, 5))
    await pg_upsert(cur=test_cur, table=table, rows=_pg_adapt_rows(table, upsert_rows))
    target_rows = target_rows[:2] + list(upsert_rows)
    db_rows = await pg_select(cur=test_cur, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with dynamic values (only some columns are updated in some rows)
    update_rows: list[RowIn] = []
    for id in range(1, 4):
        row = _generate_row(id)
        row = {k: v for k, v in row.items() if k == "id" or random.random() < 0.5}
        update_rows.append(row)
        target_rows[id] = {**target_rows[id], **row}
    db_rows = await pg_update_variable(
        cur=test_cur,
        table=table,
        dynamic_columns=table.columns,
        dynamic_values=_pg_adapt_rows(table, update_rows),
        returning=table.columns,
    )
    assert db_rows is not None
    db_rows.sort(key=lambda r: cast(int, r["id"]))
    assert db_rows == target_rows[1:4]
    db_rows = await pg_select(cur=test_cur, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with fixed values
    static_value = {
        column.name: COLUMN_VALUE_GENERATORS[column.underlying_type]()
        for i, column in enumerate(table.columns)
        if not column.is_primary_key and not column.is_array and i % 2 == 0
    }
    target_rows = [{**row, **static_value} for row in target_rows]
    db_rows = await pg_update_static(
        cur=test_cur,
        table=table,
        static_value=_pg_adapt_row(table, static_value),
        returning=table.columns,
    )
    assert db_rows is not None
    db_rows.sort(key=lambda r: cast(int, r["id"]))
    assert db_rows == target_rows
    db_rows = await pg_select(cur=test_cur, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # delete
    await pg_delete(cur=test_cur, table=table, where=sql.SQL("id > 3"))
    target_rows = target_rows[:4]
    db_rows = await pg_select(cur=test_cur, table=table, order_by=sql.SQL("id"))
    assert db_rows is not None
    db_rows.sort(key=lambda r: cast(int, r["id"]))
    assert db_rows == target_rows


async def _do_test_stored_migrations(cur: psycopg.AsyncCursor, *, is_global: bool):
    # run all stored migrations
    stored_migrations = read_migrations_from_fs()
    await sql_migrate(cur, target=stored_migrations[-1].id, is_global=is_global, oracle=REAL_ORACLE)

    # diff again (should be empty now)
    current_schema = await introspect_sql_schema(cur)
    new_schema = GLOBAL_SCHEMA if is_global else LOCAL_SCHEMA
    current_ops = generate_sql_migration_ops(current_schema, new_schema)
    current_ops = [op for op in current_ops if op.object_kind != ObjectKind.EXTENSION]
    assert not current_ops, f"out of sync migrations, got {len(current_ops)} ops"


async def test_stored_migrations_global(blank_cur: psycopg.AsyncCursor):
    """Existing global migrations against a blank database."""
    await _do_test_stored_migrations(blank_cur, is_global=True)


async def test_stored_migrations_local(blank_cur: psycopg.AsyncCursor):
    """Existing local migrations against a blank database."""
    await _do_test_stored_migrations(blank_cur, is_global=False)


async def test_cascade_edits(global_real_session: Session):
    """Ensure basic cascading works. A simpler, isolated version of the simulation workloads."""

    async with global_real_session as session:
        user_1 = User(
            name="Rabbit", slug="rabbit", status=UserStatus.REGISTERED, email="rabbit@symbolx.com"
        )
        session._create(user_1)
        await session.flush()

        client_1_a = Client(
            parent=user_1,
            type=ClientType.BENCH_WEB,
            seen_at=session._oracle.utc(),
            name="Rabbit's Web",
        )
        client_1_b = Client(
            parent=user_1,
            type=ClientType.BENCH_MOBILE,
            seen_at=session._oracle.utc(),
            name="Rabbit's iPhone",
        )
        client_1_c = Client(
            parent=user_1,
            type=ClientType.BENCH_MOBILE,
            seen_at=session._oracle.utc(),
            name="Rabbit's Android",
        )
        session._create(client_1_a, client_1_b, client_1_c)
        await session.commit()

        # delete non-cascading
        session._delete(client_1_c)
        await session.commit()
        with pytest.raises(NodeNotFoundError):
            await Client.get(id=client_1_c.id)

        # delete cascading
        session._delete(user_1)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 2
        with pytest.raises(NodeNotFoundError):
            await User.get(id=user_1.id)
        with pytest.raises(NodeNotFoundError):
            await Client.get(id=client_1_a.id)

        # restore cascading
        session._restore(user_1)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 2
        assert await User.get(id=user_1.id)
        assert await Client.get(id=client_1_a.id)
        with pytest.raises(NodeNotFoundError):  # should only restore its own deleted children
            await Client.get(id=client_1_c.id)

        # restore non-cascading
        session._restore(client_1_c)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 0
        assert await Client.get(id=client_1_c.id)


async def test_crud_node_pointers(global_real_session: Session):
    """Ensures that node pointers (parent, regular, ancestor) roundtrip correctly."""
    async with global_real_session as session:
        # write
        bench: Bench = Bench(
            slug="test",
            name="test_b",
            region=Region.GLOBAL,
            encryption_key=generate_encryption_key(32),
        )
        session._create(bench)
        await session.flush()
        server = bench.servers.create(name="Production A", profile=ServerProfile.TINY)
        store = bench.stores.create(name="Production A")
        drive = bench.drives.create(name="Production A")
        client = server.clients.create(
            type=ClientType.BENCH_MOBILE, seen_at=session._oracle.utc(), name="Testificate's iPhone"
        )
        environment = bench.environments.create(
            name="main a", server=server, store=store, drive=drive
        )
        branch = bench.branches.create(name="main a")
        package = branch.packages.create(environment=environment)
        await session.flush()
        session.parent = package
        branch.main_package = package
        bench.main_environment = environment
        bench.main_branch = branch
        await session.commit()
        bench._untrack_rec()

        session.track(bench)
        block_1 = package.blocks.create(type=BlockType.CODE)
        block_1.fields.create(name="foo", kind=TypeKind.ENUM, bench_type=EnumType.PRIMITIVE_TYPE)
        await session.commit()

        # read back
        server = await Server.include_ancestors().get(id=server.id)
        assert server.parent_ptr
        assert server.parent_ptr._equals_content(bench.to_ref())
        assert server.bench_id == bench.id
        assert server.to_ref()._equals_content(
            NodeReference(type=NodeType.SERVER, id=server.id, ck=server.ck, bench_id=bench.id)
        )

        client = await Client.include_ancestors().get(id=client.id)
        assert client.bench_id == bench.id
        assert client.to_ref()._equals_content(
            NodeReference(type=NodeType.CLIENT, id=client.id, ck=client.ck, bench_id=bench.id)
        )

        block_1 = await Block.include_ancestors().get(id=block_1.id)
        assert block_1.bench_id == bench.id
        assert block_1.to_ref()._equals_content(
            NodeReference(type=NodeType.BLOCK, id=block_1.id, ck=block_1.ck, bench_id=bench.id)
        )

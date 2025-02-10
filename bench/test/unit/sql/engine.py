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
    ClientType,
    NodeNotFoundError,
    NodeReference,
    NodeType,
    PackageType,
    PrimitiveType,
    Region,
    Session,
    Store,
    User,
    UserStatus,
    View,
    ViewType,
)
from bench.sql import (
    GLOBAL_EXTENSIONS,
    Column,
    RowIn,
    Schema,
    StaticContext,
    Table,
    force_create_schema,
    pg_connection,
    pg_delete,
    pg_insert,
    pg_select,
    pg_update_static,
    pg_update_variable,
    pg_upsert,
)
from bench.sql.engine import _pg_adapt_row, _pg_adapt_rows
from bench.utils.func import generate_encryption_key

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


@pytest.fixture
async def test_cur(blank_store: Store):
    # creates test schema
    async with pg_connection(blank_store, owner=blank_store, autocommit=True) as conn:
        await force_create_schema(conn.cursor, TEST_SCHEMA)
        await conn.commit()

    async with pg_connection(blank_store, owner=blank_store) as conn:
        yield conn.cursor


@pytest.mark.parametrize("table", TEST_TABLES, ids=lambda t: t.name)
async def test_crud_rows(test_cur: psycopg.AsyncCursor, table: Table):
    random.seed(42)
    ctx = StaticContext(crypto_key=random.randbytes(32).hex())

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
    _ = await pg_insert(
        cur=test_cur,
        ctx=ctx,
        table=table,
        rows=_pg_adapt_rows(table, initial_rows),
    )
    db_rows = await pg_select(cur=test_cur, ctx=ctx, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # upsert
    upsert_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(2, 5))
    await pg_upsert(cur=test_cur, ctx=ctx, table=table, rows=_pg_adapt_rows(table, upsert_rows))
    target_rows = target_rows[:2] + list(upsert_rows)
    db_rows = await pg_select(cur=test_cur, ctx=ctx, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with dynamic values (only some columns are updated in some rows)
    update_rows: list[RowIn] = []
    for id in range(1, 4):
        row = _generate_row(id)
        row = {k: v for k, v in row.items() if k == "id" or random.random() < 0.5}
        update_rows.append(row)
        target_rows[id] = {**target_rows[id], **row}
    _ = await pg_update_variable(
        cur=test_cur,
        ctx=ctx,
        table=table,
        dynamic_columns=table.columns,
        dynamic_values=_pg_adapt_rows(table, update_rows),
    )
    db_rows = await pg_select(cur=test_cur, ctx=ctx, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update with fixed values
    static_value = {
        column.name: COLUMN_VALUE_GENERATORS[column.underlying_type]()
        for i, column in enumerate(table.columns)
        if not column.is_primary_key and not column.is_array and i % 2 == 0
    }
    target_rows = [{**row, **static_value} for row in target_rows]
    _ = await pg_update_static(
        cur=test_cur,
        ctx=ctx,
        table=table,
        static_value=_pg_adapt_row(table, static_value),
    )
    db_rows = await pg_select(cur=test_cur, ctx=ctx, table=table, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # delete
    _ = await pg_delete(cur=test_cur, ctx=ctx, table=table, where=sql.SQL("id > 3"))
    target_rows = target_rows[:4]
    db_rows = await pg_select(cur=test_cur, ctx=ctx, table=table, order_by=sql.SQL("id"))
    assert db_rows is not None
    db_rows = list(db_rows)
    db_rows.sort(key=lambda r: cast(int, r["id"]))
    assert db_rows == target_rows


async def test_cascade_edits(omni_session: Session):
    """Ensure basic cascading works. A simpler, isolated version of the simulation workloads."""

    async with omni_session as session:
        user_1 = User(
            name="Rabbit",
            slug="rabbit",
            region=Region.ZURICH,
            status=UserStatus.REGISTERED,
            email="rabbit@symbolx.com",
        )
        session._create(user_1)
        await session.flush()

        client_1_a = Client(
            parent=user_1,
            type=ClientType.WEB,
            seen_at=session._oracle.utc(),
            name="Rabbit's Web",
        )
        client_1_b = Client(
            parent=user_1,
            type=ClientType.MOBILE,
            seen_at=session._oracle.utc(),
            name="Rabbit's iPhone",
        )
        client_1_c = Client(
            parent=user_1,
            type=ClientType.MOBILE,
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


async def test_crud_node_pointers(omni_session: Session):
    """Ensures that node pointers (parent, regular, ancestor, ...) roundtrip correctly."""
    async with omni_session as session:
        # write
        bench: Bench = Bench(
            slug="test",
            name="test_b",
            region=Region.ZURICH,
            encryption_key=generate_encryption_key(32),
        )
        session._create(bench)
        await session.flush()
        store = bench.stores.create(name="Production A")
        client = Client(
            parent=bench,
            type=ClientType.MOBILE,
            seen_at=session._oracle.utc(),
            name="Testificate's iPhone",
        )
        session._create(client)
        package = bench.packages.create(type=PackageType.MAIN, name="Main", slug="main")
        await session.flush()
        session.parent = bench  # patch in the session parent
        bench.main_store = store
        await session.commit()
        bench._untrack_rec()

        session._track(bench)
        page1 = package.pages.create()
        view1 = View.new(ViewType.COLOR, "View1")
        page1.append(view1)
        block1 = page1.blocks.create(type=BlockType.VIEW)
        view11 = view1.views.create(type=ViewType.COLOR, name="View1")  # noqa: F841
        await session.commit()

        # read back
        client = await Client.include_ancestors().get(id=client.id)
        assert client.bench_id == bench.id
        assert client.to_ref().equals(
            NodeReference(node_type=NodeType.CLIENT, id=client.id, ck=client.ck, bench_id=bench.id)
        )

        block1 = await Block.include_ancestors().get(id=block1.id)
        assert block1.bench_id == bench.id
        assert block1.to_ref().equals(
            NodeReference(node_type=NodeType.BLOCK, id=block1.id, ck=block1.ck, bench_id=bench.id)
        )

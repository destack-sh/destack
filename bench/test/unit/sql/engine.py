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
    Database,
    NodeNotFoundError,
    NodeReference,
    NodeType,
    Package,
    PackageType,
    Page,
    PrimitiveType,
    Region,
    Session,
    ThreadView,
    User,
    UserStatus,
)
from bench.sql import (
    GLOBAL_EXTENSIONS,
    NullContext,
    RowIn,
    SqlColumn,
    SqlSchema,
    SqlTable,
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
MINI_REGULAR_TABLE = SqlTable(
    "_test_mini_table",
    columns=(
        SqlColumn("id", PrimitiveType.INT32, is_primary_key=True),
        SqlColumn("foo", PrimitiveType.STRING),
        SqlColumn("foo_n", PrimitiveType.STRING, is_nullable=True),
    ),
)
REGULAR_TABLE = SqlTable(
    "_test_regular_table",
    columns=(
        SqlColumn("id", PrimitiveType.INT32, is_primary_key=True),
        *(SqlColumn(f"regular_{t.name.lower()}", t) for t in TEST_PRIMITIVE_TYPES),
        *(
            SqlColumn(f"regular_{t.name.lower()}_n", t, is_nullable=True)
            for t in TEST_PRIMITIVE_TYPES
        ),
        *(SqlColumn(f"regular_{t.name.lower()}_a", t, is_array=True) for t in TEST_PRIMITIVE_TYPES),
    ),
)

TEST_TABLES = (MINI_REGULAR_TABLE, REGULAR_TABLE)
TEST_SCHEMA = SqlSchema(extensions=GLOBAL_EXTENSIONS, tables=TEST_TABLES)

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
async def test_cur(blank_database: Database):
    # creates test schema
    async with pg_connection(blank_database, owner=blank_database, autocommit=True) as conn:
        await force_create_schema(conn.cursor, TEST_SCHEMA)
        await conn.commit()

    async with pg_connection(blank_database, owner=blank_database) as conn:
        yield conn.cursor


@pytest.mark.parametrize("table", TEST_TABLES, ids=lambda t: t.name)
async def test_crud_rows(test_cur: psycopg.AsyncCursor, table: SqlTable):
    random.seed(42)
    ctx = NullContext()

    def _generate_row(id: int) -> Mapping[str, Any]:
        row: dict[str, Any] = {"id": id}
        for column in table.columns:
            if column.is_primary_key:
                continue
            elif column.is_nullable and random.random() < 0.5:
                row[column.name] = None
            elif column.is_array:
                row[column.name] = [COLUMN_VALUE_GENERATORS[column.type]() for _ in range(0, 3)]
            else:
                row[column.name] = COLUMN_VALUE_GENERATORS[column.type]()
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
        column.name: COLUMN_VALUE_GENERATORS[column.type]()
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
        session._create(client_1_a)
        session._create(client_1_b)
        session._create(client_1_c)
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

        # redatabase cascading
        session._restore(user_1)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 2
        assert await User.get(id=user_1.id)
        assert await Client.get(id=client_1_a.id)
        with pytest.raises(NodeNotFoundError):  # should only redatabase its own deleted children
            await Client.get(id=client_1_c.id)

        # redatabase non-cascading
        session._restore(client_1_c)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 0
        assert await Client.get(id=client_1_c.id)

        # archive non-cascading
        session._archive(client_1_c)
        await session.commit()
        with pytest.raises(NodeNotFoundError):
            await Client.get(id=client_1_c.id)

        # archive cascading
        session._archive(user_1)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 2
        with pytest.raises(NodeNotFoundError):
            await User.get(id=user_1.id)
        with pytest.raises(NodeNotFoundError):
            await Client.get(id=client_1_a.id)

        # unarchive cascading
        session._unarchive(user_1)
        edits, cascaded_edits = await session.commit()
        assert len(edits) == 1
        assert len(cascaded_edits) == 2
        assert await User.get(id=user_1.id)
        assert await Client.get(id=client_1_a.id)
        with pytest.raises(NodeNotFoundError):  # should only unarchive its own archived children
            await Client.get(id=client_1_c.id)

        # unarchive non-cascading
        session._unarchive(client_1_c)
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
        )
        session._create(bench)
        await session.flush()
        package = bench.add_child(Package(type=PackageType.OPEN, name="Main", slug="main"))
        database = package.add_child(Database(name="Production A"))
        client = Client(
            parent=bench,
            type=ClientType.MOBILE,
            seen_at=session._oracle.utc(),
            name="Testificate's iPhone",
        )
        session._create(client)
        await session.flush()
        session.parent = bench  # patch in the session parent
        bench.database = database
        await session.commit()
        bench._untrack_rec()

        session._track(bench)
        page1 = Page()
        package.add_child(page1)
        view1 = ThreadView(name="View1")
        page1.add_child(view1)
        block1 = Block(type=BlockType.PARAGRAPH)
        page1.add_child(block1)
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

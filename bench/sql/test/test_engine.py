import random
import string
from typing import Callable, Mapping

import psycopg
from psycopg import sql
from psycopg.types.json import Jsonb
import pytest

from bench.language import PrimitiveType
from bench.sql.core import Table, Column
from bench.sql.engine import (
    RowIn,
    pg_insert,
    pg_select,
    pg_upsert,
    pg_update_dynamic,
    pg_delete,
    pg_update_static,
)
from bench.sql.migration import force_create_tables

_ENCRYPTED_TYPES = (
    PrimitiveType.BOOLEAN,
    PrimitiveType.INT32,
    PrimitiveType.FLOAT32,
    PrimitiveType.STRING,
    PrimitiveType.JSON,
    PrimitiveType.BYTES,
)
_ENCRYPTED_TABLE = Table(
    "_test_encrypted_table",
    columns=(
        Column("id", PrimitiveType.INT32, is_primary_key=True),
        *(Column(f"secret_{t.name.lower()}", t, is_encrypted=True) for t in _ENCRYPTED_TYPES),
    ),
)


@pytest.fixture(autouse=True, scope="module")
async def encrypted_table(test_cur: psycopg.AsyncCursor):
    await force_create_tables(test_cur, [_ENCRYPTED_TABLE])


random.seed(42)

COLUMN_VALUE_GENERATORS: Mapping[PrimitiveType, Callable[[], any]] = {
    PrimitiveType.BOOLEAN: lambda: random.choice((True, False)),
    PrimitiveType.INT32: lambda: random.randint(-(2**31), 2**31 - 1),
    PrimitiveType.FLOAT32: lambda: random.random(),
    PrimitiveType.STRING: lambda: "".join(random.choices(string.ascii_letters, k=10)),
    PrimitiveType.JSON: lambda: Jsonb({"foo": "bar", "nested": [1, 2, 3]}),
    PrimitiveType.BYTES: lambda: random.randbytes(20),
}


async def test_crud_encrypted_columns(test_cur: psycopg.AsyncCursor):
    def _generate_row(id: int) -> dict[str, any]:
        row = {f"secret_{t.name.lower()}": COLUMN_VALUE_GENERATORS[t]() for t in _ENCRYPTED_TYPES}
        row["id"] = id
        return row

    # insert
    initial_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(0, 3))
    target_rows = list(initial_rows)
    await pg_insert(test_cur, _ENCRYPTED_TABLE, initial_rows)
    db_rows = await pg_select(test_cur, _ENCRYPTED_TABLE, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # upsert
    upsert_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(2, 5))
    target_rows = target_rows[:2] + list(upsert_rows)
    await pg_upsert(test_cur, _ENCRYPTED_TABLE, upsert_rows)
    db_rows = await pg_select(test_cur, _ENCRYPTED_TABLE, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update per row
    update_rows: tuple[RowIn, ...] = tuple(_generate_row(id) for id in range(1, 4))
    await pg_update_dynamic(test_cur, _ENCRYPTED_TABLE, dynamic_values=update_rows)
    target_rows = target_rows[:1] + list(update_rows) + target_rows[4:]
    db_rows = await pg_select(test_cur, _ENCRYPTED_TABLE, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # update statically
    await pg_update_static(test_cur, _ENCRYPTED_TABLE, static_value={"secret_int32": 42})
    target_rows = [{**row, "secret_int32": 42} for row in target_rows]
    db_rows = await pg_select(test_cur, _ENCRYPTED_TABLE, order_by=sql.SQL("id"))
    assert db_rows == target_rows

    # delete
    await pg_delete(test_cur, _ENCRYPTED_TABLE, where=sql.SQL("id > 3"))
    target_rows = target_rows[:3]
    db_rows = await pg_select(test_cur, _ENCRYPTED_TABLE, order_by=sql.SQL("id"))
    assert db_rows == target_rows

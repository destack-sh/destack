import random
import string
from typing import Callable, Mapping

import psycopg
from psycopg.types.json import Jsonb
import pytest

from bench.language import PrimitiveType
from bench.sql.core import Table, Column
from bench.sql.engine import RowIn, pg_insert, pg_select
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
    initial_rows: list[RowIn] = []
    for id in range(5):
        row = {f"secret_{t.name.lower()}": COLUMN_VALUE_GENERATORS[t]() for t in _ENCRYPTED_TYPES}
        row["id"] = id
        initial_rows.append(row)

    await pg_insert(test_cur, _ENCRYPTED_TABLE, initial_rows)

    rows = await pg_select(test_cur, _ENCRYPTED_TABLE)

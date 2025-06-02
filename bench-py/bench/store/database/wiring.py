from collections.abc import Sequence
from typing import Any

import asyncpg

from bench.language import Value


def pack_node_value_to_row(value: Value) -> Sequence[Any]:
    """Pack a Node Value into an asyncpg row."""
    raise NotImplementedError


def unpack_row_to_node_value(row: asyncpg.Record) -> Value:
    """Unpack an asyncpg row into a Node Value."""
    raise NotImplementedError

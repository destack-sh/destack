from collections.abc import Sequence

import asyncpg
import structlog
from opentelemetry import trace

from bench.language import Change, Edit, EditType, NodeReference, NodeType

from .core import DatabaseContext, DatabaseTable
from .map import get_table_name
from .wiring import pack_node_value_to_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


async def _get_table(ctx: DatabaseContext, node_ptr: NodeReference) -> DatabaseTable:
    table_name = get_table_name(node_ptr)
    table = ctx.tables_by_name.get(table_name)
    if table is None:
        if node_ptr.node_type == NodeType.CUSTOM_NODE_INSTANCE:
            # nocheckin: create custom Tables/Columns just in time
            raise NotImplementedError(f"no table for {node_ptr!r} in {ctx!r}")
        else:
            raise RuntimeError(f"no table for {node_ptr!r} in {ctx!r}")
    return table


async def execute_change(
    conn: asyncpg.Connection, ctx: DatabaseContext, change: Change
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """Execute the Change."""
    assert change.edits, f"no Edits in {change!r}"

    cascaded_edits: list[Edit] = []

    current_table = await _get_table(ctx, change.edits[0].node_ptr)
    current_edit_type = change.edits[0].type
    current_batch: list[Edit] = []

    for edit in change.edits:
        edit_table = await _get_table(ctx, edit.node_ptr)
        if edit_table is not current_table or edit.type != current_edit_type:
            _, batch_cascaded_edits = await execute_edits(
                conn, ctx, current_table, current_edit_type, current_batch
            )
            cascaded_edits.extend(batch_cascaded_edits)
            current_table = edit_table
            current_edit_type = edit.type
            current_batch = []
        current_batch.append(edit)

    if current_batch:
        _, batch_cascaded_edits = await execute_edits(
            conn, ctx, current_table, current_edit_type, current_batch
        )
        cascaded_edits.extend(batch_cascaded_edits)

    return change.edits, cascaded_edits


async def execute_edits(
    conn: asyncpg.Connection,
    ctx: DatabaseContext,
    table: DatabaseTable,
    edit_type: EditType,
    edits: list[Edit],
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """
    Execute the Edits for a table.
    Returns the Edits and any cascaded Edits.
    """
    if edit_type == EditType.CREATE:
        stmt_parts = [
            f"INSERT INTO {table.name} ({', '.join(col.name for col in table.columns)})",
            f"VALUES ({', '.join(f'${i}' for i in range(len(edits)))});",
        ]
        stmt = " ".join(stmt_parts)
        values_packed = []
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            values_packed.append(pack_node_value_to_row(edit.value))
        logger.debug("database.execute_edits", stmt=stmt, span="current")
        await conn.execute(stmt, *values_packed)
        return edits, ()
    else:
        raise NotImplementedError(f"no implementation for {edit_type!r}")

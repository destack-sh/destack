import uuid
from collections import defaultdict
from collections.abc import Sequence
from typing import Any, assert_never

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
            _, batch_cascaded_edits = await _execute_edits(
                conn=conn,
                ctx=ctx,
                change=change,
                table=current_table,
                edit_type=current_edit_type,
                edits=current_batch,
            )
            cascaded_edits.extend(batch_cascaded_edits)
            current_table = edit_table
            current_edit_type = edit.type
            current_batch = []
        current_batch.append(edit)

    if current_batch:
        _, batch_cascaded_edits = await _execute_edits(
            conn=conn,
            ctx=ctx,
            change=change,
            table=current_table,
            edit_type=current_edit_type,
            edits=current_batch,
        )
        cascaded_edits.extend(batch_cascaded_edits)

    return change.edits, cascaded_edits


async def _get_cascaded_node_ptrs(
    conn: asyncpg.Connection,
    ctx: DatabaseContext,
    change: Change,
    table: DatabaseTable,
    edit_type: EditType,
) -> Sequence[NodeReference]:
    return ()


async def _execute_edits(
    conn: asyncpg.Connection,
    ctx: DatabaseContext,
    change: Change,
    table: DatabaseTable,
    edit_type: EditType,
    edits: list[Edit],
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """
    Execute the Edits for a table.
    Returns the Edits and any cascaded Edits.
    """

    # create/upsert
    if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
        stmt = f"""\
INSERT INTO {table.name} ({", ".join(col.name for col in table.columns)})
VALUES ({", ".join(f"${i + 1}" for i in range(len(table.columns)))})
"""
        if edit_type == EditType.UPSERT:
            override_columns = tuple(
                col
                for col in table.columns
                if not col.is_primary_key and not col.name.startswith("created_")
            )
            stmt += f"""\
ON CONFLICT DO UPDATE
SET {", ".join(f"{col.name} = EXCLUDED.{col.name}" for col in override_columns)}
"""
        stmt += ";"
        values_packed: list[Sequence[Any]] = []
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            values_packed.append(pack_node_value_to_row(table, edit.value))
        logger.debug("database.execute_edits", stmt=stmt, span="current")
        await conn.executemany(stmt, values_packed)
        return edits, ()

    # update
    elif edit_type == EditType.UPDATE:
        # TODO :Performance: batch database update edits somehow
        for edit in edits:
            prop = edit.prop
            assert prop is not None, f"no prop for {edit!r}"
            raise NotImplementedError(f"no implementation for {edit_type!r}")
        return edits, ()

    # move
    elif edit_type == EditType.MOVE:
        raise NotImplementedError(f"no implementation for {edit_type!r}")

    # archive/unarchive/delete/restore
    elif edit_type in (
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.DELETE,
        EditType.RESTORE,
    ):
        # cascade
        cascaded_node_ptrs = await _get_cascaded_node_ptrs(
            conn=conn,
            ctx=ctx,
            change=change,
            table=table,
            edit_type=edit_type,
        )
        cascaded_edits = tuple(
            Edit(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # update
        if edit_type == EditType.ARCHIVE:
            update_stmt = "SET archived_at = $2"
        elif edit_type == EditType.UNARCHIVE:
            update_stmt = "SET archived_at = NULL"
        elif edit_type == EditType.DELETE:
            update_stmt = "SET deleted_at = $2"
        elif edit_type == EditType.RESTORE:
            update_stmt = "SET deleted_at = NULL"
        else:
            assert_never(edit_type)
        edited_node_ptrs_by_table: dict[str, list[uuid.UUID]] = defaultdict(list)
        for node_ptr in cascaded_node_ptrs:
            table_name = get_table_name(node_ptr)
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        at_packed = uuid.UUID(str(change.created_at))
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            stmt = f"""\
UPDATE {table_name}
SET {update_stmt}
WHERE id = $1
"""
            await conn.executemany(stmt, [(node_id, at_packed) for node_id in table_node_ids])

        return edits, cascaded_edits

    # erase
    elif edit_type == EditType.ERASE:
        # cascade
        cascaded_node_ptrs = await _get_cascaded_node_ptrs(
            conn=conn,
            ctx=ctx,
            change=change,
            table=table,
            edit_type=edit_type,
        )
        cascaded_edits = tuple(
            Edit(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # delete
        edited_node_ptrs_by_table: dict[str, list[uuid.UUID]] = defaultdict(list)
        for node_ptr in cascaded_node_ptrs:
            table_name = get_table_name(node_ptr)
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            stmt = f"""\
DELETE FROM {table_name}
WHERE id = $1
"""
            await conn.executemany(stmt, table_node_ids)

        return edits, cascaded_edits

    else:
        assert_never(edit_type)

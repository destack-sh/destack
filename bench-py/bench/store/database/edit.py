import uuid
from collections import defaultdict
from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import structlog
from opentelemetry import trace

from bench.language import Change, Edit, EditOperation, EditType, NodeReference, ScalarType
from bench.language.registry import NODE_CLASS_BY_TYPE

from .core import DatabaseContext, DatabaseTable
from .wiring import pack_column_wide, pack_node_value_to_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


@tracer.start_as_current_span("database.execute_change")
async def execute_change(
    conn: asyncpg.Connection, context: DatabaseContext, change: Change
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """Execute the Change."""
    assert change.edits, f"no Edits in {change!r}"

    cascaded_edits: list[Edit] = []

    current_table = context.get_table(change.edits[0].node_ptr)
    current_edit_type = change.edits[0].type
    current_batch: list[Edit] = []
    for edit in change.edits:
        edit_table = context.get_table(edit.node_ptr)
        if edit_table is not current_table or edit.type != current_edit_type:
            _, batch_cascaded_edits = await _execute_data_edit(
                conn=conn,
                context=context,
                change=change,
                table=current_table,
                edit_type=current_edit_type,
                edits=current_batch,
            )
            cascaded_edits.extend(batch_cascaded_edits)
            schema_edits = context.apply(tuple(current_batch) + tuple(batch_cascaded_edits))
            if schema_edits:
                await _execute_schema_edits(conn=conn, context=context, edits=schema_edits)

            current_table = edit_table
            current_edit_type = edit.type
            current_batch = []

        current_batch.append(edit)

    if current_batch:
        _, batch_cascaded_edits = await _execute_data_edit(
            conn=conn,
            context=context,
            change=change,
            table=current_table,
            edit_type=current_edit_type,
            edits=current_batch,
        )
        cascaded_edits.extend(batch_cascaded_edits)
        schema_edits = context.apply(tuple(current_batch) + tuple(batch_cascaded_edits))
        if schema_edits:
            await _execute_schema_edits(conn=conn, context=context, edits=schema_edits)

    logger.debug("database.execute_change", change=change, span="current")
    return change.edits, cascaded_edits


@tracer.start_as_current_span("database.cascade_nodes")
async def _cascade_nodes(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    change: Change,
    table: DatabaseTable,
    node_ptrs: Sequence[NodeReference],
) -> Sequence[NodeReference]:
    """Get the cascaded Nodes for an Edit."""
    raise NotImplementedError


@tracer.start_as_current_span("database.execute_schema_edits")
async def _execute_schema_edits(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    edits: Sequence[Edit],
) -> None:
    """Execute the Edits against the schema (schema only, no data)."""
    raise NotImplementedError


@tracer.start_as_current_span("database.execute_data_edit")
async def _execute_data_edit(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    change: Change,
    table: DatabaseTable,
    edit_type: EditType,
    edits: list[Edit],
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """
    Execute the Edits to the data (data only, no schema).
    Returns the Edits and any cascaded Edits.
    TODO :Performance: batch database update & move edits
    """

    node_type = table.node_type
    node_cls = NODE_CLASS_BY_TYPE[node_type]

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
ON CONFLICT (id) DO UPDATE
SET {", ".join(f"{col.name} = EXCLUDED.{col.name}" for col in override_columns)}
"""
        stmt += ";"
        values_packed: list[Sequence[Any]] = []
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            row_values_packed = pack_node_value_to_row(table, edit.value)
            values_packed.append(row_values_packed)
        await conn.executemany(stmt, values_packed)
        logger.debug(f"database.{edit_type.name.lower()}", change=change, stmt=stmt, span="current")
        return edits, ()

    # update
    elif edit_type == EditType.UPDATE:
        for edit in edits:
            prop = edit.prop
            assert prop is not None, f"no prop for {edit!r}"
            assert edit.value is not None, f"no value for {edit!r}"
            assert edit.operation in (EditOperation.SET, EditOperation.CLEAR), (
                f"unsupported operation: {edit!r}"
            )
            update: dict[str, Any] = {}
            pack_column_wide(prop.type, edit.value.value, table, prop.name, update)
            stmt = f"""\
UPDATE {table.name}
SET {", ".join(f"{key} = ${i + 1}" for i, key in enumerate(update.keys()))}
WHERE id = ${len(update) + 1}
"""
            await conn.execute(stmt, *update.values(), edit.node_ptr.id)
            logger.debug("database.update", change=change, stmt=stmt, span="current")

        return edits, ()

    # move
    elif edit_type == EditType.MOVE:
        parent_prop = node_cls.__parent_property__
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            assert edit.value.type.scalar_type == ScalarType.NODE_REFERENCE, (
                f"unexpected value: {edit!r}"
            )
            update: dict[str, Any] = {}
            pack_column_wide(parent_prop.type, edit.value.value, table, parent_prop.name, update)
            stmt = f"""\
UPDATE {table.name}
SET {", ".join(f"{key} = ${i + 1}" for i, key in enumerate(update.keys()))}
WHERE id = ${len(update) + 1}
"""
            await conn.execute(stmt, *update.values(), edit.node_ptr.id)
            logger.debug("database.move", change=change, stmt=stmt, span="current")

        return edits, ()

    # archive/unarchive/delete/restore
    elif edit_type in (
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.DELETE,
        EditType.RESTORE,
    ):
        # cascade
        cascaded_node_ptrs = await _cascade_nodes(
            conn=conn,
            context=context,
            change=change,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
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
            table_name = context.get_table(node_ptr).name
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
            logger.debug(
                f"database.{edit_type.name.lower()}",
                change=change,
                cascaded_edits=len(cascaded_edits),
                stmt=stmt,
                span="current",
            )

        return edits, cascaded_edits

    # erase
    elif edit_type == EditType.ERASE:
        # cascade
        cascaded_node_ptrs = await _cascade_nodes(
            conn=conn,
            context=context,
            change=change,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
        )
        cascaded_edits = tuple(
            Edit(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # delete
        edited_node_ptrs_by_table: dict[str, list[uuid.UUID]] = defaultdict(list)
        for node_ptr in cascaded_node_ptrs:
            table_name = context.get_table(node_ptr).name
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            stmt = f"""\
DELETE FROM {table_name}
WHERE id = $1
"""
            await conn.executemany(stmt, table_node_ids)
            logger.debug(
                "database.erase",
                change=change,
                cascaded_edits=len(cascaded_edits),
                stmt=stmt,
                span="current",
            )

        return edits, cascaded_edits

    else:
        assert_never(edit_type)

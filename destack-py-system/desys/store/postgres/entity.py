import uuid
from collections import OrderedDict, defaultdict
from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import structlog
from opentelemetry import trace

from destack.language import (
    CASCADING_EDIT_TYPES,
    EditEvent,
    EditOperation,
    EditType,
    NodeReference,
    ScalarType,
)
from destack.language.registry import NODE_CLASS_BY_TYPE

from .core import PostgresContext, PostgresTable
from .wiring import pack_column_wide, pack_node_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


@tracer.start_as_current_span("postgres.execute_edits")
async def execute_edits(
    conn: asyncpg.Connection, context: PostgresContext, events: Sequence[EditEvent]
) -> tuple[Sequence[EditEvent], Sequence[EditEvent]]:
    """Execute the Events."""
    assert events, f"no Events in {events!r}"

    edits = _optimize_edits(context, events)
    cascaded_edits: list[EditEvent] = []
    applied_edits: list[EditEvent] = []

    current_table = context.get(edits[0].node_ptr)
    current_edit_type = edits[0].type
    current_batch: list[EditEvent] = []
    for edit in edits:
        edit_table = context.get(edit.node_ptr)
        if edit_table is not current_table or edit.type != current_edit_type:
            batch_applied_edits, batch_cascaded_edits = await _execute_data_edit(
                conn=conn,
                context=context,
                table=current_table,
                edit_type=current_edit_type,
                edits=current_batch,
            )
            applied_edits.extend(batch_applied_edits)
            cascaded_edits.extend(batch_cascaded_edits)

            current_table = edit_table
            current_edit_type = edit.type
            current_batch = []

        current_batch.append(edit)

    if current_batch:
        batch_applied_edits, batch_cascaded_edits = await _execute_data_edit(
            conn=conn,
            context=context,
            table=current_table,
            edit_type=current_edit_type,
            edits=current_batch,
        )
        applied_edits.extend(batch_applied_edits)
        cascaded_edits.extend(batch_cascaded_edits)

    logger.trace("postgres.execute_edits", events=len(events), span="current")
    return applied_edits, cascaded_edits


@tracer.start_as_current_span("postgres.optimize_edits")
def _optimize_edits(context: PostgresContext, edits: Sequence[EditEvent]) -> list[EditEvent]:
    """
    Optimize the Change/Edits *while retaining semantic equivalence*.
    Reorder and batch non-interfering Edits to minimize roundtrips.
    """

    optimized_edits: list[EditEvent] = []
    buffer: list[EditEvent] = []

    def flush():
        if not buffer:
            return
        grouped: OrderedDict[tuple[str, EditType], list[EditEvent]] = OrderedDict()
        for e in buffer:
            table = context.get(e.node_ptr)
            key = (table.name, e.type)
            if key not in grouped:
                grouped[key] = []
            grouped[key].append(e)
        for batch in grouped.values():
            optimized_edits.extend(batch)  # contiguously batched
        buffer.clear()

    for edit in edits:
        if edit.type in CASCADING_EDIT_TYPES:
            flush()  # close current segment
            optimized_edits.append(edit)  # keep position
        else:
            buffer.append(edit)  # postpone

    flush()  # trailing segment
    return optimized_edits


@tracer.start_as_current_span("postgres.execute_cascade")
async def _execute_cascade(
    conn: asyncpg.Connection,
    context: PostgresContext,
    table: PostgresTable,
    node_ptrs: Sequence[NodeReference],
) -> Sequence[NodeReference]:
    """Get the cascaded Nodes for an Edit."""
    raise NotImplementedError


@tracer.start_as_current_span("postgres.execute_data_edit")
async def _execute_data_edit(
    conn: asyncpg.Connection,
    context: PostgresContext,
    table: PostgresTable,
    edit_type: EditType,
    edits: Sequence[EditEvent],
) -> tuple[Sequence[EditEvent], Sequence[EditEvent]]:
    """
    Execute the Edits to the data (data only, no schema).
    Returns the Edits and any cascaded Edits.
    """

    node_type = table.node_type
    node_cls = NODE_CLASS_BY_TYPE[node_type]

    # create/upsert
    if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
        stmt = f"""\
INSERT INTO {table.name} ({", ".join(f'"{col.name}"' for col in table.columns)})
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
SET {", ".join(f'"{col.name}" = EXCLUDED."{col.name}"' for col in override_columns)}
"""
        stmt += ";"
        values_packed: list[Sequence[Any]] = []
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            row_values_packed = pack_node_row(table, edit.value)
            values_packed.append(row_values_packed)
        await conn.executemany(stmt, values_packed)
        logger.trace(
            f"postgres.{edit_type.name.lower()}",
            edits=len(edits),
            stmt=stmt,
            span="current",
        )
        return edits, ()

    # update
    elif edit_type == EditType.UPDATE:
        # gather all db-columns possibly touched in this batch
        all_updated_columns: set[str] = set()
        for edit in edits:
            assert edit.attribute is not None, f"no prop_ptr for {edit!r}"
            prop = edit.attribute.resolve()
            assert prop is not None, f"no prop for {edit!r}"
            update: dict[str, Any] = {}
            pack_column_wide(prop, None, table, prop.name, update)
            all_updated_columns.update(update.keys())

        updated_column_names = list(all_updated_columns)
        updated_column_names.sort()
        updated_column_idx = {column: i for i, column in enumerate(updated_column_names)}

        # build "CASE WHEN changed THEN value ELSE col END" for every column
        set_clauses: list[str] = []
        param_i = 1
        for column in updated_column_names:
            value_param = f"${param_i}"  # new value (may be NULL)
            changed_param = f"${param_i + 1}"  # whether the value changed (bool)
            set_clauses.append(
                f'"{column}" = CASE WHEN {changed_param} THEN {value_param} ELSE "{column}" END'
            )
            param_i += 2
        stmt = f"""\
UPDATE {table.name}
SET {", ".join(set_clauses)}
WHERE id = ${param_i}
"""

        # prepare parameters row-by-row
        values_packed: list[Sequence[Any]] = []
        for edit in edits:
            assert edit.attribute is not None, f"no prop_ptr for {edit!r}"
            prop = edit.attribute.resolve()
            assert prop is not None, f"no prop for {edit!r}"
            if edit.operation == EditOperation.SET:
                assert edit.value is not None, f"no value for {edit!r}"
                value_packed = edit.value.value
            elif edit.operation == EditOperation.CLEAR:
                value_packed = None
            else:
                raise RuntimeError(f"unsupported operation: {edit!r}")

            update_row: list[Any] = [None, False] * len(updated_column_names)  # default: keep
            update: dict[str, Any] = {}
            pack_column_wide(prop, value_packed, table, prop.name, update)
            for column, value in update.items():  # mark touched cols
                i = updated_column_idx[column] * 2
                update_row[i] = value
                update_row[i + 1] = True
            update_row.append(edit.node_ptr.id)  # WHERE id = …
            values_packed.append(tuple(update_row))

        await conn.executemany(stmt, values_packed)
        logger.trace(
            "postgres.update",
            edits=len(edits),
            stmt=stmt,
            span="current",
        )
        return edits, ()

    # move
    elif edit_type == EditType.MOVE:
        # prepare statement
        parent_prop = node_cls.__parent_property__
        parent_prop_type = parent_prop.to_type()
        update_template: dict[str, Any] = {}
        pack_column_wide(parent_prop_type, None, table, parent_prop.name, update_template)
        stmt = f"""\
UPDATE {table.name}
SET {", ".join(f'"{key}" = ${i + 1}' for i, key in enumerate(update_template.keys()))}
WHERE id = ${len(update_template) + 1}
"""
        # prepare values
        values_packed: list[Sequence[Any]] = []
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            assert edit.value.type.scalar_type == ScalarType.NODE_REFERENCE, (
                f"unexpected value: {edit!r}"
            )
            update: dict[str, Any] = {}
            pack_column_wide(parent_prop_type, edit.value.value, table, parent_prop.name, update)
            row_values = (*update.values(), edit.node_ptr.id)
            values_packed.append(row_values)
        await conn.executemany(stmt, values_packed)
        logger.trace(
            "postgres.move",
            edits=len(edits),
            stmt=stmt,
            span="current",
        )

        return edits, ()

    # archive/unarchive/delete/restore
    elif edit_type in (
        EditType.ARCHIVE,
        EditType.UNARCHIVE,
        EditType.DELETE,
        EditType.RESTORE,
    ):
        # cascade
        cascaded_node_ptrs = await _execute_cascade(
            conn=conn,
            context=context,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
        )
        cascaded_edits = tuple(
            EditEvent(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
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
            table_name = context.get(node_ptr).name
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        at_packed = uuid.UUID(str(edits[0].created_at))
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            stmt = f"""\
UPDATE {table_name}
SET {update_stmt}
WHERE id = $1
"""
            await conn.executemany(stmt, [(node_id, at_packed) for node_id in table_node_ids])
            logger.trace(
                f"postgres.{edit_type.name.lower()}",
                edits=len(edits),
                cascaded_edits=len(cascaded_edits),
                stmt=stmt,
                span="current",
            )

        return edits, cascaded_edits

    # erase
    elif edit_type == EditType.ERASE:
        # cascade
        cascaded_node_ptrs = await _execute_cascade(
            conn=conn,
            context=context,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
        )
        cascaded_edits = tuple(
            EditEvent(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # delete
        edited_node_ptrs_by_table: dict[str, list[uuid.UUID]] = defaultdict(list)
        for node_ptr in cascaded_node_ptrs:
            table_name = context.get(node_ptr).name
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            stmt = f"""\
DELETE FROM {table_name}
WHERE id = $1
"""
            await conn.executemany(stmt, table_node_ids)
            logger.trace(
                "postgres.erase",
                edits=len(edits),
                cascaded_edits=len(cascaded_edits),
                stmt=stmt,
                span="current",
            )

        return edits, cascaded_edits

    else:
        assert_never(edit_type)

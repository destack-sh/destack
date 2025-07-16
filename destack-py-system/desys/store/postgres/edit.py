import uuid
from collections import OrderedDict, defaultdict
from collections.abc import Sequence
from datetime import UTC, datetime
from itertools import chain
from typing import Any, assert_never

import asyncpg
import structlog
from opentelemetry import trace

from destack.language import (
    CASCADING_EDIT_TYPES,
    Condition,
    EdgeDirection,
    EditEvent,
    EditOperation,
    EditType,
    Entity,
    IsDeletable,
    NodeDefinitionReference,
    NodeReference,
    ScalarType,
)
from destack.language.registry import NODE_CLASS_BY_TYPE

from .core import PostgresContext
from .wiring import pack_column_wide, pack_node_row

MAX_RECURSION_DEPTH = 100

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)


@tracer.start_as_current_span("postgres.execute_edits")
async def execute_edits(
    conn: asyncpg.Connection, context: PostgresContext, events: Sequence[EditEvent]
) -> tuple[Sequence[EditEvent], Sequence[EditEvent]]:
    """Execute the Events."""
    if not events:
        return (), ()

    edits = _optimize_edits(context, events)
    cascaded_edits: list[EditEvent] = []
    applied_edits: list[EditEvent] = []

    current_definition = NodeDefinitionReference.of(edits[0].node_ptr)
    current_edit_type = edits[0].type
    current_batch: list[EditEvent] = []
    for edit in edits:
        edit_definition = NodeDefinitionReference.of(edit.node_ptr)
        if (
            edit_definition.node_type != current_definition.node_type
            or edit.type != current_edit_type
        ):
            batch_applied_edits, batch_cascaded_edits = await _execute_edit(
                conn=conn,
                context=context,
                definition=current_definition,
                edit_type=current_edit_type,
                edits=current_batch,
            )
            applied_edits.extend(batch_applied_edits)
            cascaded_edits.extend(batch_cascaded_edits)
            current_definition = edit_definition
            current_edit_type = edit.type
            current_batch = []

        current_batch.append(edit)

    if current_batch:
        batch_applied_edits, batch_cascaded_edits = await _execute_edit(
            conn=conn,
            context=context,
            definition=current_definition,
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
            definition = NodeDefinitionReference.of(e.node_ptr)
            table = context.get(definition)
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


async def _execute_cascade(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    node_ptrs: Sequence[NodeReference],
    where: Condition | None,
) -> tuple[Sequence[NodeReference], dict[uuid.UUID, uuid.UUID]]:
    """Get the cascaded Nodes for an Edit."""
    from .query import _walk_node

    child_ptrs = await _walk_node(
        conn=conn,
        context=context,
        definition=definition,
        nodes_ptr=node_ptrs,
        direction=EdgeDirection.CHILD,
        depth=MAX_RECURSION_DEPTH,
        where=None,
    )
    return child_ptrs


@tracer.start_as_current_span("postgres.execute_edit")
async def _execute_edit(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    edit_type: EditType,
    edits: Sequence[EditEvent],
) -> tuple[Sequence[EditEvent], Sequence[EditEvent]]:
    """
    Execute the Edits to the data (data only, no schema).
    Returns the Edits and any cascaded Edits.
    """

    table = context.get(definition)
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
            assert edit.property_id is not None, f"no property_id for {edit!r}"
            prop = definition.resolve_property(edit.property_id)
            assert prop is not None, f"no property for {edit!r}"
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
            assert edit.property_id is not None, f"no property_id for {edit!r}"
            prop = definition.resolve_property(edit.property_id)
            assert prop is not None, f"no property for {edit!r}"
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
        assert issubclass(node_cls, Entity), (
            f"cannot move non-Entity {node_cls.__name__} in {edits!r}"
        )
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
            if edit.value is not None and edit.value.value is not None:
                assert edit.value.type.scalar_type == ScalarType.NODE_REFERENCE, (
                    f"unexpected value: {edit!r}"
                )
                parent_ptr_value = edit.value.value
            else:
                parent_ptr_value = None
            update: dict[str, Any] = {}
            pack_column_wide(parent_prop_type, parent_ptr_value, table, parent_prop.name, update)
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
        EditType.DELETE,
        EditType.RESTORE,
    ):
        # cascade
        nodes_ptr = tuple(edit.node_ptr for edit in edits)
        edited_at_by_node_id: dict[uuid.UUID, datetime] = {
            uuid.UUID(str(edit.node_ptr.id)): edit.created_at.astimezone(UTC).replace(tzinfo=None)
            for edit in edits
        }
        where: Condition | None = None
        if edit_type == EditType.RESTORE:
            # restrict to nodes with same deleted_at/archived_at
            root_dts: set[datetime] = set()
            root_stmt = f"SELECT id, deleted_at FROM {table.name} WHERE id IN ($1)"
            root_rows = await conn.fetch(root_stmt, *(n.id for n in nodes_ptr))
            for row in root_rows:
                if deleted_at := row["deleted_at"]:
                    root_dts.add(deleted_at.replace(tzinfo=UTC))
            where = IsDeletable.property("deleted_at").in_(*root_dts)

        cascaded_node_ptrs, source_id_by_node_id = await _execute_cascade(
            conn=conn,
            context=context,
            definition=definition,
            node_ptrs=nodes_ptr,
            where=where,
        )
        cascaded_edits = tuple(
            EditEvent(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # update
        edited_node_ptrs_by_table: dict[str, list[uuid.UUID]] = defaultdict(list)
        for node_ptr in chain(nodes_ptr, cascaded_node_ptrs):
            table_name = context.get(node_ptr).name
            node_id_packed = uuid.UUID(str(node_ptr.id))
            edited_node_ptrs_by_table.setdefault(table_name, []).append(node_id_packed)
        for table_name, table_node_ids in edited_node_ptrs_by_table.items():
            arguments: list[Any] = []
            if edit_type == EditType.DELETE:
                update_stmt = "deleted_at = $2"
                for node_id in table_node_ids:
                    edited_at = edited_at_by_node_id[source_id_by_node_id[node_id]]
                    arguments.append((node_id, edited_at))
            elif edit_type == EditType.RESTORE:
                update_stmt = "deleted_at = NULL"
                for node_id in table_node_ids:
                    arguments.append((node_id,))
            else:
                assert_never(edit_type)
            stmt = f"""\
UPDATE {table_name}
SET {update_stmt}
WHERE id = $1
"""
            await conn.executemany(stmt, arguments)

        logger.trace(
            f"postgres.{edit_type.name.lower()}",
            edits=len(edits),
            cascaded_edits=len(cascaded_edits),
            span="current",
        )
        return edits, cascaded_edits

    else:
        assert_never(edit_type)

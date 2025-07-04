from collections import OrderedDict
from collections.abc import Sequence
from typing import assert_never

import structlog
from opentelemetry import trace

from destack.language import (
    CASCADING_EDIT_TYPES,
    Change,
    Edit,
    EditOperation,
    EditType,
    IsArchivable,
    IsDeletable,
    Node,
    NodeReference,
    NodeType,
)

from .core import MemoryContext, MemoryDatabase, MemoryTable, VersionedNodeKey
from .wiring import pack_node_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

NODE_PARENT_KEY = str(Node.property("parent").id)

ARCHIVED_AT_KEY = str(IsArchivable.property("archived_at").id)
DELETED_AT_KEY = str(IsDeletable.property("deleted_at").id)


@tracer.start_as_current_span("memory.execute_change")
def execute_change(
    database: MemoryDatabase, context: MemoryContext, change: Change
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """Execute the Change."""
    assert change.edits, f"no Edits in {change!r}"

    edits = _optimize_change(context, change.edits)
    cascaded_edits: list[Edit] = []
    applied_edits: list[Edit] = []

    current_table = context.get(change.edits[0].node_ptr)
    current_edit_type = change.edits[0].type
    current_batch: list[Edit] = []
    for edit in edits:
        edit_table = context.get(edit.node_ptr)
        if edit_table is not current_table or edit.type != current_edit_type:
            batch_applied_edits, batch_cascaded_edits = _execute_data_edit(
                database, context, change, current_table, current_edit_type, current_batch
            )
            current_table = edit_table
            current_edit_type = edit.type
            current_batch = []

        current_batch.append(edit)

    if current_batch:
        batch_applied_edits, batch_cascaded_edits = _execute_data_edit(
            database, context, change, current_table, current_edit_type, current_batch
        )
        applied_edits.extend(batch_applied_edits)
        cascaded_edits.extend(batch_cascaded_edits)

    logger.trace("memory.execute_change", change=change, span="current")
    return applied_edits, cascaded_edits


@tracer.start_as_current_span("memory.optimize_change")
def _optimize_change(context: MemoryContext, edits: Sequence[Edit]) -> list[Edit]:
    """
    Optimize the Change/Edits *while retaining semantic equivalence*.
    Reorder and batch non-interfering Edits to minimize roundtrips.
    """

    optimized_edits: list[Edit] = []
    buffer: list[Edit] = []

    def flush():
        if not buffer:
            return
        grouped: OrderedDict[tuple[NodeType, EditType], list[Edit]] = OrderedDict()
        for e in buffer:
            key = (e.node_ptr.type, e.type)
            if key not in grouped:
                grouped[key] = []
            grouped[key].append(e)  # contiguously batched
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


@tracer.start_as_current_span("memory.execute_cascade")
def _execute_cascade(
    database: MemoryDatabase,
    context: MemoryContext,
    table: MemoryTable,
    node_ptrs: Sequence[NodeReference],
) -> Sequence[NodeReference]:
    """Get the cascaded Nodes for an Edit."""
    raise NotImplementedError


@tracer.start_as_current_span("memory.execute_data_edit")
def _execute_data_edit(
    database: MemoryDatabase,
    context: MemoryContext,
    change: Change,
    table: MemoryTable,
    edit_type: EditType,
    edits: Sequence[Edit],
) -> tuple[Sequence[Edit], Sequence[Edit]]:
    """
    Execute the Edits to the data (data only, no schema).
    Returns the applied Edits and any cascaded Edits.
    """
    from destack.language import ScalarType

    # create/upsert
    if edit_type == EditType.CREATE or edit_type == EditType.UPSERT:
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            snapshot_id = edit.snapshot_ptr.id if edit.snapshot_ptr is not None else None
            node_key = VersionedNodeKey(id=edit.node_ptr.id, snapshot_id=snapshot_id)
            if edit_type == EditType.UPSERT or node_key not in table.rows:
                row = pack_node_row(table, edit.value)
                table.rows[node_key] = row
                table.rows_by_snapshot[snapshot_id][node_key.id] = row
                if row.parent_ptr is not None:
                    parent_table = context.get(row.parent_ptr)
                    parent_key = VersionedNodeKey(id=row.parent_ptr.id, snapshot_id=snapshot_id)
                    parent_table.rows_by_parent[parent_key].append(row)

        logger.trace(
            f"memory.{edit_type.name.lower()}",
            change=change,
            edits=len(edits),
            span="current",
        )
        return edits, ()

    # update
    elif edit_type == EditType.UPDATE:
        for edit in edits:
            snapshot_id = edit.snapshot_ptr.id if edit.snapshot_ptr is not None else None
            assert edit.attribute is not None, f"no prop_ptr for {edit!r}"
            prop = edit.attribute.resolve()
            assert prop is not None, f"no prop for {edit!r}"
            node_key = VersionedNodeKey(id=edit.node_ptr.id, snapshot_id=snapshot_id)
            if row := table.rows.get(node_key):
                if edit.operation == EditOperation.SET:
                    assert edit.value is not None, f"no value for {edit!r}"
                    row.value[str(prop.id)] = edit.value.value
                elif edit.operation == EditOperation.CLEAR:
                    row.value.pop(str(prop.id), None)
                else:
                    raise RuntimeError(f"unsupported operation: {edit!r}")

        logger.trace(
            f"memory.{edit_type.name.lower()}",
            change=change,
            edits=len(edits),
            span="current",
        )
        return edits, ()

    # move
    elif edit_type == EditType.MOVE:
        for edit in edits:
            assert edit.value is not None, f"no value for {edit!r}"
            assert edit.value.type.scalar_type == ScalarType.NODE_REFERENCE, (
                f"unexpected value: {edit!r}"
            )
            snapshot_id = edit.snapshot_ptr.id if edit.snapshot_ptr is not None else None
            node_key = VersionedNodeKey(id=edit.node_ptr.id, snapshot_id=snapshot_id)
            if row := table.rows.get(node_key):
                if row.parent_ptr is not None:
                    parent_table = context.get(row.parent_ptr)
                    parent_key = VersionedNodeKey(id=row.parent_ptr.id, snapshot_id=snapshot_id)
                    parent_table.rows_by_parent[parent_key].remove(row)
                row.parent_ptr = edit.value.value
                row.value[NODE_PARENT_KEY] = edit.value.value
                if row.parent_ptr is not None:
                    parent_table = context.get(row.parent_ptr)
                    parent_key = VersionedNodeKey(id=row.parent_ptr.id, snapshot_id=snapshot_id)
                    parent_table.rows_by_parent[parent_key].append(row)

        logger.trace(
            f"memory.{edit_type.name.lower()}",
            change=change,
            edits=len(edits),
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
        cascaded_node_ptrs = _execute_cascade(
            database=database,
            context=context,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
        )
        cascaded_edits = tuple(
            Edit(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # update timestamps
        for node_ptr in cascaded_node_ptrs:
            node_table = context.get(node_ptr)
            snapshot_id = node_ptr.snapshot_id if node_ptr.snapshot_id is not None else None
            node_key = VersionedNodeKey(id=node_ptr.id, snapshot_id=snapshot_id)
            if row := node_table.rows.get(node_key):
                if edit_type == EditType.ARCHIVE:
                    row.value[ARCHIVED_AT_KEY] = change.created_at
                elif edit_type == EditType.UNARCHIVE:
                    row.value.pop(ARCHIVED_AT_KEY, None)
                elif edit_type == EditType.DELETE:
                    row.value[DELETED_AT_KEY] = change.created_at
                elif edit_type == EditType.RESTORE:
                    row.value.pop(DELETED_AT_KEY, None)
                else:
                    assert_never(edit_type)

        logger.trace(
            f"memory.{edit_type.name.lower()}",
            change=change,
            edits=len(edits),
            span="current",
        )
        return edits, cascaded_edits

    # erase
    elif edit_type == EditType.ERASE:
        # cascade
        cascaded_node_ptrs = _execute_cascade(
            database=database,
            context=context,
            table=table,
            node_ptrs=tuple(edit.node_ptr for edit in edits),
        )
        cascaded_edits = tuple(
            Edit(type=edit_type, node_ptr=node_ptr) for node_ptr in cascaded_node_ptrs
        )

        # delete rows
        for node_ptr in cascaded_node_ptrs:
            node_table = context.get(node_ptr)
            snapshot_id = node_ptr.snapshot_id if node_ptr.snapshot_id is not None else None
            node_key = VersionedNodeKey(id=node_ptr.id, snapshot_id=snapshot_id)
            row = node_table.rows.pop(node_key, None)
            if row is not None:
                node_table.rows_by_snapshot[snapshot_id].pop(node_key.id, None)
                if not node_table.rows_by_snapshot[snapshot_id]:
                    node_table.rows_by_snapshot.pop(snapshot_id, None)
                if row.parent_ptr is not None:
                    parent_table = context.get(row.parent_ptr)
                    parent_key = VersionedNodeKey(id=row.parent_ptr.id, snapshot_id=snapshot_id)
                    parent_table.rows_by_parent[parent_key].remove(row)

        logger.trace(
            f"memory.{edit_type.name.lower()}",
            change=change,
            edits=len(edits),
            span="current",
        )
        return edits, cascaded_edits

    else:
        assert_never(edit_type)

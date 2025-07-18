import {
  CASCADING_EDIT_TYPES,
  EdgeDirection,
  EditEvent,
  EditOperation,
  EditType,
  NodeDefinitionReference,
  NodeReference,
  ScalarType,
} from "@destack/language";
import {
  NODE_DELETED_AT_KEY as ENTITY_DELETED_AT_KEY,
  ENTITY_PARENT_KEY,
  MAX_RECURSION_DEPTH,
  MemoryContext,
} from "@destack/store/memory/core";
import { walkNode } from "@destack/store/memory/entity/query";
import { packEntityRow } from "@destack/store/memory/entity/wiring";
import { getLogger, getTracer, traceFunction } from "@destack/utils";

const logger = getLogger("memory.entity.edit");
const tracer = getTracer("memory.entity.edit");

/**
 * Execute the Edits in-memory.
 */
function _executeEdits(options: { context: MemoryContext; edits: EditEvent[] }): {
  edits: EditEvent[];
  cascadedEdits: EditEvent[];
} {
  const { context, edits } = options;
  if (edits.length === 0) {
    return { edits: [], cascadedEdits: [] };
  }

  const optimizedEdits = optimizeEdits({ context, edits });
  const cascadedEdits: EditEvent[] = [];
  const appliedEdits: EditEvent[] = [];

  let currentDefinition = NodeDefinitionReference.of(optimizedEdits[0].nodePtr);
  let currentEditType = optimizedEdits[0].type;
  let currentBatch: EditEvent[] = [];

  for (const edit of edits) {
    const editDefinition = NodeDefinitionReference.of(edit.nodePtr);
    if (editDefinition.nodeType !== currentDefinition.nodeType || edit.type !== currentEditType) {
      const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = executeEdit({
        context,
        edits: currentBatch,
        definition: currentDefinition,
        editType: currentEditType,
      });
      appliedEdits.push(...batchAppliedEdits);
      cascadedEdits.push(...batchCascadedEdits);
      currentDefinition = editDefinition;
      currentEditType = edit.type;
      currentBatch = [];
    }
    currentBatch.push(edit);
  }

  // flush last batch
  if (currentBatch.length > 0) {
    const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = executeEdit({
      context,
      edits: currentBatch,
      definition: currentDefinition,
      editType: currentEditType,
    });
    appliedEdits.push(...batchAppliedEdits);
    cascadedEdits.push(...batchCascadedEdits);
  }

  return { edits: appliedEdits, cascadedEdits };
}
export const executeEdits = traceFunction(tracer, "execute_edits", _executeEdits);

/**
 * Optimize the Edits while retaining semantic equivalence.
 * Reorder and batch non-interfering Edits to minimize roundtrips.
 */
function optimizeEdits(options: { context: MemoryContext; edits: EditEvent[] }): EditEvent[] {
  const { context, edits } = options;
  const optimizedEdits: EditEvent[] = [];
  const buffer: EditEvent[] = [];

  function flush() {
    if (buffer.length === 0) {
      return;
    }
    const grouped = new Map<string, EditEvent[]>();
    for (const e of buffer) {
      const key = `${e.nodePtr.type}:${e.type}`;
      if (!grouped.has(key)) {
        grouped.set(key, []);
      }
      grouped.get(key)!.push(e);
    }
    for (const batch of grouped.values()) {
      optimizedEdits.push(...batch);
    }
    buffer.length = 0;
  }

  for (const edit of edits) {
    if (CASCADING_EDIT_TYPES.includes(edit.type)) {
      flush(); // close current segment
      optimizedEdits.push(edit); // keep position
    } else {
      buffer.push(edit); // postpone
    }
  }

  flush(); // trailing segment
  return optimizedEdits;
}

/**
 * Get the cascaded Nodes for an Edit.
 */
function _executeCascade(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  nodePtrs: NodeReference[];
  includeDeleted?: boolean | Array<string>;
}): { cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> } {
  const { context, definition, nodePtrs, includeDeleted } = options;
  const { cascadedNodePtrs, sourceIdByNodeId } = walkNode({
    context,
    definition,
    nodesPtrs: nodePtrs,
    direction: EdgeDirection.CHILD,
    depth: MAX_RECURSION_DEPTH,
    includeDeleted,
    snapshotPath: [],
  });
  return { cascadedNodePtrs, sourceIdByNodeId };
}
const executeCascade = traceFunction(tracer, "execute_cascade", _executeCascade);

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the applied Edits and any cascaded Edits.
 */
function _executeEdit(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  editType: EditType;
  edits: EditEvent[];
}): { edits: EditEvent[]; cascadedEdits: EditEvent[] } {
  const { context, definition, edits, editType } = options;
  const table = context.getEntityTable(definition);

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = table.getNodeKey({ id: edit.nodePtr.id, snapshotId });
      if (editType === EditType.UPSERT || !table.rows.has(nodeKey)) {
        const row = packEntityRow(edit.value);
        // main table
        table.rows.set(nodeKey, row);
        // rowsBySnapshot
        if (!table.rowsBySnapshot.has(snapshotId)) {
          table.rowsBySnapshot.set(snapshotId, new Map());
        }
        table.rowsBySnapshot.get(snapshotId)!.set(edit.nodePtr.id, row);
        // parent-child relationships
        if (row.parentPtr) {
          const parentTable = context.getEntityTable(row.parentPtr);
          const parentKey = parentTable.getNodeKey({ id: row.parentPtr.id, snapshotId });
          if (!parentTable.rowsByParent.has(parentKey)) {
            parentTable.rowsByParent.set(parentKey, []);
          }
          parentTable.rowsByParent.get(parentKey)!.push(row);
        }
      }
    }
    return { edits, cascadedEdits: [] };
  }

  // update
  else if (editType === EditType.UPDATE) {
    for (const edit of edits) {
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      if (!edit.propertyId) {
        throw new Error(`no propertyId for ${edit.repr()}`);
      }
      const nodeKey = table.getNodeKey({ id: edit.nodePtr.id, snapshotId });
      const row = table.rows.get(nodeKey);
      if (!row) {
        throw new Error(`node not found for ${edit.repr()}: ${edit.nodePtr.repr()}`);
      } else if (edit.operation === EditOperation.SET) {
        if (!edit.value) {
          throw new Error(`no value for ${edit.repr()}`);
        }
        row.value[String(edit.propertyId)] = edit.value.value;
      } else {
        throw new Error(`unsupported operation: ${edit.repr()}`);
      }
    }
    return { edits, cascadedEdits: [] };
  }

  // move
  else if (editType === EditType.MOVE) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      if (edit.value.type.scalarType !== ScalarType.NODE_REFERENCE) {
        throw new Error(`unexpected value: ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = table.getNodeKey({ id: edit.nodePtr.id, snapshotId });
      const row = table.rows.get(nodeKey);
      if (!row) {
        throw new Error(`node not found for ${edit.repr()}: ${edit.nodePtr.repr()}`);
      }
      // remove from old parent
      if (row.parentPtr) {
        const parentTable = context.getEntityTable(row.parentPtr);
        const parentKey = parentTable.getNodeKey({ id: row.parentPtr.id, snapshotId });
        const children = parentTable.rowsByParent.get(parentKey);
        if (children) {
          const index = children.indexOf(row);
          if (index >= 0) {
            children.splice(index, 1);
          }
        }
      }
      // update parent pointer
      row.parentPtr = NodeReference.fromValue(edit.value.value);
      row.value[ENTITY_PARENT_KEY] = edit.value.value;
      // add to new parent
      if (row.parentPtr) {
        const parentTable = context.getEntityTable(row.parentPtr);
        const parentKey = parentTable.getNodeKey({ id: row.parentPtr.id, snapshotId });
        if (!parentTable.rowsByParent.has(parentKey)) {
          parentTable.rowsByParent.set(parentKey, []);
        }
        parentTable.rowsByParent.get(parentKey)!.push(row);
      }
    }
    return { edits, cascadedEdits: [] };
  }

  // delete/restore
  else if (editType === EditType.DELETE || editType === EditType.RESTORE) {
    // cascade
    const nodesPtrs = edits.map((edit) => edit.nodePtr);
    const editByNodeId = new Map<string, EditEvent>();
    for (const edit of edits) {
      editByNodeId.set(edit.nodePtr.id, edit);
    }

    let includeDeleted: boolean | Array<string> = false;
    if (editType === EditType.RESTORE) {
      // restrict to nodes with same deleted_at
      includeDeleted = [];
      for (const nodePtr of nodesPtrs) {
        const snapshotId = nodePtr.snapshotId || null;
        const nodeKey = table.getNodeKey({ id: nodePtr.id, snapshotId });
        const row = table.rows.get(nodeKey);
        if (!row) {
          throw new Error(`node not found: ${nodePtr.repr()}`);
        } else {
          const deletedAt = row.value[ENTITY_DELETED_AT_KEY];
          if (deletedAt && !includeDeleted.includes(deletedAt)) {
            includeDeleted.push(deletedAt);
          }
        }
      }
    }

    const { cascadedNodePtrs, sourceIdByNodeId } = executeCascade({
      context,
      definition,
      nodePtrs: nodesPtrs,
      includeDeleted,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // update timestamps
    for (const nodePtr of [...nodesPtrs, ...cascadedNodePtrs]) {
      const edit = editByNodeId.get(sourceIdByNodeId.get(nodePtr.id) || nodePtr.id);
      if (!edit) {
        throw new Error(`edit not found for ${nodePtr.repr()}`);
      }
      const snapshotId = edit.snapshotPtr?.id || null;
      const nodeKey = table.getNodeKey({ id: nodePtr.id, snapshotId });
      const row = table.rows.get(nodeKey);
      if (!row) {
        throw new Error(`node not found for ${edit.repr()}: ${nodePtr.repr()}`);
      } else if (editType === EditType.DELETE) {
        row.value[ENTITY_DELETED_AT_KEY] = edit.createdAt.toString({ timeZoneName: "never" });
      } else if (editType === EditType.RESTORE) {
        delete row.value[ENTITY_DELETED_AT_KEY];
      }
    }

    return { edits, cascadedEdits };
  }

  //
  else {
    throw new Error(`unsupported edit type: ${EditType[editType]}`);
  }
}
const executeEdit = traceFunction(tracer, "execute_edit", _executeEdit);

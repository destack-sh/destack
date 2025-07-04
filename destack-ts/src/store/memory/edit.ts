import {
  CASCADING_EDIT_TYPES,
  Change,
  Edit,
  EditOperation,
  EditType,
  IsArchivable,
  IsDeletable,
  Node,
  NodeReference,
  ScalarType,
} from "@destack/language";

import { MemoryContext, MemoryDatabase, MemoryTable, VersionedNodeKey } from "./core";
import { packNodeRow } from "./wiring";

const NODE_PARENT_KEY = String(Node.property("parent").id);

const ARCHIVED_AT_KEY = String(IsArchivable.property("archived_at").id);
const DELETED_AT_KEY = String(IsDeletable.property("deleted_at").id);

/**
 * Execute the Change.
 */
export function executeChange(options: {
  database: MemoryDatabase;
  context: MemoryContext;
  change: Change;
}): { edits: Edit[]; cascadedEdits: Edit[] } {
  const { database, context, change } = options;

  if (!change.edits || change.edits.length === 0) {
    throw new Error(`no Edits in ${change.repr()}`);
  }

  const edits = optimizeChange({ context, edits: change.edits });
  const cascadedEdits: Edit[] = [];
  const appliedEdits: Edit[] = [];

  let currentTable = context.get(change.edits[0].nodePtr);
  let currentEditType = change.edits[0].type;
  let currentBatch: Edit[] = [];

  for (const edit of edits) {
    const editTable = context.get(edit.nodePtr);
    if (editTable !== currentTable || edit.type !== currentEditType) {
      const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = executeDataEdit({
        database,
        context,
        change,
        table: currentTable,
        editType: currentEditType,
        edits: currentBatch,
      });
      appliedEdits.push(...batchAppliedEdits);
      cascadedEdits.push(...batchCascadedEdits);
      currentTable = editTable;
      currentEditType = edit.type;
      currentBatch = [];
    }
    currentBatch.push(edit);
  }

  if (currentBatch.length > 0) {
    const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = executeDataEdit({
      database,
      context,
      change,
      table: currentTable,
      editType: currentEditType,
      edits: currentBatch,
    });
    appliedEdits.push(...batchAppliedEdits);
    cascadedEdits.push(...batchCascadedEdits);
  }

  return { edits: appliedEdits, cascadedEdits };
}

/**
 * Optimize the Change/Edits while retaining semantic equivalence.
 * Reorder and batch non-interfering Edits to minimize roundtrips.
 */
function optimizeChange(options: { context: MemoryContext; edits: Edit[] }): Edit[] {
  const { context, edits } = options;
  const optimizedEdits: Edit[] = [];
  const buffer: Edit[] = [];

  function flush() {
    if (buffer.length === 0) {
      return;
    }
    const grouped = new Map<string, Edit[]>();
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
function executeCascade(options: {
  database: MemoryDatabase;
  context: MemoryContext;
  table: MemoryTable;
  nodePtrs: NodeReference[];
}): NodeReference[] {
  throw new Error("not implemented");
}

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the applied Edits and any cascaded Edits.
 */
function executeDataEdit(options: {
  database: MemoryDatabase;
  context: MemoryContext;
  change: Change;
  table: MemoryTable;
  editType: EditType;
  edits: Edit[];
}): { edits: Edit[]; cascadedEdits: Edit[] } {
  const { database, context, change, table, editType, edits } = options;

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = table.getNodeKey({ id: edit.nodePtr.id, snapshotId });
      if (editType === EditType.UPSERT || !table.rows.has(nodeKey)) {
        const row = packNodeRow(table, edit.value);
        // main table
        table.rows.set(nodeKey, row);
        // rowsBySnapshot
        if (!table.rowsBySnapshot.has(snapshotId)) {
          table.rowsBySnapshot.set(snapshotId, new Map());
        }
        table.rowsBySnapshot.get(snapshotId)!.set(edit.nodePtr.id, row);
        // parent-child relationships
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
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
      if (!edit.attribute) {
        throw new Error(`no attribute for ${edit.repr()}`);
      }
      const nodeKey = table.getNodeKey({ id: edit.nodePtr.id, snapshotId });
      const row = table.rows.get(nodeKey);
      if (row) {
        if (edit.operation === EditOperation.SET) {
          if (!edit.value) {
            throw new Error(`no value for ${edit.repr()}`);
          }
          row.value[String(edit.attribute.id)] = edit.value.value;
        } else if (edit.operation === EditOperation.CLEAR) {
          delete row.value[String(edit.attribute.id)];
        } else {
          throw new Error(`unsupported operation: ${edit.repr()}`);
        }
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

      if (row) {
        // remove from old parent
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
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
        row.value[NODE_PARENT_KEY] = edit.value.value;
        // add to new parent
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
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

  // archive/unarchive/delete/restore
  else if (
    editType === EditType.ARCHIVE ||
    editType === EditType.UNARCHIVE ||
    editType === EditType.DELETE ||
    editType === EditType.RESTORE
  ) {
    // cascade
    const cascadedNodePtrs = executeCascade({
      database,
      context,
      table,
      nodePtrs: edits.map((edit) => edit.nodePtr),
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new Edit({ type: editType, node: nodePtr }),
    );

    // update timestamps
    for (const nodePtr of cascadedNodePtrs) {
      const nodeTable = context.get(nodePtr);
      const snapshotId = nodePtr.snapshotId || null;
      const nodeKey = nodeTable.getNodeKey({ id: nodePtr.id, snapshotId });
      const row = nodeTable.rows.get(nodeKey);

      if (row) {
        if (editType === EditType.ARCHIVE) {
          row.value[ARCHIVED_AT_KEY] = change.createdAt;
        } else if (editType === EditType.UNARCHIVE) {
          delete row.value[ARCHIVED_AT_KEY];
        } else if (editType === EditType.DELETE) {
          row.value[DELETED_AT_KEY] = change.createdAt;
        } else if (editType === EditType.RESTORE) {
          delete row.value[DELETED_AT_KEY];
        }
      }
    }

    return { edits, cascadedEdits };
  }

  // erase
  else if (editType === EditType.ERASE) {
    // cascade
    const cascadedNodePtrs = executeCascade({
      database,
      context,
      table,
      nodePtrs: edits.map((edit) => edit.nodePtr),
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new Edit({ type: editType, node: nodePtr }),
    );

    // delete rows
    for (const nodePtr of cascadedNodePtrs) {
      const nodeTable = context.get(nodePtr);
      const snapshotId = nodePtr.snapshotId || null;
      const nodeKey = nodeTable.getNodeKey({ id: nodePtr.id, snapshotId });
      const row = nodeTable.rows.get(nodeKey);

      if (row) {
        // remove from main table
        nodeTable.rows.delete(nodeKey);
        // remove from rowsBySnapshot
        const snapshotRows = nodeTable.rowsBySnapshot.get(snapshotId);
        if (snapshotRows) {
          snapshotRows.delete(nodePtr.id);
          if (snapshotRows.size === 0) {
            nodeTable.rowsBySnapshot.delete(snapshotId);
          }
        }
        // remove from parent's children
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
          const parentKey = parentTable.getNodeKey({ id: row.parentPtr.id, snapshotId });
          const children = parentTable.rowsByParent.get(parentKey);
          if (children) {
            const index = children.indexOf(row);
            if (index >= 0) {
              children.splice(index, 1);
            }
          }
        }
      }
    }

    return { edits, cascadedEdits };
  } else {
    throw new Error(`Unsupported edit type: ${editType}`);
  }
}

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

interface ExecuteChangeOptions {
  database: MemoryDatabase;
  context: MemoryContext;
  change: Change;
}

interface ExecuteDataEditOptions {
  database: MemoryDatabase;
  context: MemoryContext;
  change: Change;
  table: MemoryTable;
  editType: EditType;
  edits: Edit[];
}

interface ExecuteCascadeOptions {
  database: MemoryDatabase;
  context: MemoryContext;
  table: MemoryTable;
  nodePtrs: NodeReference[];
}

/**
 * Execute the Change.
 */
export function executeChange(options: ExecuteChangeOptions): [Edit[], Edit[]] {
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
      const [batchAppliedEdits, batchCascadedEdits] = executeDataEdit({
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
    const [batchAppliedEdits, batchCascadedEdits] = executeDataEdit({
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

  return [appliedEdits, cascadedEdits];
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
function executeCascade(options: ExecuteCascadeOptions): NodeReference[] {
  // TODO: Implement cascading logic
  throw new Error("Not implemented");
}

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the applied Edits and any cascaded Edits.
 */
function executeDataEdit(options: ExecuteDataEditOptions): [Edit[], Edit[]] {
  const { database, context, change, table, editType, edits } = options;

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey: VersionedNodeKey = { id: edit.nodePtr.id, snapshotId };
      const nodeKeyStr = table.createKey(nodeKey);

      if (editType === EditType.UPSERT || !table.rows.has(nodeKeyStr)) {
        const row = packNodeRow(table, edit.value);
        
        // Set row in main table
        table.rows.set(nodeKeyStr, row);
        
        // Update rowsBySnapshot
        if (!table.rowsBySnapshot.has(snapshotId)) {
          table.rowsBySnapshot.set(snapshotId, new Map());
        }
        table.rowsBySnapshot.get(snapshotId)!.set(nodeKey.id, row);
        
        // Update parent-child relationships
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
          const parentKey: VersionedNodeKey = { id: row.parentPtr.id, snapshotId };
          const parentKeyStr = parentTable.createKey(parentKey);
          
          if (!parentTable.rowsByParent.has(parentKeyStr)) {
            parentTable.rowsByParent.set(parentKeyStr, []);
          }
          parentTable.rowsByParent.get(parentKeyStr)!.push(row);
        }
      }
    }
    return [edits, []];
  }

  // update
  else if (editType === EditType.UPDATE) {
    for (const edit of edits) {
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      if (!edit.attribute) {
        throw new Error(`no prop_ptr for ${edit.repr()}`);
      }
      const nodeKey: VersionedNodeKey = { id: edit.nodePtr.id, snapshotId };
      const nodeKeyStr = table.createKey(nodeKey);
      const row = table.rows.get(nodeKeyStr);
      
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
    return [edits, []];
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
      const nodeKey: VersionedNodeKey = { id: edit.nodePtr.id, snapshotId };
      const nodeKeyStr = table.createKey(nodeKey);
      const row = table.rows.get(nodeKeyStr);
      
      if (row) {
        // Remove from old parent
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
          const parentKey: VersionedNodeKey = { id: row.parentPtr.id, snapshotId };
          const parentKeyStr = parentTable.createKey(parentKey);
          const children = parentTable.rowsByParent.get(parentKeyStr);
          if (children) {
            const index = children.indexOf(row);
            if (index >= 0) {
              children.splice(index, 1);
            }
          }
        }
        
        // Update parent pointer
        row.parentPtr = edit.value.value as NodeReference;
        row.value[NODE_PARENT_KEY] = edit.value.value;
        
        // Add to new parent
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
          const parentKey: VersionedNodeKey = { id: row.parentPtr.id, snapshotId };
          const parentKeyStr = parentTable.createKey(parentKey);
          
          if (!parentTable.rowsByParent.has(parentKeyStr)) {
            parentTable.rowsByParent.set(parentKeyStr, []);
          }
          parentTable.rowsByParent.get(parentKeyStr)!.push(row);
        }
      }
    }
    return [edits, []];
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
    const cascadedEdits = cascadedNodePtrs.map((nodePtr) => 
      new Edit({ type: editType, node: nodePtr })
    );

    // update timestamps
    for (const nodePtr of cascadedNodePtrs) {
      const nodeTable = context.get(nodePtr);
      const snapshotId = nodePtr.snapshotId || null;
      const nodeKey: VersionedNodeKey = { id: nodePtr.id, snapshotId };
      const nodeKeyStr = nodeTable.createKey(nodeKey);
      const row = nodeTable.rows.get(nodeKeyStr);
      
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

    return [edits, cascadedEdits];
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
    const cascadedEdits = cascadedNodePtrs.map((nodePtr) => 
      new Edit({ type: editType, node: nodePtr })
    );

    // delete rows
    for (const nodePtr of cascadedNodePtrs) {
      const nodeTable = context.get(nodePtr);
      const snapshotId = nodePtr.snapshotId || null;
      const nodeKey: VersionedNodeKey = { id: nodePtr.id, snapshotId };
      const nodeKeyStr = nodeTable.createKey(nodeKey);
      const row = nodeTable.rows.get(nodeKeyStr);
      
      if (row) {
        // Remove from main table
        nodeTable.rows.delete(nodeKeyStr);
        
        // Remove from rowsBySnapshot
        const snapshotRows = nodeTable.rowsBySnapshot.get(snapshotId);
        if (snapshotRows) {
          snapshotRows.delete(nodeKey.id);
          if (snapshotRows.size === 0) {
            nodeTable.rowsBySnapshot.delete(snapshotId);
          }
        }
        
        // Remove from parent's children
        if (row.parentPtr) {
          const parentTable = context.get(row.parentPtr);
          const parentKey: VersionedNodeKey = { id: row.parentPtr.id, snapshotId };
          const parentKeyStr = parentTable.createKey(parentKey);
          const children = parentTable.rowsByParent.get(parentKeyStr);
          if (children) {
            const index = children.indexOf(row);
            if (index >= 0) {
              children.splice(index, 1);
            }
          }
        }
      }
    }

    return [edits, cascadedEdits];
  } else {
    throw new Error(`Unsupported edit type: ${editType}`);
  }
}

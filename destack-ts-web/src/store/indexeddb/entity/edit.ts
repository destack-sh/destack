import { IndexedDBContext, MAX_RECURSION_DEPTH, NODE_PARENT_KEY } from "@destack-web/store/indexeddb/core";
import { getEntityKey } from "@destack-web/store/indexeddb/map";
import { packEntityRow } from "@destack-web/store/indexeddb/entity/wiring";
import {
  CASCADING_EDIT_TYPES,
  Condition,
  EdgeDirection,
  EditEvent,
  EditOperation,
  EditType,
  IsArchivable,
  IsDeletable,
  NodeDefinitionReference,
  NodeReference,
  ScalarType,
} from "@destack/language";
import { IDBPTransaction } from "idb";
import { Temporal } from "temporal-polyfill";
import { walkNode } from "@destack-web/store/indexeddb/entity/query";

// define these constants since they're not in Indexeddb core yet
const NODE_ARCHIVED_AT_KEY = String(IsArchivable.property("archived_at").id);
const NODE_DELETED_AT_KEY = String(IsDeletable.property("deleted_at").id);

/**
 * Execute the Edits in IndexedDB.
 */
export async function executeEdits(options: {
  tx: IDBPTransaction<unknown, string[], "readwrite">;
  context: IndexedDBContext;
  edits: EditEvent[];
}): Promise<{
  edits: EditEvent[];
  cascadedEdits: EditEvent[];
}> {
  const { tx, context, edits } = options;

  if (!edits || edits.length === 0) {
    return { edits: [], cascadedEdits: [] };
  }

  const optimizedEdits = optimizeEdits({ context, edits });
  const cascadedEdits: EditEvent[] = [];
  const appliedEdits: EditEvent[] = [];

  let currentDefinition = NodeDefinitionReference.of(optimizedEdits[0].nodePtr);
  let currentEditType = optimizedEdits[0].type;
  let currentBatch: EditEvent[] = [];

  for (const edit of optimizedEdits) {
    const editDefinition = NodeDefinitionReference.of(edit.nodePtr);
    if (editDefinition.nodeType !== currentDefinition.nodeType || edit.type !== currentEditType) {
      const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = await executeEdit({
        tx,
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

  if (currentBatch.length > 0) {
    const { edits: batchAppliedEdits, cascadedEdits: batchCascadedEdits } = await executeEdit({
      tx,
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

/**
 * Optimize the Edits while retaining semantic equivalence.
 * Reorder and batch non-interfering Edits to minimize roundtrips.
 */
function optimizeEdits(options: { context: IndexedDBContext; edits: EditEvent[] }): EditEvent[] {
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
 * This is a placeholder that assumes walkNode will be implemented.
 */
async function executeCascade(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  nodePtrs: NodeReference[];
  where: Condition | null;
}): Promise<{ cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodePtrs, where } = options;
  const { cascadedNodePtrs, sourceIdByNodeId } = await walkNode({
    tx,
    context,
    definition,
    nodesPtrs: nodePtrs,
    direction: EdgeDirection.CHILD,
    depth: MAX_RECURSION_DEPTH,
    where,
    snapshotPath: [],
  });
  return { cascadedNodePtrs, sourceIdByNodeId };
}

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the applied Edits and any cascaded Edits.
 */
async function executeEdit(options: {
  tx: IDBPTransaction<unknown, string[], "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  editType: EditType;
  edits: EditEvent[];
}): Promise<{ edits: EditEvent[]; cascadedEdits: EditEvent[] }> {
  const { tx, context, definition, edits, editType } = options;
  const table = context.getEntityTable(definition);
  const store = tx.objectStore(table.name);

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);

      if (editType === EditType.UPSERT) {
        const row = packEntityRow(edit.nodePtr, edit.value);
        await store.put(row);
      } else {
        // check if exists for CREATE
        const existing = await store.get(nodeKey);
        if (!existing) {
          const row = packEntityRow(edit.nodePtr, edit.value);
          await store.put(row);
        }
      }
    }
    return { edits, cascadedEdits: [] };
  }

  // update
  else if (editType === EditType.UPDATE) {
    for (const edit of edits) {
      if (!edit.attribute) {
        throw new Error(`no attribute for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);
      const row = await store.get(nodeKey);

      if (row) {
        if (edit.operation === EditOperation.SET) {
          if (!edit.value) {
            throw new Error(`no value for ${edit.repr()}`);
          }
          row[String(edit.attribute.id)] = edit.value.value;
        } else if (edit.operation === EditOperation.CLEAR) {
          delete row[String(edit.attribute.id)];
        } else {
          throw new Error(`unsupported operation: ${edit.repr()}`);
        }
        await store.put(row);
      }
    }
    return { edits, cascadedEdits: [] };
  }

  // move
  else if (editType === EditType.MOVE) {
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      } else if (edit.value.type.scalarType !== ScalarType.NODE_REFERENCE) {
        throw new Error(`unexpected value: ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);
      const row = await store.get(nodeKey);

      if (row) {
        row[NODE_PARENT_KEY] = edit.value.value;
        await store.put(row);
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
    const nodesPtrs = edits.map((edit) => edit.nodePtr);
    const editedAtByNodeId = new Map<string, Temporal.ZonedDateTime>();
    for (const edit of edits) {
      editedAtByNodeId.set(edit.nodePtr.id, edit.createdAt);
    }

    let where: Condition | null = null;
    if (editType === EditType.UNARCHIVE || editType === EditType.RESTORE) {
      // restrict to nodes with same deleted_at/archived_at
      const rootDts = new Set<Temporal.ZonedDateTime>();
      for (const nodePtr of nodesPtrs) {
        const snapshotId = nodePtr.snapshotId ? nodePtr.snapshotId : null;
        const nodeKey = getEntityKey(nodePtr.id, snapshotId);
        const row = await store.get(nodeKey);
        if (row) {
          if (editType === EditType.UNARCHIVE) {
            const archivedAt = row[NODE_ARCHIVED_AT_KEY];
            if (archivedAt) {
              rootDts.add(Temporal.Instant.from(archivedAt).toZonedDateTimeISO("UTC"));
            }
          } else if (editType === EditType.RESTORE) {
            const deletedAt = row[NODE_DELETED_AT_KEY];
            if (deletedAt) {
              rootDts.add(Temporal.Instant.from(deletedAt).toZonedDateTimeISO("UTC"));
            }
          }
        }
      }
      if (rootDts.size > 0) {
        if (editType === EditType.UNARCHIVE) {
          where = IsArchivable.property("archived_at").in(...Array.from(rootDts));
        } else if (editType === EditType.RESTORE) {
          where = IsDeletable.property("deleted_at").in(...Array.from(rootDts));
        }
      }
    }

    const { cascadedNodePtrs, sourceIdByNodeId } = await executeCascade({
      tx,
      context,
      definition,
      nodePtrs: nodesPtrs,
      where,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // update timestamps
    for (const nodePtr of [...nodesPtrs, ...cascadedNodePtrs]) {
      const snapshotId = nodePtr.snapshotId ? nodePtr.snapshotId : null;
      const nodeKey = getEntityKey(nodePtr.id, snapshotId);
      const row = await store.get(nodeKey);

      if (row) {
        if (editType === EditType.ARCHIVE) {
          const editedAt = editedAtByNodeId.get(sourceIdByNodeId.get(nodePtr.id) || nodePtr.id);
          row[NODE_ARCHIVED_AT_KEY] = editedAt?.toString({ timeZoneName: "never" });
        } else if (editType === EditType.UNARCHIVE) {
          delete row[NODE_ARCHIVED_AT_KEY];
        } else if (editType === EditType.DELETE) {
          const editedAt = editedAtByNodeId.get(sourceIdByNodeId.get(nodePtr.id) || nodePtr.id);
          row[NODE_DELETED_AT_KEY] = editedAt?.toString({ timeZoneName: "never" });
        } else if (editType === EditType.RESTORE) {
          delete row[NODE_DELETED_AT_KEY];
        }
        await store.put(row);
      }
    }

    return { edits, cascadedEdits };
  }

  // erase
  else if (editType === EditType.ERASE) {
    // cascade
    const nodesPtrs = edits.map((edit) => edit.nodePtr);
    const { cascadedNodePtrs } = await executeCascade({
      tx,
      context,
      definition,
      nodePtrs: nodesPtrs,
      where: null,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // delete rows
    for (const nodePtr of [...nodesPtrs, ...cascadedNodePtrs]) {
      const nodeTable = context.getEntityTable(nodePtr);
      const nodeStore = tx.objectStore(nodeTable.name);
      const snapshotId = nodePtr.snapshotId ? nodePtr.snapshotId : null;
      const nodeKey = getEntityKey(nodePtr.id, snapshotId);
      await nodeStore.delete(nodeKey);
    }

    return { edits, cascadedEdits };
  } else {
    throw new Error(`Unsupported edit type: ${editType}`);
  }
}

import {
  ENTITY_PARENT_KEY,
  IndexedDBContext,
  MAX_RECURSION_DEPTH,
} from "@destack-web/store/indexeddb/core";
import { walkNode } from "@destack-web/store/indexeddb/entity/query";
import { packEntityRow } from "@destack-web/store/indexeddb/entity/wiring";
import { getEntityKey } from "@destack-web/store/indexeddb/map";
import {
  CASCADING_EDIT_TYPES,
  EdgeDirection,
  EditEvent,
  EditOperation,
  EditType,
  Entity,
  NodeDefinitionReference,
  NodeReference,
  ScalarType,
} from "@destack/language";
import { getLogger, getTracer, traceFunction } from "@destack/utils";
import { IDBPTransaction } from "idb";

// define these constants since they're not in Indexeddb core yet
const NODE_DELETED_AT_KEY = String(Entity.property("deleted_at").id);

const tracer = getTracer("indexeddb.entity.edit");
const logger = getLogger("indexeddb.entity.edit");

/**
 * Execute the Edits in IndexedDB.
 */
async function _executeEdits(options: {
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
export const executeEdits = traceFunction(tracer, "execute_edits", _executeEdits);

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
async function _executeCascade(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  nodePtrs: NodeReference[];
  includeDeleted?: boolean | Array<string>;
}): Promise<{ cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodePtrs, includeDeleted } = options;
  const { cascadedNodePtrs, sourceIdByNodeId } = await walkNode({
    tx,
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
async function _executeEdit(options: {
  tx: IDBPTransaction<unknown, string[], "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  editType: EditType;
  edits: EditEvent[];
}): Promise<{ edits: EditEvent[]; cascadedEdits: EditEvent[] }> {
  const { tx, context, definition, edits, editType } = options;

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    const table = context.getEntityTable(definition);
    const store = tx.objectStore(table.name);
    const editPromises: Promise<any>[] = [];

    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);
      const row = packEntityRow(edit.nodePtr, edit.value);
      if (editType === EditType.UPSERT) {
        editPromises.push(store.put(row));
      } else {
        editPromises.push(
          store.get(nodeKey).then((existing) => {
            if (!existing) {
              store.put(row);
            }
          }),
        );
      }
    }
    await Promise.all(editPromises);
    return { edits, cascadedEdits: [] };
  }

  // update
  else if (editType === EditType.UPDATE) {
    const table = context.getEntityTable(definition);
    const store = tx.objectStore(table.name);
    for (const edit of edits) {
      if (!edit.propertyId) {
        throw new Error(`no propertyId for ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);
      const row = await store.get(nodeKey);
      if (!row) {
        throw new Error(`node not found for ${edit.repr()}: ${edit.nodePtr.repr()}`);
      } else if (edit.operation === EditOperation.SET) {
        if (!edit.value) {
          throw new Error(`no value for ${edit.repr()}`);
        }
        row[String(edit.propertyId)] = edit.value.value;
      } else if (edit.operation === EditOperation.CLEAR) {
        delete row[String(edit.propertyId)];
      } else {
        throw new Error(`unsupported operation: ${edit.repr()}`);
      }
      await store.put(row);
    }
    return { edits, cascadedEdits: [] };
  }

  // move
  else if (editType === EditType.MOVE) {
    const table = context.getEntityTable(definition);
    const store = tx.objectStore(table.name);
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${edit.repr()}`);
      } else if (edit.value.type.scalarType !== ScalarType.NODE_REFERENCE) {
        throw new Error(`unexpected value: ${edit.repr()}`);
      }
      const snapshotId = edit.snapshotPtr ? edit.snapshotPtr.id : null;
      const nodeKey = getEntityKey(edit.nodePtr.id, snapshotId);
      const row = await store.get(nodeKey);
      if (!row) {
        throw new Error(`node not found for ${edit.repr()}: ${edit.nodePtr.repr()}`);
      }
      row[ENTITY_PARENT_KEY] = edit.value.value;
      await store.put(row);
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
      const editPromises: Promise<any>[] = [];
      for (const nodePtr of nodesPtrs) {
        const edit = editByNodeId.get(nodePtr.id);
        if (!edit) {
          throw new Error(`edit not found for ${nodePtr.repr()}`);
        }
        const table = context.getEntityTable(nodePtr);
        const store = tx.objectStore(table.name);
        const snapshotId = edit.snapshotPtr?.id || null;
        const nodeKey = getEntityKey(nodePtr.id, snapshotId);
        editPromises.push(
          store.get(nodeKey).then((row) => {
            if (!row) {
              throw new Error(`node not found for ${edit.repr()}: ${nodePtr.repr()}`);
            } else if (editType === EditType.RESTORE) {
              const deletedAt = row[NODE_DELETED_AT_KEY];
              if (deletedAt && !(includeDeleted as string[]).includes(deletedAt)) {
                (includeDeleted as string[]).push(deletedAt);
              }
            }
          }),
        );
      }
      await Promise.all(editPromises);
    }

    const { cascadedNodePtrs, sourceIdByNodeId } = await executeCascade({
      tx,
      context,
      definition,
      nodePtrs: nodesPtrs,
      includeDeleted,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // update timestamps
    const editPromises: Promise<any>[] = [];
    for (const nodePtr of [...nodesPtrs, ...cascadedNodePtrs]) {
      const table = context.getEntityTable(nodePtr);
      const store = tx.objectStore(table.name);
      const edit = editByNodeId.get(sourceIdByNodeId.get(nodePtr.id) || nodePtr.id);
      if (!edit) {
        throw new Error(`edit not found for ${nodePtr.repr()}`);
      }
      const snapshotId = edit?.snapshotPtr?.id || null;
      const nodeKey = getEntityKey(nodePtr.id, snapshotId);
      editPromises.push(
        store.get(nodeKey).then((row) => {
          if (!row) {
            throw new Error(`node not found for ${edit.repr()}: ${nodePtr.repr()}`);
          } else if (editType === EditType.DELETE) {
            row[NODE_DELETED_AT_KEY] = edit!.createdAt.toString({ timeZoneName: "never" });
          } else if (editType === EditType.RESTORE) {
            delete row[NODE_DELETED_AT_KEY];
          }
          return store.put(row);
        }),
      );
    }
    await Promise.all(editPromises);
    return { edits, cascadedEdits };
  }

  //
  else {
    throw new Error(`unsupported edit type: ${EditType[editType]}`);
  }
}
const executeEdit = traceFunction(tracer, "execute_edit", _executeEdit);

import { walkNode } from "@desys/store/postgres/entity/query";
import { TransactionSQL } from "bun";
import {
  CASCADING_EDIT_TYPES,
  Condition,
  EdgeDirection,
  EditEvent,
  EditOperation,
  EditType,
  Entity,
  getLogger,
  getTracer,
  Node,
  NODE_CLASS_BY_TYPE,
  NodeDefinitionReference,
  NodeReference,
  ScalarType,
  StoreDomain,
} from "destack";
import { Temporal } from "temporal-polyfill";
import { PostgresContext } from "./core";
import { packColumnWide, packNodeRow } from "./wiring";

const MAX_RECURSION_DEPTH = 100;

const NODE_ID_KEY = String(Node.property("id").id);
const ENTITY_PARENT_KEY = String(Entity.property("parent").id);
const ENTITY_PARENT_PROPERTY = Entity.property("parent");
const ENTITY_DELETED_AT_KEY = String(Entity.property("deleted_at").id);

const logger = getLogger(__filename);
const tracer = getTracer(__filename);

/**
 * Optimize the Edits while retaining semantic equivalence.
 * Reorder and batch non-interfering Edits to minimize roundtrips.
 */
function optimizeEdits(options: { context: PostgresContext; edits: EditEvent[] }): EditEvent[] {
  const { context, edits } = options;
  const optimizedEdits: EditEvent[] = [];
  const buffer: EditEvent[] = [];

  function flush(): void {
    if (buffer.length === 0) {
      return;
    }
    const grouped = new Map<string, EditEvent[]>();
    for (const e of buffer) {
      const definition = NodeDefinitionReference.of(e.nodePtr);
      const table = context.getEntityTable(definition);
      const key = `${table.name}:${e.type}`;
      if (!grouped.has(key)) {
        grouped.set(key, []);
      }
      grouped.get(key)!.push(e);
    }
    for (const batch of grouped.values()) {
      optimizedEdits.push(...batch); // contiguously batched
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
async function executeCascade(options: {
  tx: TransactionSQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  nodePtrs: NodeReference[];
  where: Condition | null;
}): Promise<{ cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodePtrs, where } = options;

  const { nodes: childPtrs, sourceIdByNodeId } = await walkNode({
    tx,
    context,
    definition,
    nodesPtrs: nodePtrs,
    direction: EdgeDirection.CHILD,
    depth: MAX_RECURSION_DEPTH,
    where,
  });

  return { cascadedNodePtrs: childPtrs, sourceIdByNodeId };
}

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the Edits and any cascaded Edits.
 */
async function executeEdit(options: {
  tx: TransactionSQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  editType: EditType;
  edits: EditEvent[];
}): Promise<{ edits: EditEvent[]; cascadedEdits: EditEvent[] }> {
  const { tx, context, definition, editType, edits } = options;

  const table = context.getEntityTable(definition);
  const nodeType = table.nodeType;
  const nodeCls = NODE_CLASS_BY_TYPE[nodeType];

  // TODO :Performance: PostgresStore execute edits in bulk (insert/update)

  // create/upsert
  if (editType === EditType.CREATE || editType === EditType.UPSERT) {
    let stmt = `
INSERT INTO "${table.name}" (${table.columns.map((col) => `"${col.name}"`).join(", ")})
VALUES (${table.columns.map((_, i) => `$${i + 1}`).join(", ")})`;

    if (editType === EditType.UPSERT) {
      const overrideColumns = table.columns.filter(
        (col) => !col.isPrimaryKey && !col.prop.name.startsWith("created_"),
      );
      stmt += `
ON CONFLICT ("${NODE_ID_KEY}") DO UPDATE
SET ${overrideColumns.map((col) => `"${col.name}" = EXCLUDED."${col.name}"`).join(", ")}`;
    }
    stmt += ";";

    const valuesPacked: any[][] = [];
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${JSON.stringify(edit)}`);
      }
      const rowValuesPacked = packNodeRow({ table, value: edit.value });
      valuesPacked.push(rowValuesPacked);
    }

    for (const row of valuesPacked) {
      await tx.unsafe(stmt, row);
    }

    return { edits, cascadedEdits: [] };
  }

  // update
  else if (editType === EditType.UPDATE) {
    // update each edit one by one
    for (const edit of edits) {
      if (!edit.propertyId) {
        throw new Error(`no property_id for ${JSON.stringify(edit)}`);
      }
      const prop = definition.resolvePropertyMaybe(edit.propertyId);
      if (!prop) {
        throw new Error(`no property for ${JSON.stringify(edit)}`);
      }

      let valuePacked: any;
      if (edit.operation === EditOperation.SET) {
        if (!edit.value) {
          throw new Error(`no value for ${JSON.stringify(edit)}`);
        }
        valuePacked = edit.value.value;
      } else if (edit.operation === EditOperation.CLEAR) {
        valuePacked = null;
      } else {
        throw new Error(`unsupported operation: ${JSON.stringify(edit)}`);
      }

      const update: Map<string, any> = new Map();
      packColumnWide({
        type: prop.toType(),
        value: valuePacked,
        table,
        columnName: String(prop.id),
        columnOut: update,
      });

      const columnNames = Array.from(update.keys()).sort();
      const columnValues = columnNames.map((name) => update.get(name));

      const stmt = `
UPDATE "${table.name}"
SET ${columnNames.map((name, i) => `"${name}" = $${i + 1}`).join(", ")}
WHERE "${NODE_ID_KEY}" = $${columnNames.length + 1}`;

      await tx.unsafe(stmt, [...columnValues, edit.nodePtr.id]);
    }

    return { edits, cascadedEdits: [] };
  }

  // move
  else if (editType === EditType.MOVE) {
    // prepare statement
    if (nodeCls.__definition__.storeDomain !== StoreDomain.ENTITY) {
      throw new Error(`cannot move non-Entity ${nodeCls.name} in ${JSON.stringify(edits)}`);
    }
    const updateTemplate: Map<string, any> = new Map();
    packColumnWide({
      type: ENTITY_PARENT_PROPERTY.toType(),
      value: null,
      table,
      columnName: ENTITY_PARENT_KEY,
      columnOut: updateTemplate,
    });
    const stmt = `
UPDATE "${table.name}"
SET ${Object.keys(updateTemplate)
      .map((key, i) => `"${key}" = $${i + 1}`)
      .join(", ")}
WHERE "${NODE_ID_KEY}" = $${Object.keys(updateTemplate).length + 1}`;

    // prepare values
    const valuesPacked: any[][] = [];
    for (const edit of edits) {
      let parentPtrValue: any = null;
      if (edit.value?.value !== null && edit.value?.value !== undefined) {
        if (edit.value.type.scalarType !== ScalarType.NODE_REFERENCE) {
          throw new Error(`unexpected value: ${JSON.stringify(edit)}`);
        }
        parentPtrValue = edit.value.value;
      }

      const update: Map<string, any> = new Map();
      packColumnWide({
        type: ENTITY_PARENT_PROPERTY.toType(),
        value: parentPtrValue,
        table,
        columnName: ENTITY_PARENT_KEY,
        columnOut: update,
      });
      const rowValues = [...Object.values(update), edit.nodePtr.id];
      valuesPacked.push(rowValues);
    }

    await tx.unsafe(stmt, ...valuesPacked);

    return { edits, cascadedEdits: [] };
  }

  // delete/restore
  else if (editType === EditType.DELETE || editType === EditType.RESTORE) {
    // cascade
    const nodePtrs = edits.map((edit) => edit.nodePtr);
    const editedAtByNodeId = new Map<string, Temporal.ZonedDateTime>();
    for (const edit of edits) {
      editedAtByNodeId.set(String(edit.nodePtr.id), edit.createdAt);
    }

    let where: Condition | null = null;
    if (editType === EditType.RESTORE) {
      // restrict to nodes with same deleted_at
      const rootDts = new Set<Temporal.ZonedDateTime>();
      const rootStmt = `
SELECT "${NODE_ID_KEY}", "${ENTITY_DELETED_AT_KEY}"
FROM "${table.name}"
WHERE "${NODE_ID_KEY}"
IN (${nodePtrs.map((_, i) => `$${i + 1}`).join(", ")})`;
      const rootRows = await tx.unsafe(
        rootStmt,
        nodePtrs.map((n) => n.id),
      );
      for (const row of rootRows) {
        if (row[ENTITY_DELETED_AT_KEY]) {
          rootDts.add(Temporal.Instant.from(row[ENTITY_DELETED_AT_KEY]).toZonedDateTimeISO("UTC"));
        }
      }
      where = Entity.property("deleted_at").in(...Array.from(rootDts));
    }

    const { cascadedNodePtrs, sourceIdByNodeId } = await executeCascade({
      tx,
      context,
      definition,
      nodePtrs,
      where,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // update
    const editedNodePtrsByTable = new Map<string, string[]>();
    for (const nodePtr of [...nodePtrs, ...cascadedNodePtrs]) {
      const tableName = context.getEntityTable(nodePtr).name;
      const nodeIdPacked = String(nodePtr.id);
      if (!editedNodePtrsByTable.has(tableName)) {
        editedNodePtrsByTable.set(tableName, []);
      }
      editedNodePtrsByTable.get(tableName)!.push(nodeIdPacked);
    }

    for (const [tableName, tableNodeIds] of editedNodePtrsByTable) {
      const arguments_: any[][] = [];
      let updateStmt: string;

      if (editType === EditType.DELETE) {
        updateStmt = `"${ENTITY_DELETED_AT_KEY}" = $2`;
        for (const nodeId of tableNodeIds) {
          const editedAt = editedAtByNodeId.get(sourceIdByNodeId.get(nodeId) || nodeId);
          arguments_.push([nodeId, editedAt]);
        }
      } else if (editType === EditType.RESTORE) {
        updateStmt = `"${ENTITY_DELETED_AT_KEY}" = NULL`;
        for (const nodeId of tableNodeIds) {
          arguments_.push([nodeId]);
        }
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }

      const stmt = `
UPDATE "${tableName}"   
SET ${updateStmt}
WHERE "${NODE_ID_KEY}" = $1`;

      await tx.unsafe(stmt, ...arguments_);
    }

    return { edits, cascadedEdits };
  } else {
    throw new Error(`unexpected edit type: ${editType}`);
  }
}

/**
 * Execute the Edits in Postgres.
 */
export async function executeEdits(options: {
  tx: TransactionSQL;
  context: PostgresContext;
  edits: EditEvent[];
}): Promise<{ edits: EditEvent[]; cascadedEdits: EditEvent[] }> {
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
        definition: currentDefinition,
        editType: currentEditType,
        edits: currentBatch,
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
      definition: currentDefinition,
      editType: currentEditType,
      edits: currentBatch,
    });
    appliedEdits.push(...batchAppliedEdits);
    cascadedEdits.push(...batchCascadedEdits);
  }

  return { edits: appliedEdits, cascadedEdits };
}

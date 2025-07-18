import { walkNode } from "@desys/store/postgres/entity/query";
import { TransactionSQL } from "bun";
import {
  CASCADING_EDIT_TYPES,
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
  traceFunction,
} from "destack";
import { Temporal } from "temporal-polyfill";
import { PostgresContext } from "./core";
import { packColumnWide, packNodeRow } from "./wiring";

const MAX_RECURSION_DEPTH = 100;

const NODE_ID_KEY = String(Node.property("id").id);
const ENTITY_PARENT_KEY = String(Entity.property("parent").id);
const ENTITY_PARENT_PROPERTY = Entity.property("parent");
const ENTITY_DELETED_AT_KEY = String(Entity.property("deleted_at").id);

const logger = getLogger("postgres.entity.edit");
const tracer = getTracer("postgres.entity.edit");

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
  includeDeleted?: boolean | Array<string>;
}): Promise<{ cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodePtrs, includeDeleted } = options;

  const { nodes: childPtrs, sourceIdByNodeId } = await walkNode({
    tx,
    context,
    definition,
    nodesPtrs: nodePtrs,
    direction: EdgeDirection.CHILD,
    depth: MAX_RECURSION_DEPTH,
    includeDeleted,
  });

  return { cascadedNodePtrs: childPtrs, sourceIdByNodeId };
}

/**
 * Execute the Edits to the data (data only, no schema).
 * Returns the Edits and any cascaded Edits.
 */
async function _executeEdit(options: {
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

  // create
  if (editType === EditType.CREATE) {
    const rowsData: Record<string, any>[] = [];
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${JSON.stringify(edit)}`);
      }
      const row = packNodeRow({ table, value: edit.value });
      rowsData.push(row);
    }
    await tx`INSERT INTO ${tx(table.name)} ${tx(rowsData)}`;
    return { edits, cascadedEdits: [] };
  }

  // upsert
  else if (editType === EditType.UPSERT) {
    const overrideColumns = table.columns.filter(
      (col) => !col.isPrimaryKey && !col.prop.name.startsWith("created_"),
    );
    const rowsData: Record<string, any>[] = [];
    for (const edit of edits) {
      if (!edit.value) {
        throw new Error(`no value for ${JSON.stringify(edit)}`);
      }
      const row = packNodeRow({ table, value: edit.value });
      rowsData.push(row);
    }
    await tx`
      INSERT INTO ${tx(table.name)} ${tx(rowsData)}
      ON CONFLICT (${tx(NODE_ID_KEY)}) DO UPDATE
      SET ${overrideColumns.map((col) => tx`${tx(col.name)} = EXCLUDED.${tx(col.name)}`).join(", ")}
    `;
    return { edits, cascadedEdits: [] };
  }

  // update
  else if (editType === EditType.UPDATE) {
    // TODO :Performance: PostgresStore execute update edits in bulk

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
      } else {
        throw new Error(`unsupported operation: ${JSON.stringify(edit)}`);
      }

      const update: Record<string, any> = {};
      packColumnWide({
        type: prop.toType(),
        value: valuePacked,
        table,
        columnName: String(prop.id),
        columnOut: update,
      });
      const columnNames = Object.keys(update).sort();
      const columnValues = columnNames.map((name) => update[name]);

      const stmt = `
UPDATE "${table.name}"
SET ${columnNames.map((name, i) => `"${name}" = $${i + 1}`).join(", ")}
WHERE "${NODE_ID_KEY}" = $${columnNames.length + 1}::uuid`;

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
    // track cascade
    const nodePtrs = edits.map((edit) => edit.nodePtr);
    const editedAtByNodeId = new Map<string, Temporal.ZonedDateTime>();
    for (const edit of edits) {
      editedAtByNodeId.set(String(edit.nodePtr.id), edit.createdAt);
    }

    let includeDeleted: boolean | Array<string> = false;
    if (editType === EditType.RESTORE) {
      // restrict to nodes with same deleted_at
      includeDeleted = [];
      const rootStmt = `
SELECT "${NODE_ID_KEY}", "${ENTITY_DELETED_AT_KEY}"
FROM "${table.name}"
WHERE "${NODE_ID_KEY}" IN (${nodePtrs.map((_, i) => `$${i + 1}`).join(", ")})`;
      const rootRows = await tx.unsafe(
        rootStmt,
        nodePtrs.map((n) => n.id),
      );
      for (const row of rootRows) {
        if (
          row[ENTITY_DELETED_AT_KEY] &&
          !(includeDeleted as string[]).includes(row[ENTITY_DELETED_AT_KEY])
        ) {
          (includeDeleted as string[]).push(row[ENTITY_DELETED_AT_KEY]);
        }
      }
    }

    // do cascade
    const { cascadedNodePtrs, sourceIdByNodeId } = await executeCascade({
      tx,
      context,
      definition,
      nodePtrs,
      includeDeleted,
    });
    const cascadedEdits = cascadedNodePtrs.map(
      (nodePtr) => new EditEvent({ type: editType, node: nodePtr }),
    );

    // update timestamps
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
      if (editType === EditType.DELETE) {
        // set deleted_at to the specific deleted_at
        const params: unknown[] = [];
        const valuesTuples = tableNodeIds.map((nodeId, i) => {
          const editedAt = editedAtByNodeId.get(sourceIdByNodeId.get(nodeId) ?? nodeId);
          if (editedAt == null) {
            throw new Error(`missing editedAt for nodeId ${nodeId} (table ${tableName})`);
          }
          // push param pair
          params.push(nodeId, editedAt);
          const a = i * 2 + 1;
          const b = i * 2 + 2;
          return `($${a}::uuid,$${b}::timestamptz)`;
        });

        const stmt = `
          UPDATE "${tableName}" AS t
          SET "${ENTITY_DELETED_AT_KEY}" = v.edited_at
          FROM (VALUES ${valuesTuples.join(",")}) AS v("${NODE_ID_KEY}", edited_at)
          WHERE t."${NODE_ID_KEY}" = v."${NODE_ID_KEY}";
        `;

        await tx.unsafe(stmt, params);
      } else if (editType === EditType.RESTORE) {
        // restore sets all deleted_at to NULL, so a simple ANY() is fine
        const params: unknown[] = [];
        const idPlaceholders = tableNodeIds.map((nodeId, i) => {
          params.push(nodeId);
          return `$${i + 1}::uuid`;
        });

        const stmt = `
          UPDATE "${tableName}"
          SET "${ENTITY_DELETED_AT_KEY}" = NULL
          WHERE "${NODE_ID_KEY}" = ANY(ARRAY[${idPlaceholders.join(",")}]::uuid[]);
        `;

        await tx.unsafe(stmt, params);
      } else {
        throw new Error(`unexpected edit type: ${editType}`);
      }
    }

    return { edits, cascadedEdits };
  } else {
    throw new Error(`unexpected edit type: ${editType}`);
  }
}
const executeEdit = traceFunction(tracer, "execute_edit", _executeEdit);

/**
 * Execute the Edits in Postgres.
 */
async function _executeEdits(options: {
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
export const executeEdits = traceFunction(tracer, "execute_edits", _executeEdits);

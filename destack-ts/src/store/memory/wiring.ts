import {
  Entity,
  IsExtensible,
  IsSpatial,
  Materialization,
  Node,
  NodeReference,
  ScalarType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language";
import { MemoryRow, MemoryTable } from "@destack/store/memory/core";

const NODE_ID_KEY = String(Node.property("id").id);
const NODE_PARENT_PTR_KEY = String(Node.property("parent").id);
const NODE_SPACE_PTR_KEY = String(IsSpatial.property("space").id);
const NODE_DEFINITION_PTR_KEY = String(IsExtensible.property("definition").id);

const ENTITY_SNAPSHOT_PTR_KEY = String(Entity.property("snapshot").id);
const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);

const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);

/**
 * Pack a Value into a MemoryRow.
 */
export function packNodeRow(table: MemoryTable, value: Value): MemoryRow {
  const valuePacked = value.value;
  if (!valuePacked) {
    throw new Error(`no value for ${value.repr()}`);
  }

  const id = String(valuePacked[NODE_ID_KEY]);
  const ptr = new NodeReference({
    type: table.nodeType,
    id: String(valuePacked[NODE_ID_KEY]),
    spaceId:
      NODE_SPACE_PTR_KEY in valuePacked
        ? String(valuePacked[NODE_SPACE_PTR_KEY][NODE_REFERENCE_ID_KEY])
        : null,
    definitionId:
      NODE_DEFINITION_PTR_KEY in valuePacked
        ? String(valuePacked[NODE_DEFINITION_PTR_KEY][NODE_REFERENCE_ID_KEY])
        : null,
  });

  let parentPtr: NodeReference | null = null;
  const parentPtrValue = valuePacked[NODE_PARENT_PTR_KEY];
  if (parentPtrValue !== undefined) {
    parentPtr = NodeReference.fromValue(parentPtrValue);
  }

  let snapshotPtr: NodeReference | null = null;
  const snapshotPtrValue = valuePacked[ENTITY_SNAPSHOT_PTR_KEY];
  if (snapshotPtrValue !== undefined) {
    snapshotPtr = NodeReference.fromValue(snapshotPtrValue);
  }

  let materialization: Materialization | null = null;
  const materializationValue = valuePacked[ENTITY_MATERIALIZATION_KEY];
  if (materializationValue !== undefined) {
    materialization = materializationValue as Materialization;
  }

  const row = new MemoryRow(
    table,
    table.nodeType,
    id,
    snapshotPtr?.id || null,
    materialization,
    ptr,
    parentPtr,
    valuePacked,
  );

  return row;
}

/**
 * Unpack a MemoryRow to a Value.
 */
export function unpackNodeRow(table: MemoryTable, row: MemoryRow): Value {
  const typeInfo = new Type({
    cardinality: TypeCardinality.SCALAR,
    scalarType: ScalarType.NODE_VALUE,
    nodeType: row.nodeType,
  });

  return new Value({
    type: typeInfo,
    value: row.value,
  });
}

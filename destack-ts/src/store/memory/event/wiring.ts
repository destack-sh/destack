import {
  NodeReference,
  NodeType,
  ScalarType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language";
import {
  EVENT_CREATED_AT_KEY,
  EVENT_SNAPSHOT_KEY,
  NODE_DEFINITION_PTR_ID,
  NODE_ID_KEY,
  NODE_METATYPE_KEY,
  NODE_REFERENCE_ID_KEY,
  NODE_SPACE_PTR_ID,
} from "@destack/store/memory/core";
import { MemoryEventRow } from "@destack/store/memory/event/core";

/**
 * Pack a Value into a MemoryEventRow.
 */
export function packEventRow(value: Value): MemoryEventRow {
  const valuePacked = value.value;
  if (!valuePacked) {
    throw new Error(`no value for ${value.repr()}`);
  }

  const nodeType = Number(valuePacked[NODE_METATYPE_KEY]) as NodeType;
  const id = String(valuePacked[NODE_ID_KEY]);
  const ptr = new NodeReference({
    type: nodeType,
    id: String(valuePacked[NODE_ID_KEY]),
    spaceId: valuePacked[NODE_SPACE_PTR_ID]
      ? String(valuePacked[NODE_SPACE_PTR_ID][NODE_REFERENCE_ID_KEY])
      : undefined,
    definitionId: valuePacked[NODE_DEFINITION_PTR_ID]
      ? String(valuePacked[NODE_DEFINITION_PTR_ID][NODE_REFERENCE_ID_KEY])
      : undefined,
  });

  let snapshotPtr: NodeReference | null = null;
  const snapshotPtrValue = valuePacked[EVENT_SNAPSHOT_KEY];
  if (snapshotPtrValue !== undefined) {
    snapshotPtr = NodeReference.fromValue(snapshotPtrValue);
  }

  const createdAt = new Date(valuePacked[EVENT_CREATED_AT_KEY]);

  const row = new MemoryEventRow({
    metatype: nodeType,
    id,
    snapshotId: snapshotPtr?.id || null,
    ptr,
    createdAt,
    value: valuePacked,
  });

  return row;
}

/**
 * Unpack a MemoryEventRow to a Value.
 */
export function unpackEventRow(row: MemoryEventRow): Value {
  const typeInfo = new Type({
    cardinality: TypeCardinality.SCALAR,
    scalarType: ScalarType.NODE_VALUE,
    nodeType: row.metatype,
  });

  return new Value({
    type: typeInfo,
    value: row.value,
  });
}

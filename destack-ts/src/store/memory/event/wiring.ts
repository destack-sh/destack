import { NODE_TYPE_SCALAR_BY_TYPE, NodeReference, NodeType, Value } from "@destack/language";
import {
  EVENT_CREATED_AT_KEY,
  EVENT_SNAPSHOT_KEY,
  ENTITY_DEFINITION_PTR_KEY,
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
    definitionId: valuePacked[ENTITY_DEFINITION_PTR_KEY]
      ? String(valuePacked[ENTITY_DEFINITION_PTR_KEY][NODE_REFERENCE_ID_KEY])
      : undefined,
  });

  let snapshotPtr: NodeReference | null = null;
  const snapshotPtrValue = valuePacked[EVENT_SNAPSHOT_KEY];
  if (snapshotPtrValue !== undefined) {
    snapshotPtr = NodeReference.fromCson(snapshotPtrValue);
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
  const type = NODE_TYPE_SCALAR_BY_TYPE[row.metatype];
  return new Value({ type, value: row.value });
}

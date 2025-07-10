import {
  Event,
  IsExtensible,
  IsSpatial,
  Node,
  NodeReference,
  NodeType,
  ScalarType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language";
import { MemoryEventRow } from "@destack/store/memory/event/core";

const NODE_METATYPE_KEY = String(Node.property("metatype").id);
const NODE_ID_KEY = String(Node.property("id").id);
const NODE_PARENT_PTR_KEY = String(Node.property("parent").id);
const NODE_SPACE_PTR_ID = String(IsSpatial.property("space").id);
const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);

const EVENT_CREATED_AT_KEY = String(Event.property("created_at").id);
const EVENT_SNAPSHOT_PTR_KEY = String(Event.property("snapshot").id);

const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);

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
  const snapshotPtrValue = valuePacked[EVENT_SNAPSHOT_PTR_KEY];
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

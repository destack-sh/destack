import {
  Entity,
  IsExtensible,
  IsSpatial,
  Materialization,
  Node,
  NodeReference,
  NodeType,
  ScalarType,
  Type,
  TypeCardinality,
  Value,
} from "@destack/language";
import { MemoryEntityRow } from "@destack/store/memory/entity/core";

const NODE_METATYPE_KEY = String(Node.property("metatype").id);
const NODE_ID_KEY = String(Node.property("id").id);
const NODE_PARENT_PTR_KEY = String(Node.property("parent").id);
const NODE_SPACE_PTR_ID = String(IsSpatial.property("space").id);
const NODE_DEFINITION_PTR_ID = String(IsExtensible.property("definition").id);

const ENTITY_SNAPSHOT_PTR_KEY = String(Entity.property("snapshot").id);
const ENTITY_MATERIALIZATION_KEY = String(Entity.property("materialization").id);

const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);

/**
 * Pack a Value into a MemoryEntityRow.
 */
export function packEntityRow(value: Value): MemoryEntityRow {
  const valuePacked = value.value;
  if (!valuePacked) {
    throw new Error(`no value for ${value.repr()}`);
  }

  const nodeType = Number(valuePacked[NODE_METATYPE_KEY]) as NodeType;
  const id = String(valuePacked[NODE_ID_KEY]);
  const ptr = new NodeReference({
    type: nodeType,
    id: String(valuePacked[NODE_ID_KEY]),
    spaceId:
      NODE_SPACE_PTR_ID in valuePacked
        ? String(valuePacked[NODE_SPACE_PTR_ID][NODE_REFERENCE_ID_KEY])
        : undefined,
    definitionId:
      NODE_DEFINITION_PTR_ID in valuePacked
        ? String(valuePacked[NODE_DEFINITION_PTR_ID][NODE_REFERENCE_ID_KEY])
        : undefined,
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

  const row = new MemoryEntityRow({
    metatype: nodeType,
    id,
    snapshotId: snapshotPtr?.id || null,
    materialization,
    ptr,
    parentPtr,
    value: valuePacked,
  });

  return row;
}

/**
 * Unpack a MemoryEntityRow to a Value.
 */
export function unpackEntityRow(row: MemoryEntityRow): Value {
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

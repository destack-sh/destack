import {
  ENTITY_INDEXED_KEYS,
  ENTITY_PRIMARY_KEY,
  getEntityKey,
  INDEXED_PREFIX,
  NODE_METATYPE_KEY,
} from "@destack-web/store/indexeddb/core";
import {
  NODE_REFERENCE_ID_KEY,
  NODE_TYPE_SCALAR_BY_TYPE,
  NodeReference,
  NodeType,
  StructType,
  Value,
} from "destack";

/**
 * Pack a Value into an IndexedDB entity row.
 */
export function packEntityRow(nodePtr: NodeReference, value: Value): { [key: string]: string } {
  const valuePacked = { ...value.value }; // main data (just copy)

  // primary key
  valuePacked[ENTITY_PRIMARY_KEY] = getEntityKey(nodePtr.id, nodePtr.snapshotId);

  // pack indexed keys
  for (const key of ENTITY_INDEXED_KEYS) {
    const indexedKey = INDEXED_PREFIX + key;
    let indexedValue = valuePacked[key];
    if (indexedValue !== undefined) {
      if (typeof indexedValue == "object") {
        // flatten ptr props into their id
        if (indexedValue[NODE_METATYPE_KEY] != StructType.NODE_REFERENCE) {
          throw new Error(
            `unexpected non-ptr prop: ${key} in ${value.repr()}: ${JSON.stringify(indexedValue)}`,
          );
        }
        indexedValue = indexedValue[NODE_REFERENCE_ID_KEY];
      }
      valuePacked[indexedKey] = indexedValue;
    }
  }
  return valuePacked;
}

/**
 * Unpack an IndexedDB entity row into a Value.
 */
export function unpackEntityRow(valuePacked: { [key: string]: string }): Value {
  const metatype = Number(valuePacked[NODE_METATYPE_KEY]) as NodeType;
  const type = NODE_TYPE_SCALAR_BY_TYPE[metatype];
  const valueClean = { ...valuePacked };
  // clean indexed keys (they're mirrors)
  for (const key of ENTITY_INDEXED_KEYS) {
    const indexedKey = INDEXED_PREFIX + key;
    if (valueClean[indexedKey] !== undefined) {
      delete valueClean[indexedKey];
    }
  }
  const value = new Value({ type, value: valueClean });
  return value;
}

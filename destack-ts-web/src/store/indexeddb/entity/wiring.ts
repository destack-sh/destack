import {
  ENTITY_INDEXED_KEYS,
  INDEXED_PREFIX,
  NODE_METATYPE_KEY,
} from "@destack-web/store/indexeddb/core";
import { NODE_TYPE_SCALAR_BY_TYPE, NodeType, Value } from "destack";

/**
 * Pack a Value into an IndexedDB entity row.
 */
export function packEntityRow(value: Value): { [key: string]: string } {
  const valuePacked = { ...value.value };
  for (const indexedKey in ENTITY_INDEXED_KEYS) {
    valuePacked[INDEXED_PREFIX + indexedKey] = valuePacked[indexedKey];
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
  for (const key in ENTITY_INDEXED_KEYS) {
    const indexedKey = INDEXED_PREFIX + key;
    if (valueClean[indexedKey] !== undefined) {
      delete valueClean[indexedKey];
    }
  }
  const value = new Value({ type, value: valueClean });
  return value;
}

import {
  EVENT_INDEXED_KEYS,
  INDEXED_PREFIX,
  NODE_METATYPE_KEY,
  NODE_REFERENCE_ID_KEY,
  NULL_SENTINEL,
} from "@destack-web/store/indexeddb/core";
import { NODE_TYPE_SCALAR_BY_TYPE, NodeType, StructType, Value } from "destack";

/**
 * Pack a Value into an IndexedDB event row.
 */
export function packEventRow(value: Value): { [key: string]: string } {
  const valuePacked = { ...value.value };
  // pack indexed keys
  for (const key of EVENT_INDEXED_KEYS) {
    const indexedKey = INDEXED_PREFIX + key;
    let indexedValue = valuePacked[key];
    // flatten ptr props into their id
    if (indexedValue !== undefined && typeof indexedValue == "object") {
      if (indexedValue[NODE_METATYPE_KEY] != StructType.NODE_REFERENCE) {
        throw new Error(
          `unexpected non-ptr prop: ${key} in ${value.repr()}: ${JSON.stringify(indexedValue)}`,
        );
      }
      indexedValue = indexedValue[NODE_REFERENCE_ID_KEY];
    }
    valuePacked[indexedKey] = indexedValue ?? NULL_SENTINEL;
  }
  return valuePacked;
}

/**
 * Unpack an IndexedDB event row into a Value.
 */
export function unpackEventRow(valuePacked: { [key: string]: string }): Value {
  const metatype = Number(valuePacked[NODE_METATYPE_KEY]) as NodeType;
  const type = NODE_TYPE_SCALAR_BY_TYPE[metatype];
  const valueClean = { ...valuePacked };
  // clean indexed keys (they're mirrors)
  for (const key of EVENT_INDEXED_KEYS) {
    const indexedKey = INDEXED_PREFIX + key;
    if (valueClean[indexedKey] !== undefined) {
      delete valueClean[indexedKey];
    }
  }
  const value = new Value({ type, value: valueClean });
  return value;
}

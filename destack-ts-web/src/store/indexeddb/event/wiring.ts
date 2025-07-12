import {
  EVENT_KEYS_TO_INDEX,
  EVENT_KEYS_TO_INDEX_PREFIXED,
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
  for (const key of EVENT_KEYS_TO_INDEX) {
    const indexedKey = EVENT_KEYS_TO_INDEX_PREFIXED[key];
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
  
  // clean indexed keys (they're just internal)
  const valueClean = { ...valuePacked };
  for (const key of EVENT_KEYS_TO_INDEX) {
    const indexedKey = EVENT_KEYS_TO_INDEX_PREFIXED[key];
    if (valueClean[indexedKey] !== undefined) {
      delete valueClean[indexedKey];
    }
  }
  
  const value = new Value({ type, value: valueClean });
  return value;
}

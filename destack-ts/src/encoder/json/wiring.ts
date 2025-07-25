import { BuiltinObject } from "@destack/language/core/builtin";
import {
  Encoding,
  type NodeType,
  PrimitiveType,
  ScalarType,
  StructType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import type { PropertyDefinition } from "@destack/language/core/builtin/definition";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { Type } from "@destack/language/core/builtin/type";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Session } from "@destack/language/core/runtime/session";
import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE } from "@destack/language/registry";
import {
  assertNever,
  base64Decode,
  base64Encode,
  timedeltaFromISOFormat,
  timedeltaToISOFormat,
} from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/**
 * Pack a generic typed value to a JSON object.
 */
export function packJson(value: any, type: Type | PropertyDefinition | CustomProperty): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _packScalarJson(value, type);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (!value) {
      return [];
    }
    const packedList: any[] = [];
    for (const item of value) {
      packedList.push(_packScalarJson(item, type));
    }
    return packedList;
  } else if (type.cardinality == TypeCardinality.MAP) {
    if (!value) {
      return {};
    }
    if (type.keyType === null) {
      throw new Error(`no key type for ${type.repr()}`);
    }
    const packedMap: { [key: string]: any } = {};
    for (const [key, val] of Object.entries(value)) {
      const packedKey = _packScalarJson(key, type.keyType);
      const packedVal = _packScalarJson(val, type);
      packedMap[String(packedKey)] = packedVal;
    }
    return packedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/**
 * Unpack a JSON object to a generic typed value.
 */
export function unpackJson(
  value: any,
  type: Type | PropertyDefinition | CustomProperty,
  _session: Session | null,
): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _unpackScalarJson(value, type, _session);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (value === null) {
      return [];
    }
    const unpackedList = [];
    for (const item of value) {
      unpackedList.push(_unpackScalarJson(item, type, _session));
    }
    return unpackedList;
  } else if (type.cardinality == TypeCardinality.MAP) {
    if (value === null) {
      return {};
    }
    const unpackedMap: { [key: string]: any } = {};
    for (const [key, val] of Object.entries(value)) {
      const unpackedKey = type.keyType ? _unpackScalarJson(key, type.keyType, _session) : key;
      const unpackedVal = _unpackScalarJson(val, type, _session);
      unpackedMap[unpackedKey] = unpackedVal;
    }
    return unpackedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/** Pack a scalar value to a JSON object. */
function _packScalarJson(value: any, type: Type | PropertyDefinition | CustomProperty): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType === null) {
      throw new Error(`missing primitive type for ${type.repr()}`);
    } else if (type.primitiveType == PrimitiveType.BOOLEAN) {
      return value;
    } else if (type.primitiveType == PrimitiveType.STRING) {
      return value;
    } else if (type.primitiveType == PrimitiveType.UUID) {
      return value;
    } else if (type.primitiveType == PrimitiveType.BYTES) {
      return base64Encode(value as Uint8Array);
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return (value as Temporal.ZonedDateTime).toString({ timeZoneName: "never" });
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return (value as Temporal.PlainDate).toString();
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return (value as Temporal.PlainTime).toString();
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaToISOFormat(value as Temporal.Duration);
    } else if (
      type.primitiveType == PrimitiveType.SINT8 ||
      type.primitiveType == PrimitiveType.SINT16 ||
      type.primitiveType == PrimitiveType.SINT32 ||
      type.primitiveType == PrimitiveType.SINT64 ||
      type.primitiveType == PrimitiveType.SINT128 ||
      type.primitiveType == PrimitiveType.UINT8 ||
      type.primitiveType == PrimitiveType.UINT16 ||
      type.primitiveType == PrimitiveType.UINT32 ||
      type.primitiveType == PrimitiveType.UINT64 ||
      type.primitiveType == PrimitiveType.UINT128 ||
      type.primitiveType == PrimitiveType.FLOAT16 ||
      type.primitiveType == PrimitiveType.FLOAT32 ||
      type.primitiveType == PrimitiveType.FLOAT64
    ) {
      return value;
    } else if (type.primitiveType == PrimitiveType.JSON) {
      return value;
    } else {
      assertNever(type.primitiveType);
    }
  } else if (type.scalarType == ScalarType.ENUM) {
    return value;
  } else if (
    type.scalarType == ScalarType.NODE_REFERENCE ||
    type.scalarType == ScalarType.NODE_VALUE ||
    type.scalarType == ScalarType.STRUCT
  ) {
    if (!(value instanceof BuiltinObject)) {
      throw new Error(
        `expected BuiltinObject for ${type.repr()}, got ${value.constructor.name}: ${value}`,
      );
    }
    return (value as BuiltinObject).pack(Encoding.JSON);
  } else {
    assertNever(type.scalarType);
  }
}

/** Unpack a JSON object to a scalar value. */
function _unpackScalarJson(
  value: any,
  type: Type | PropertyDefinition | CustomProperty,
  _session: Session | null,
): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType === null) {
      throw new Error(`missing primitive type for ${type.repr()}`);
    } else if (type.primitiveType == PrimitiveType.BOOLEAN) {
      return value;
    } else if (type.primitiveType == PrimitiveType.BYTES) {
      return base64Decode(value);
    } else if (type.primitiveType == PrimitiveType.UUID) {
      return value;
    } else if (type.primitiveType == PrimitiveType.STRING) {
      return value;
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return Temporal.Instant.from(value).toZonedDateTimeISO("UTC");
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return Temporal.PlainDate.from(value);
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return Temporal.PlainTime.from(value);
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaFromISOFormat(value);
    } else if (
      type.primitiveType == PrimitiveType.FLOAT16 ||
      type.primitiveType == PrimitiveType.FLOAT32 ||
      type.primitiveType == PrimitiveType.FLOAT64
    ) {
      return Number(value);
    } else if (
      type.primitiveType == PrimitiveType.SINT8 ||
      type.primitiveType == PrimitiveType.SINT16 ||
      type.primitiveType == PrimitiveType.SINT32 ||
      type.primitiveType == PrimitiveType.SINT64 ||
      type.primitiveType == PrimitiveType.SINT128 ||
      type.primitiveType == PrimitiveType.UINT8 ||
      type.primitiveType == PrimitiveType.UINT16 ||
      type.primitiveType == PrimitiveType.UINT32 ||
      type.primitiveType == PrimitiveType.UINT64 ||
      type.primitiveType == PrimitiveType.UINT128
    ) {
      return Number(value);
    } else if (type.primitiveType == PrimitiveType.JSON) {
      return value;
    } else {
      assertNever(type.primitiveType);
    }
  } else if (type.scalarType == ScalarType.ENUM) {
    return value;
  } else if (type.scalarType == ScalarType.NODE_REFERENCE) {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return _NodeReference.unpack(Encoding.JSON, value, _session);
  } else if (type.scalarType == ScalarType.NODE_VALUE) {
    const nodeType = Number(value["type"]) as NodeType;
    const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
    return nodeClass.unpack(Encoding.JSON, value, _session);
  } else if (type.scalarType == ScalarType.STRUCT) {
    if (type.structType === null) {
      throw new Error(`missing struct type for ${type.repr()}`);
    }
    const structClass = STRUCT_CLASS_BY_TYPE[type.structType];
    return structClass.unpack(Encoding.JSON, value, _session);
  } else {
    assertNever(type.scalarType);
  }
}

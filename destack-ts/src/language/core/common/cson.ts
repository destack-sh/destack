import { BuiltinObject } from "@destack/language/core/builtin";
import {
  NodeType,
  PrimitiveType,
  ScalarType,
  StructType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { PropertyDefinition } from "@destack/language/core/common/definition";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Type } from "@destack/language/core/common/type";
import type { Supergraph } from "@destack/language/core/runtime/graph";
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
 * Pack a generic typed value to a CSON object.
 */
export function packCson(value: any, type: Type | PropertyDefinition | CustomProperty): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _packScalarCson(value, type);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (!value) {
      return [];
    }
    const packedList: any[] = [];
    for (const item of value) {
      packedList.push(_packScalarCson(item, type));
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
      const packedKey = _packScalarCson(key, type.keyType);
      const packedVal = _packScalarCson(val, type);
      packedMap[String(packedKey)] = packedVal;
    }
    return packedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/**
 * Unpack a CSON object to a generic typed value.
 */
export function unpackCson(
  value: any,
  type: Type | PropertyDefinition | CustomProperty,
  options?: {
    _session?: Session | null;
    _graph?: any | null;
    _supergraph?: Supergraph | null;
    _connection?: any | null;
  },
): any {
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _unpackScalarCson(value, type, options);
  } else if (type.cardinality == TypeCardinality.LIST) {
    if (value === null) {
      return [];
    }
    const unpackedList = [];
    for (const item of value) {
      unpackedList.push(_unpackScalarCson(item, type, options));
    }
    return unpackedList;
  } else if (type.cardinality == TypeCardinality.MAP) {
    if (value === null) {
      return {};
    }
    const unpackedMap: { [key: string]: any } = {};
    for (const [key, val] of Object.entries(value)) {
      const unpackedKey = type.keyType ? _unpackScalarCson(key, type.keyType) : key;
      const unpackedVal = _unpackScalarCson(val, type, options);
      unpackedMap[unpackedKey] = unpackedVal;
    }
    return unpackedMap;
  } else {
    assertNever(type.cardinality);
  }
}

/** Pack a scalar value to a CSON object. */
function _packScalarCson(value: any, type: Type | PropertyDefinition | CustomProperty): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return base64Encode(value as Uint8Array);
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return (value as Temporal.ZonedDateTime).toString({ timeZoneName: "never" });
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return (value as Temporal.PlainDate).toString();
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return (value as Temporal.PlainTime).toString();
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaToISOFormat(value as Temporal.Duration);
    } else {
      return value;
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
    return (value as BuiltinObject).toCson();
  } else {
    assertNever(type.scalarType);
  }
}

/** Unpack a CSON object to a scalar value. */
function _unpackScalarCson(
  value: any,
  type: Type | PropertyDefinition | CustomProperty,
  options?: {
    _session?: Session | null;
    _graph?: any | null;
    _supergraph?: Supergraph | null;
    _connection?: any | null;
  },
): any {
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return base64Decode(value);
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return Temporal.Instant.from(value).toZonedDateTimeISO("UTC");
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return Temporal.PlainDate.from(value);
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return Temporal.PlainTime.from(value);
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaFromISOFormat(value);
    } else if (
      type.primitiveType == PrimitiveType.FLOAT32 ||
      type.primitiveType == PrimitiveType.FLOAT64
    ) {
      return Number(value);
    } else if (
      type.primitiveType == PrimitiveType.INT16 ||
      type.primitiveType == PrimitiveType.INT32 ||
      type.primitiveType == PrimitiveType.INT64
    ) {
      return Number(value);
    } else {
      return value;
    }
  } else if (type.scalarType == ScalarType.ENUM) {
    return value;
  } else if (type.scalarType == ScalarType.NODE_REFERENCE) {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return _NodeReference.fromCson(
      value,
      options?._session,
      options?._supergraph,
      options?._graph,
      options?._connection,
    );
  } else if (type.scalarType == ScalarType.NODE_VALUE) {
    const nodeType = Number(value["1"]) as NodeType;
    const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
    return nodeClass.__unpackCson__(
      value,
      options?._session,
      options?._supergraph,
      options?._graph,
      options?._connection,
    );
  } else if (type.scalarType == ScalarType.STRUCT) {
    if (type.structType === null) {
      throw new Error(`missing struct type for ${type.repr()}`);
    }
    const structClass = STRUCT_CLASS_BY_TYPE[type.structType];
    return structClass.__unpackCson__(
      value,
      options?._session,
      options?._supergraph,
      options?._graph,
      options?._connection,
    );
  } else {
    assertNever(type.scalarType);
  }
}

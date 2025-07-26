import { BuiltinObject, type NodeType, StructType } from "@destack/language/core/builtin";
import {
  Encoding,
  PrimitiveType,
  ScalarType,
  TypeCardinality,
} from "@destack/language/core/builtin/common";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { Type } from "@destack/language/core/builtin/type";
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
export function packCson(value: any, type: Type): any {
  // scalar
  if (type.cardinality == TypeCardinality.SCALAR) {
    if (value == null) {
      return null;
    } else {
      return _packScalarCson(value, type);
    }
  }

  // list
  else if (type.cardinality == TypeCardinality.LIST) {
    if (value == null) {
      return null;
    } else {
      if (type.valueType === null) {
        throw new Error(`no value type for ${type.repr()}`);
      }
      const packedList: any[] = [];
      for (const item of value) {
        packedList.push(_packScalarCson(item, type.valueType));
      }
      return packedList;
    }
  }

  // tuple
  else if (type.cardinality == TypeCardinality.TUPLE) {
    if (value == null) {
      return null;
    } else {
      if (type.elementTypes === null) {
        throw new Error(`no element types for ${type.repr()}`);
      }
      const packedTuple: any[] = [];
      for (let i = 0; i < value.length; i++) {
        packedTuple.push(_packScalarCson(value[i], type.elementTypes![i]));
      }
      return packedTuple;
    }
  }

  // map
  else if (type.cardinality == TypeCardinality.MAP) {
    if (value == null) {
      return null;
    } else {
      if (type.keyType === null) {
        throw new Error(`no key type for ${type.repr()}`);
      } else if (type.valueType === null) {
        throw new Error(`no value type for ${type.repr()}`);
      }
      const packedMap: { [key: string]: any } = {};
      for (const [key, val] of Object.entries(value)) {
        const packedKey = _packScalarCson(key, type.keyType);
        const packedVal = _packScalarCson(val, type.valueType);
        packedMap[String(packedKey)] = packedVal;
      }
      return packedMap;
    }
  }

  //
  else {
    assertNever(type.cardinality);
  }
}

/**
 * Unpack a CSON object to a generic typed value.
 */
export function unpackCson(
  value: any,
  type: Type,
  options?: {
    _session?: Session | null;
    _graph?: any | null;
    _connection?: any | null;
  },
): any {
  // scalar
  if (type.cardinality == TypeCardinality.SCALAR) {
    return _unpackScalarCson(value, type, options);
  }

  // list
  else if (type.cardinality == TypeCardinality.LIST) {
    if (value == null) {
      return null;
    } else {
      if (type.valueType === null) {
        throw new Error(`no value type for ${type.repr()}`);
      }
      const unpackedList = [];
      for (const item of value) {
        unpackedList.push(_unpackScalarCson(item, type.valueType, options));
      }
      return unpackedList;
    }
  }

  // tuple
  else if (type.cardinality == TypeCardinality.TUPLE) {
    if (value == null) {
      return null;
    } else {
      if (type.elementTypes === null) {
        throw new Error(`no element types for ${type.repr()}`);
      }
      const unpackedTuple: any[] = [];
      for (let i = 0; i < value.length; i++) {
        unpackedTuple.push(_unpackScalarCson(value[i], type.elementTypes![i], options));
      }
      return unpackedTuple;
    }
  }

  // map
  else if (type.cardinality == TypeCardinality.MAP) {
    if (value == null) {
      return null;
    } else {
      if (type.keyType === null) {
        throw new Error(`no key type for ${type.repr()}`);
      } else if (type.valueType === null) {
        throw new Error(`no value type for ${type.repr()}`);
      }
      const unpackedMap: { [key: string]: any } = {};
      for (const [key, val] of Object.entries(value)) {
        const unpackedKey = _unpackScalarCson(key, type.keyType, options);
        const unpackedVal = _unpackScalarCson(val, type.valueType, options);
        unpackedMap[unpackedKey] = unpackedVal;
      }
      return unpackedMap;
    }
  }

  //
  else {
    assertNever(type.cardinality);
  }
}

/** Pack a scalar value to a CSON object. */
function _packScalarCson(value: any, type: Type): any {
  if (type.cardinality != TypeCardinality.SCALAR || type.scalarType == null) {
    throw new Error(`expected scalar type, got ${type.repr()}`);
  }

  // primitive
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType === null) {
      throw new Error(`missing primitive type for ${type.repr()}`);
    } else if (type.primitiveType == PrimitiveType.NONE) {
      return null;
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
      type.primitiveType == PrimitiveType.INT8 ||
      type.primitiveType == PrimitiveType.INT16 ||
      type.primitiveType == PrimitiveType.INT32 ||
      type.primitiveType == PrimitiveType.INT64 ||
      type.primitiveType == PrimitiveType.INT128 ||
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
  }

  // enum
  else if (type.scalarType == ScalarType.ENUM) {
    return value;
  }

  // node reference, node value, struct
  else if (
    type.scalarType == ScalarType.NODE_REFERENCE ||
    type.scalarType == ScalarType.NODE_VALUE ||
    type.scalarType == ScalarType.STRUCT
  ) {
    if (!(value instanceof BuiltinObject)) {
      throw new Error(
        `expected BuiltinObject for ${type.repr()}, got ${value.constructor.name}: ${value}`,
      );
    }
    return (value as BuiltinObject).pack(Encoding.CSON);
  }

  // literal
  else if (type.scalarType == ScalarType.LITERAL) {
    throw new Error(`cannot pack literal: ${type.repr()}`);
  }

  // union
  else if (type.scalarType == ScalarType.UNION) {
    throw new Error(`cannot pack union: ${type.repr()}`);
  }

  //
  else {
    assertNever(type.scalarType);
  }
}

/** Unpack a CSON object to a scalar value. */
function _unpackScalarCson(
  value: any,
  type: Type,
  options?: {
    _session?: Session | null;
    _graph?: any | null;
    _connection?: any | null;
  },
): any {
  if (type.cardinality != TypeCardinality.SCALAR || type.scalarType == null) {
    throw new Error(`expected scalar type, got ${type.repr()}`);
  }

  // primitive
  if (type.scalarType == ScalarType.PRIMITIVE) {
    if (type.primitiveType === null) {
      throw new Error(`missing primitive type for ${type.repr()}`);
    } else if (type.primitiveType == PrimitiveType.NONE) {
      return null;
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
      type.primitiveType == PrimitiveType.INT8 ||
      type.primitiveType == PrimitiveType.INT16 ||
      type.primitiveType == PrimitiveType.INT32 ||
      type.primitiveType == PrimitiveType.INT64 ||
      type.primitiveType == PrimitiveType.INT128 ||
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
  }

  // enum
  else if (type.scalarType == ScalarType.ENUM) {
    return value;
  }

  // node reference
  else if (type.scalarType == ScalarType.NODE_REFERENCE) {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return _NodeReference.unpack(Encoding.CSON, value, options?._session ?? null);
  }

  // node value
  else if (type.scalarType == ScalarType.NODE_VALUE) {
    const nodeType = Number(value["1"]) as NodeType;
    const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
    return nodeClass.unpack(Encoding.CSON, value, options?._session ?? null);
  }

  // struct
  else if (type.scalarType == ScalarType.STRUCT) {
    if (type.structType === null) {
      throw new Error(`missing struct type for ${type.repr()}`);
    }
    const structClass = STRUCT_CLASS_BY_TYPE[type.structType];
    return structClass.unpack(Encoding.CSON, value, options?._session ?? null);
  }

  // literal
  else if (type.scalarType == ScalarType.LITERAL) {
    throw new Error(`cannot unpack literal: ${type.repr()}`);
  }

  // union
  else if (type.scalarType == ScalarType.UNION) {
    throw new Error(`cannot unpack union: ${type.repr()}`);
  }

  //
  else {
    assertNever(type.scalarType);
  }
}

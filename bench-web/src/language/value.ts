import { FLOAT_EPSILON } from "@/language/const";
import {
  describeTypeIdentity,
  encodeTypeIdentity,
  getPropertyType,
  getStorageKey,
  resolveFields,
  resolveType,
  type TypeIdentity,
} from "@/language/field";
import type { ReadNodeGraph } from "@/language/graph";
import {
  NodeReferenceData,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PrimitiveType,
  Struct as ProtoStruct,
  TYPE_CONSTRAINT_BY_FORMAT,
  Timestamp,
  TypeConstraintIn,
  TypeKind,
  type AnyNodeData,
  type AnyStructData,
  type AnyTypeMapping,
} from "@/proto/wire";
import {
  isNodeRef,
  isProtoJson,
  isStruct,
  toNodeRefOneOf,
  unwrapProtoOneOf,
  type SomeNodeReferenceData,
} from "@/proto/wiring";

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | { [key: string]: JsonValue } | JsonValue[];
export type PrimitiveValue = JsonPrimitive | bigint | Timestamp;
export type ScalarValue = PrimitiveValue | ProtoStruct | AnyStructData | AnyNodeData;
export type SomeValue = ScalarValue | SomeValue[] | { [key: string]: SomeValue };

//
// Packing/unpacking
//

// TODO :Architecture :Performance: encode/decode protoStruct/Json in „connections (at the fetch/commit boundary) :ProtoStructMapping
//  Could either fork protobuf-ts or just switch to ts-proto?

/** Packs a single data value in its robust JSON-able representation. */
function packValueScalar(value: ScalarValue, type: TypeIdentity): JsonValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.DATETIME) {
      return Timestamp.toDate(value as Timestamp).toISOString();
    } else if (typeof value == "bigint") {
      // NOTE :Robustness: we pack bigints as numbers, which is only safe up to 2^53-1
      //  (should be fine, we only use it for epoch/revision which will last ~300k years at 1000edits/sec)
      if (value > Number.MAX_SAFE_INTEGER) {
        throw new Error(`bigint ${value} too large for Number for ${describeTypeIdentity(type)}`);
      }
      return Number(value);
    } else if (type.primitiveType == PrimitiveType.JSON) {
      // auto-unpack proto json :ProtoStructMapping
      if (isProtoJson(value)) {
        return ProtoStruct.toJson(value);
      } else {
        return value as JsonValue;
      }
    } else {
      return value as JsonPrimitive;
    }
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    if (isProtoJson(value)) {
      // shortcut if already packed :ProtoStructMapping
      return ProtoStruct.toJson(value);
    } else if (!isNodeRef(value)) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    } else {
      return packBuiltinObject(value as NodeReferenceData);
    }
  } else if (type.kind == TypeKind.ENUM) {
    return value as JsonPrimitive;
  } else if (type.kind == TypeKind.STRUCT) {
    if (isProtoJson(value)) {
      // shortcut if already packed :ProtoStructMapping
      return ProtoStruct.toJson(value);
    } else if (!isStruct(value)) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    } else {
      return packBuiltinObject(value);
    }
  } else {
    throw new Error(`cannot pack value of type ${describeTypeIdentity(type)}`);
  }
}

/** Unpacks a single value into its data representation (except for JSON, which remains as is for value objects). */
function unpackValueScalar(valuePacked: JsonValue, type: TypeIdentity): ScalarValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.DATETIME) {
      return Timestamp.fromDate(new Date(valuePacked as string));
    } else if (type.primitiveType == PrimitiveType.INT64) {
      // see above
      return BigInt(valuePacked as number);
    } else {
      return valuePacked as PrimitiveValue;
    }
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    if (typeof valuePacked !== "object") {
      throw new Error(`unexpected value ${JSON.stringify(valuePacked)} for type ${describeTypeIdentity(type)}`);
    }
    return unpackBuiltinObject(valuePacked as unknown as NodeReferenceData);
  } else if (type.kind == TypeKind.ENUM) {
    return valuePacked as PrimitiveValue;
  } else if (type.kind == TypeKind.STRUCT) {
    if (typeof valuePacked !== "object") {
      throw new Error(`unexpected value ${JSON.stringify(valuePacked)} for type ${describeTypeIdentity(type)}`);
    }
    return unpackBuiltinObject(valuePacked as unknown as AnyStructData);
  } else {
    throw new Error(`cannot unpack value of type ${describeTypeIdentity(type)}`);
  }
}

/** Unpacks a single value into its data representation (converting JSON into ProtoJson ƒor builtin objects).  */
function unpackValueScalarData(valuePacked: JsonValue, type: TypeIdentity): ScalarValue {
  let value = unpackValueScalar(valuePacked, type);
  if (type.kind == TypeKind.PRIMITIVE) {
    // unpack
    if (type.primitiveType == PrimitiveType.JSON) {
      value = ProtoStruct.fromJson(value as JsonValue);
    }
  }
  return value;
}

/** Packs a single struct/node proto value using proto ids for keys and enums. */
export function packBuiltinObject(
  value: AnyStructData | AnyNodeData,
  options?: { only?: string[] },
): Record<string, any> {
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[value.metatype];
  const properties = PROPERTY_INFOS_BY_TYPE[value.metatype];
  if (propertyEnum == null || properties == null) throw new Error(`unexpected object type ${value.metatype}`);

  const valuePacked: Record<string, any> = {};
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propType = getPropertyType(prop);
    let propValue = (value as any)[propName];
    let propValuePacked;
    if (propValue == null || (prop.isList && propValue.length == 0)) {
      continue;
    } else if (prop.isList) {
      propValuePacked = propValue.map((v: any) => packValueScalar(v, propType));
    } else {
      if (prop.referenceIsRich) {
        // :RichReferences
        propValue = unwrapProtoOneOf(propValue);
        if (propValue == null) continue; // one-of fields are always nullable
      }
      propValuePacked = packValueScalar(propValue, propType);
    }
    valuePacked[prop.id.toString()] = propValuePacked;
  }

  return valuePacked;
}

/** Convenience wrapper around packBuiltinObject and packProtoJson */
export function packBuiltinObjectJson(value: AnyStructData | AnyNodeData, options?: { only?: string[] }): ProtoStruct {
  return ProtoStruct.fromJson(packBuiltinObject(value, options));
}

/** Decodes proto value representation of a struct. See encode. */
export function unpackBuiltinObject<T extends ObjectType>(valuePacked: any, objectType?: T): AnyTypeMapping[T] {
  if (objectType == null) {
    if (valuePacked["1"] == null) throw new Error(`missing object type in ${JSON.stringify(valuePacked)}`);
    objectType = valuePacked["1"] as T;
  }
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[objectType];
  const properties = PROPERTY_INFOS_BY_TYPE[objectType];
  if (propertyEnum == null || properties == null) throw new Error(`unexpected object type ${objectType}`);

  const value = {} as AnyTypeMapping[T];
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propType = getPropertyType(prop);
    const propValuePacked = valuePacked[prop.id.toString()];
    let propValue;
    if (prop.isList) {
      if (propValuePacked == null) {
        propValue = [];
      } else {
        propValue = propValuePacked.map((v: any) => unpackValueScalarData(v, propType));
      }
    } else {
      if (propValuePacked == null) {
        if (prop.referenceIsRich) {
          propValue = { oneofKind: undefined };
        } else if (!prop.isRequired) {
          continue;
        } else {
          propValue = null;
        }
      } else {
        propValue = unpackValueScalarData(propValuePacked, propType);
        if (prop.referenceIsRich) {
          // :RichReferences
          propValue = toNodeRefOneOf(propValue as SomeNodeReferenceData);
        }
      }
    }
    (value as any)[propName] = propValue;
  }
  value.metatype = objectType;
  return value;
}

// TODO :Test!: figure out how to test value packing on bench-web properly (ensure it's in sync with bench)

/** Packs a single object value into a packed & secret packed value. */
function packValueObject(
  value: ScalarValue,
  type: TypeIdentity,
  options: { graph: ReadNodeGraph; recurseValueObject: boolean },
): JsonValue {
  const fields = resolveFields(type, options.graph);
  const valuePacked: { [key: string]: JsonValue } = {};
  for (const field of fields) {
    const fieldType = resolveType(field, options.graph);
    const fieldStorageKey = getStorageKey(field, fieldType);
    const fieldValue = (value as any)[fieldStorageKey];
    if (fieldValue == null) {
      continue;
    } else if (fieldType.kind == TypeKind.OBJECT) {
      if (options.recurseValueObject) {
        valuePacked[fieldStorageKey] = packValue(fieldValue, fieldType, {
          graph: options.graph,
          wrapScalar: false,
          recurseValueObject: options.recurseValueObject,
        });
      } else {
        valuePacked[fieldStorageKey] = fieldValue; // keep packed as is
      }
    } else if (!fieldType.isList) {
      valuePacked[fieldStorageKey] = packValueScalar(fieldValue, fieldType);
    } else {
      valuePacked[fieldStorageKey] = fieldValue.map((v: any) => packValueScalar(v, fieldType));
    }
  }
  return valuePacked;
}

/** Unpacks a single packed & secret packed value into an object. */
function unpackValueObject(
  valuePacked: JsonValue,
  type: TypeIdentity,
  options: { graph: ReadNodeGraph; recurseValueObject: boolean },
): SomeValue {
  const fields = resolveFields(type, options.graph);
  const value: { [key: string]: SomeValue } = {};
  for (const field of fields) {
    const fieldType = resolveType(field, options.graph);
    const fieldStorageKey = getStorageKey(field, fieldType);
    const fieldValuePacked = (valuePacked as any)[fieldStorageKey];
    if (fieldValuePacked == null) {
      continue;
    } else if (fieldType.kind == TypeKind.OBJECT) {
      if (options.recurseValueObject) {
        const fieldValue = unpackValue(fieldValuePacked, fieldType, {
          graph: options.graph,
          unwrapScalar: false,
          recurseValueObject: options.recurseValueObject,
        });
        if (fieldValue != null) {
          value[fieldStorageKey] = fieldValue;
        }
      } else {
        value[fieldStorageKey] = fieldValuePacked; // keep packed as is
      }
    } else if (!fieldType.isList) {
      value[fieldStorageKey] = unpackValueScalar(fieldValuePacked, fieldType);
    } else {
      value[fieldStorageKey] = fieldValuePacked.map((v: any) => unpackValueScalar(v, fieldType));
    }
  }
  return value;
}

/**
 * Pack the value data into JSON wire format.
 * Graph is required if we're dealing with an alias or any object type.
 * If previous is passed, old values with different types will be retained.
 * */
export function packValue(
  value: any,
  type: TypeIdentity,
  options: { graph?: ReadNodeGraph; wrapScalar: boolean; recurseValueObject: boolean } = {
    wrapScalar: true,
    recurseValueObject: true,
  },
  previous?: JsonValue,
): JsonValue {
  if (type.kind == TypeKind.ALIAS) {
    if (options.graph == null) throw new Error(`missing graph to resolve ${describeTypeIdentity(type)}`);
    type = resolveType(type, options.graph);
  }
  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested value object
    if (options.graph == null) throw new Error(`missing graph to pack object type ${describeTypeIdentity(type)}`);
    if (value == null) {
      return null;
    } else if (!type.isList) {
      return packValueObject(value, type, { graph: options.graph, recurseValueObject: options.recurseValueObject });
    } else {
      const valuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = packValueObject(value[i], type, {
          graph: options.graph,
          recurseValueObject: options.recurseValueObject,
        });
        valuePacked.push(packed);
      }
      return valuePacked;
    }
  } else {
    // scalar
    let valuePacked;
    if (value == null) {
      valuePacked = null;
    } else if (!type.isList) {
      valuePacked = packValueScalar(value, type);
    } else {
      valuePacked = value.map((v: any) => packValueScalar(v, type));
    }
    if (options.wrapScalar) {
      valuePacked = { [encodeTypeIdentity(type)]: valuePacked };
    }
    return valuePacked;
  }
}

/**
 * Unpack the value data from JSON wire format.
 * Graph is required if we're dealing with an alias or any object type.
 */
export function unpackValue(
  valuePacked: JsonValue,
  type: TypeIdentity,
  options: { graph?: ReadNodeGraph; unwrapScalar: boolean; recurseValueObject: boolean } = {
    unwrapScalar: true,
    recurseValueObject: true,
  },
): any {
  if (type.kind == TypeKind.ALIAS) {
    if (options.graph == null) throw new Error(`missing graph to resolve ${describeTypeIdentity(type)}`);
    type = resolveType(type, options.graph);
  }

  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested value object
    if (options.graph == null) {
      throw new Error(
        `missing graph to unpack object type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`,
      );
    }
    if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackValueObject(valuePacked, type, {
        graph: options.graph,
        recurseValueObject: options.recurseValueObject,
      });
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`);
      }
      return valuePacked!.map((v: any, i: number) =>
        unpackValueObject(v, type, { graph: options.graph!, recurseValueObject: options.recurseValueObject }),
      );
    }
  } else {
    // scalar
    if (valuePacked == null) {
      return null;
    }
    let value;
    if (options?.unwrapScalar) {
      if (typeof valuePacked !== "object" || Array.isArray(valuePacked)) {
        throw new Error(
          `expected object for scalar type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`,
        );
      }
      value = valuePacked[encodeTypeIdentity(type)];
    } else {
      value = valuePacked;
    }
    if (value == null) {
      return null;
    } else if (!type.isList) {
      return unpackValueScalar(value, type);
    } else {
      if (!Array.isArray(value)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}`);
      }
      return value.map((v: any) => unpackValueScalar(v, type));
    }
  }
}

//
// Type checking :TypeChecking
// NOTE :Architecture :Cleanup: run type-checking via :BenchWebRuntime if available?
//

type ValidationHandler = (value: any, message: string, type: TypeIdentity) => void;

/** Checks a scalar value against the given type constraint. :TypeChecking */
export function checkValueScalarConstraint(
  value: any,
  type: TypeIdentity,
  constraint: TypeConstraintIn,
  invalid: ValidationHandler,
): void {
  if (typeof value == "bigint") value = Number(value);
  if (typeof value == "number") {
    if (constraint.minValue != null && value < constraint.minValue) {
      invalid(value, "too small", type);
    }
    if (constraint.maxValue != null && value > constraint.maxValue) {
      invalid(value, "too large", type);
    }
    if (constraint.stepValue != null && Math.abs(value % constraint.stepValue) > FLOAT_EPSILON) {
      invalid(value, `not a multiple ${constraint.stepValue}`, type);
    }
  } else if (typeof value == "string") {
    if (constraint.minLength != null && value.length < constraint.minLength) {
      invalid(value, "too short", type);
    }
    if (constraint.maxLength != null && value.length > constraint.maxLength) {
      invalid(value, "too long", type);
    }
    if (constraint.regex != null && !new RegExp(constraint.regex, "u").test(value)) {
      invalid(value, `does not match regex`, type);
    }
    if (constraint.startsWith != null && !value.startsWith(constraint.startsWith)) {
      invalid(value, `does not start with ${constraint.startsWith}`, type);
    }
    if (constraint.endsWith != null && !value.endsWith(constraint.endsWith)) {
      invalid(value, `does not end with ${constraint.endsWith}`, type);
    }
  }
}

export function checkValueScalar(value: any, type: TypeIdentity, invalid: ValidationHandler): void {
  if (type.kind == TypeKind.PRIMITIVE) {
    if (type.constraint != null) {
      checkValueScalarConstraint(value, type, type.constraint, invalid);
    }
    if (type.format != null && TYPE_CONSTRAINT_BY_FORMAT[type.format] != null) {
      checkValueScalarConstraint(value, type, TYPE_CONSTRAINT_BY_FORMAT[type.format]!, invalid);
    }
    if (typeof value == "string" && value.length == 0) {
      invalid(value, "empty string", type);
    }
  } else {
    // NOTE :Incomplete: checkValueScalar for Node/Struct/Enum
  }
}

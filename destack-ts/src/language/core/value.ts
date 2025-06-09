import { FLOAT_EPSILON, isNodeType } from "@/language/core/const";
import {
  CK_LENGTH_B64,
  decodeTypeIdentity,
  describeTypeIdentity,
  encodeTypeIdentity,
  getPropertyType,
  type TypeIdentity,
} from "@/language/core/type";
import {
  DateTime,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PrimitiveType,
  PropertyInfo,
  Date as ProtoDate,
  TYPE_CONSTRAINT_BY_FORMAT,
  TimeOfDay,
  Timestamp,
  TypeConstraintIn,
  TypeKind,
  type AnyNodeData,
  type AnyStructData,
  type AnyTypeMapping,
} from "@/proto/wire";
import { Duration } from "@/proto/wire/google/protobuf/duration";
import { isNodeRef, isStruct } from "@/proto/wiring";
import {
  timeOfDayFromISOFormat,
  timeOfDayToISOFormat,
  timedeltaFromISOFormat,
  timedeltaToISOFormat,
} from "@/utils/time";

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | { [key: string]: JsonValue } | JsonValue[];
export type PrimitiveValue =
  | JsonPrimitive
  | JsonValue
  | bigint
  | Timestamp
  | DateTime
  | ProtoDate
  | TimeOfDay
  | Duration;
export type ScalarValue = PrimitiveValue | AnyStructData | AnyNodeData;
export type SomeValue = ScalarValue | SomeValue[] | { [key: string]: SomeValue };

//
// Packing/unpacking
//

/** Packs a single data value in its robust JSON-able representation. */
function packValueScalar(value: ScalarValue, type: TypeIdentity): JsonValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return Buffer.from(value as string).toString("base64");
    } else if (type.primitiveType == PrimitiveType.JSON) {
      return value as JsonValue;
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return Timestamp.toDate(value as Timestamp).toISOString();
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return ProtoDate.toJsDate(value as ProtoDate).toISOString();
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return timeOfDayToISOFormat(value as TimeOfDay);
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaToISOFormat(value as Duration);
    } else if (typeof value == "bigint") {
      // NOTE :Cleanup: we pack bigints as numbers, which is only safe up to 2^53-1
      //  (should be fine, we only use it for epoch which will last ~300k years at 1000edits/sec)
      if (value > Number.MAX_SAFE_INTEGER) {
        throw new Error(`bigint ${value} too large for Number for ${describeTypeIdentity(type)}`);
      }
      return Number(value);
    } else {
      return value as JsonPrimitive;
    }
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    if (!isNodeRef(value)) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    }
    return packBuiltinObject(value as NodeReferenceData);
  } else if (type.kind == TypeKind.ENUM) {
    return value as JsonPrimitive;
  } else if (type.kind == TypeKind.STRUCT) {
    if (!isStruct(value)) {
      throw new Error(`unexpected value ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    }
    return packBuiltinObject(value);
  } else {
    throw new Error(`cannot pack value of type ${describeTypeIdentity(type)}`);
  }
}

/** Unpacks a single value into its data representation (except for JSON, which remains as is for custom objects). */
function unpackValueScalar(valuePacked: JsonValue, type: TypeIdentity): ScalarValue {
  if (type.kind == TypeKind.PRIMITIVE) {
    if (type.primitiveType == PrimitiveType.BYTES) {
      return Buffer.from(valuePacked as string, "base64").toString("utf8");
    } else if (type.primitiveType == PrimitiveType.JSON) {
      return valuePacked as JsonValue;
    } else if (type.primitiveType == PrimitiveType.DATETIME) {
      return Timestamp.fromDate(new Date(valuePacked as string));
    } else if (type.primitiveType == PrimitiveType.DATE) {
      return ProtoDate.fromJsDate(new Date(valuePacked as string));
    } else if (type.primitiveType == PrimitiveType.TIME) {
      return timeOfDayFromISOFormat(valuePacked as string);
    } else if (type.primitiveType == PrimitiveType.DURATION) {
      return timedeltaFromISOFormat(valuePacked as string);
    } else if (type.primitiveType == PrimitiveType.INT64) {
      // see packing above
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

/** Pack a builtin object property */
export function packBuiltinObjectProperty(propValue: any, prop: PropertyInfo) {
  const propType = getPropertyType(prop);
  if (propValue == null || (prop.isList && propValue.length == 0)) {
    return undefined;
  } else {
    return packValue(propValue, propType);
  }
}

/** Packs a single struct/node proto value using proto ids for keys and enums. */
export function packBuiltinObject(value: AnyStructData | AnyNodeData): Record<string, any> {
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[value.metatype];
  const properties = PROPERTY_INFOS_BY_TYPE[value.metatype];
  if (propertyEnum == null || properties == null) throw new Error(`unexpected object type ${value.metatype}`);

  const valuePacked: Record<string, any> = {};
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propValue = (value as any)[propName];
    const propValuePacked = packBuiltinObjectProperty(propValue, prop);
    if (propValuePacked != null) {
      valuePacked[prop.id.toString()] = propValuePacked;
    }
  }

  return valuePacked;
}

/** Unpacks a builtin object property */
export function unpackBuiltinObjectProperty(propValuePacked: any, prop: PropertyInfo) {
  const propType = getPropertyType(prop);
  let propValue = unpackValue(propValuePacked, propType);
  if (propValue == null && prop.isList) {
    propValue = [];
  }
  return propValue;
}

/** Unpacks proto value representation of a struct. See encode. */
export function unpackBuiltinObject<T extends ObjectType>(valuePacked: any, objectType?: T): AnyTypeMapping[T] {
  if (objectType == null) {
    if (valuePacked["1"] == null) {
      throw new Error(`missing object type in ${JSON.stringify(valuePacked)}`);
    }
    objectType = valuePacked["1"] as T;
  }
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[objectType];
  const properties = PROPERTY_INFOS_BY_TYPE[objectType];
  if (propertyEnum == null || properties == null) {
    throw new Error(`unexpected object type ${objectType}`);
  }

  const value = {} as AnyTypeMapping[T];
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propValuePacked = valuePacked[prop.id.toString()];
    const propValue = unpackBuiltinObjectProperty(propValuePacked, prop);
    if (propValue != null) {
      (value as any)[propName] = propValue;
    } else if (prop.default != null) {
      (value as any)[propName] = prop.default;
    } else if (prop.isList) {
      (value as any)[propName] = [];
    }
  }
  value.metatype = objectType;
  return value;
}

/** Gets the node type and subtype for a custom object (from type or current value for partials). */
export function getPartialObjectType(
  type: TypeIdentity,
  valuePacked: Record<string, JsonValue>,
): { nodeType: NodeType | null } {
  // node type
  let nodeType: NodeType | null = null;
  if (type.kind == TypeKind.PARTIAL_OBJECT) {
    if (type.destackType != null) {
      nodeType = type.destackType as unknown as NodeType;
    } else if ("1" in valuePacked) {
      nodeType = valuePacked["1"] as NodeType;
    }
    if (!isNodeType(nodeType)) {
      nodeType = null;
    }
  }

  return { nodeType };
}

/** Gets the properties for a custom object (from type or current value for partials). */
export function getCustomObjectProperties(type: TypeIdentity, valuePacked: Record<string, JsonValue>): PropertyInfo[] {
  if (type.kind == TypeKind.PARTIAL_OBJECT) {
    let properties: PropertyInfo[] = [];

    // node
    const { nodeType } = getPartialObjectType(type, valuePacked);
    if (nodeType == null) return [];
    const nodeProperties = PROPERTY_INFOS_BY_TYPE[nodeType]!;
    for (const prop of Object.values(nodeProperties)) {
      properties.push(prop);
    }

    // assemble properties
    if (type.propertyFieldTypes != null) {
      properties = properties.filter((p) => type.propertyFieldTypes!.includes(p.fieldType!));
    }
    return properties;
  } else {
    return [];
  }
}

// NOTE :Test: figure out how to test value packing on destack-web properly (ensure it's in sync with destack)

/** Packs a single object value into a packed & secret packed value. */
export function packCustomObject(
  value: ScalarValue,
  type: TypeIdentity,
  options: { recurseCustomObject?: boolean },
): JsonValue {
  const valuePacked: { [key: string]: JsonValue } = {};

  // fields
  for (const [storageKey, fieldValue] of Object.entries(value as any)) {
    if (!isNaN(parseInt(storageKey[0]))) {
      continue; // property
    } else if (fieldValue == null) {
      continue;
    }
    const typeIdentity = decodeTypeIdentity(storageKey.slice(CK_LENGTH_B64 + 1));
    if (typeIdentity.kind == TypeKind.CUSTOM_OBJECT || typeIdentity.kind == TypeKind.PARTIAL_OBJECT) {
      if (options.recurseCustomObject) {
        valuePacked[storageKey] = packValue(fieldValue, typeIdentity, {
          recurseCustomObject: options.recurseCustomObject,
        });
      } else {
        valuePacked[storageKey] = fieldValue as JsonValue; // keep packed as is
      }
    } else {
      valuePacked[storageKey] = packValue(fieldValue, typeIdentity);
    }
  }

  // properties
  const properties = getCustomObjectProperties(type, value as any);
  if (properties != null) {
    for (const prop of Object.values(properties)) {
      const propStorageKey = prop.id.toString();
      const propValue = (value as any)[propStorageKey];
      const propType = getPropertyType(prop);
      if (propValue == null) {
        continue;
      } else {
        valuePacked[propStorageKey] = packValue(propValue, propType);
      }
    }
  }

  return valuePacked;
}

/** Unpacks a single packed & secret packed value into an object. */
export function unpackCustomObject(
  valuePacked: JsonValue,
  type: TypeIdentity,
  options: { recurseCustomObject?: boolean },
): SomeValue {
  const value: { [key: string]: SomeValue } = {};

  // fields
  for (const [storageKey, fieldValuePacked] of Object.entries(valuePacked as any)) {
    if (!isNaN(parseInt(storageKey[0]))) {
      continue; // property
    } else if (fieldValuePacked == null) {
      continue;
    }
    const typeIdentity = decodeTypeIdentity(storageKey.slice(CK_LENGTH_B64 + 1));
    if (typeIdentity.kind == TypeKind.CUSTOM_OBJECT || typeIdentity.kind == TypeKind.PARTIAL_OBJECT) {
      if (options.recurseCustomObject) {
        const fieldValue = unpackValue(fieldValuePacked as JsonValue, typeIdentity, {
          recurseCustomObject: options.recurseCustomObject,
        });
        if (fieldValue != null) {
          value[storageKey] = fieldValue;
        }
      } else {
        value[storageKey] = fieldValuePacked as JsonValue; // keep packed as is
      }
    } else {
      value[storageKey] = unpackValue(fieldValuePacked as JsonValue, typeIdentity);
    }
  }

  // properties
  const properties = getCustomObjectProperties(type, valuePacked as any);
  if (properties != null) {
    for (const prop of properties) {
      const propStorageKey = prop.id.toString();
      const propValue = (valuePacked as any)[propStorageKey];
      const propType = getPropertyType(prop);
      if (propValue == null) {
        continue;
      } else {
        value[propStorageKey] = unpackValue(propValue, propType);
      }
    }
  }

  return value;
}

/**
 * Pack the value data into JSON wire format.
 * Graph is required if we're dealing with an alias or any object type.
 * */
export function packValue(
  value: any,
  type: TypeIdentity,
  options: { wrapScalar?: boolean; recurseCustomObject?: boolean } = {
    wrapScalar: false,
    recurseCustomObject: false,
  },
): JsonValue {
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_OBJECT) {
    // nested custom object
    if (!options.recurseCustomObject) {
      return value; // as is
    } else if (value == null) {
      return null;
    } else if (!type.isList) {
      return packCustomObject(value, type, { recurseCustomObject: options.recurseCustomObject });
    } else {
      const valuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = packCustomObject(value[i], type, { recurseCustomObject: options.recurseCustomObject });
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
  options: { wrapScalar?: boolean; recurseCustomObject?: boolean } = {
    wrapScalar: false,
    recurseCustomObject: false,
  },
): any {
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_OBJECT) {
    // nested custom object
    if (!options.recurseCustomObject) {
      return valuePacked; // as is
    } else if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackCustomObject(valuePacked, type, {
        recurseCustomObject: options.recurseCustomObject,
      });
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`);
      }
      return valuePacked!.map((v: any, i: number) =>
        unpackCustomObject(v, type, {
          recurseCustomObject: options.recurseCustomObject,
        }),
      );
    }
  } else {
    // scalar
    if (valuePacked == null) {
      return null;
    }
    let value;
    if (options?.wrapScalar) {
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
// NOTE :Architecture :Cleanup: run type-checking via :DestackWebRuntime if available?
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
  } else {
    // NOTE :Incomplete: checkValueScalar for Node/Struct/Enum
  }
}

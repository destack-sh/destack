import { FLOAT_EPSILON, isNodeType } from "@/language/const";
import {
  describeTypeIdentity,
  encodeTypeIdentity,
  getPropertyType,
  getStorageKey,
  resolveFields,
  type TypeIdentity,
} from "@/language/field";
import type { ReadNodeGraph } from "@/language/graph";
import {
  DateTime,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NODE_SUBTYPE_PROPERTY_ID,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
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
      // NOTE :Robustness: we pack bigints as numbers, which is only safe up to 2^53-1
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
  } else if (prop.isList) {
    return propValue.map((v: any) => packValueScalar(v, propType));
  } else {
    return packValueScalar(propValue, propType);
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
  let propValue;
  if (prop.isList) {
    if (propValuePacked == null) {
      propValue = [];
    } else {
      propValue = propValuePacked.map((v: any) => unpackValueScalar(v, propType));
    }
  } else {
    if (propValuePacked == null) {
      if (!prop.isRequired) {
        return undefined;
      } else {
        propValue = null;
      }
    } else {
      propValue = unpackValueScalar(propValuePacked, propType);
    }
  }
  return propValue;
}

/** Unpacks proto value representation of a struct. See encode. */
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
    const propValuePacked = valuePacked[prop.id.toString()];
    const propValue = unpackBuiltinObjectProperty(propValuePacked, prop);
    if (propValue != null) {
      (value as any)[propName] = propValue;
    } else if (prop.default != null) {
      (value as any)[propName] = prop;
    } else if (prop.isList) {
      (value as any)[propName] = [];
    }
  }
  value.metatype = objectType;
  return value;
}

/** Gets the node type for a custom object (from type or current value for partials). */
export function getCustomObjectNodeType(type: TypeIdentity, valuePacked: Record<string, JsonValue>): NodeType | null {
  if (type.kind == TypeKind.PARTIAL_OBJECT) {
    let nodeType: NodeType;
    if (type.benchType != null) {
      nodeType = type.benchType as unknown as NodeType;
    } else {
      nodeType = valuePacked["1"] as NodeType;
    }
    if (isNodeType(nodeType)) {
      return nodeType;
    }
  }
  return null;
}

/** Gets the subtype for a custom object (from type or current value for partials). */
export function getCustomObjectSubtype(type: TypeIdentity, valuePacked: Record<string, JsonValue>): number | null {
  const nodeType = getCustomObjectNodeType(type, valuePacked);
  if (nodeType == null) return null;
  const subtypePropertyId = NODE_SUBTYPE_PROPERTY_ID[nodeType];
  if (subtypePropertyId == null) return null;
  return valuePacked[subtypePropertyId.toString()] as number;
}

/** Gets the properties for a custom object (from type or current value for partials). */
export function getCustomObjectProperties(type: TypeIdentity, valuePacked: Record<string, JsonValue>): PropertyInfo[] {
  if (type.kind == TypeKind.PARTIAL_OBJECT) {
    let properties: PropertyInfo[] = [];

    // figure out actual node type
    const nodeType = getCustomObjectNodeType(type, valuePacked);
    if (nodeType == null) throw new Error(`missing node type in ${describeTypeIdentity(type)}`);
    const nodeProperties = PROPERTY_INFOS_BY_TYPE[nodeType]!;
    for (const prop of Object.values(nodeProperties)) {
      properties.push(prop);
    }

    // check subtype
    const subtypePropertyId = NODE_SUBTYPE_PROPERTY_ID[nodeType];
    if (subtypePropertyId != null) {
      const subtype = getCustomObjectSubtype(type, valuePacked);
      if (subtype != null) {
        const subtypeProperties = PROPERTY_INFOS_BY_SUBTYPE[nodeType]?.[subtype];
        if (subtypeProperties != null) {
          for (const prop of Object.values(subtypeProperties)) {
            properties.push(prop);
          }
        }
      }
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

// NOTE :Test: figure out how to test value packing on bench-web properly (ensure it's in sync with bench)

/** Packs a single object value into a packed & secret packed value. */
export function packCustomObject(
  value: ScalarValue,
  type: TypeIdentity,
  options: { graph: ReadNodeGraph; recurseCustomObject?: boolean },
): JsonValue {
  const fields = resolveFields(type, options.graph);
  const valuePacked: { [key: string]: JsonValue } = {};

  // fields
  for (const field of fields) {
    const fieldStorageKey = getStorageKey(field, field);
    const fieldValue = (value as any)[fieldStorageKey];
    if (fieldValue == null) {
      continue;
    } else if (field.kind == TypeKind.CUSTOM_OBJECT || field.kind == TypeKind.PARTIAL_OBJECT) {
      if (options.recurseCustomObject) {
        valuePacked[fieldStorageKey] = packValue(fieldValue, field, {
          graph: options.graph,
          wrapScalar: false,
          recurseCustomObject: options.recurseCustomObject,
        });
      } else {
        valuePacked[fieldStorageKey] = fieldValue; // keep packed as is
      }
    } else if (!field.isList) {
      valuePacked[fieldStorageKey] = packValueScalar(fieldValue, field);
    } else {
      valuePacked[fieldStorageKey] = fieldValue.map((v: any) => packValueScalar(v, field));
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
      } else if (!prop.isList) {
        valuePacked[propStorageKey] = packValueScalar(propValue, propType);
      } else {
        valuePacked[propStorageKey] = propValue.map((v: any) => packValueScalar(v, propType));
      }
    }
  }

  return valuePacked;
}

/** Unpacks a single packed & secret packed value into an object. */
export function unpackCustomObject(
  valuePacked: JsonValue,
  type: TypeIdentity,
  options: { graph: ReadNodeGraph; recurseCustomObject?: boolean },
): SomeValue {
  const fields = resolveFields(type, options.graph);
  const value: { [key: string]: SomeValue } = {};

  // fields
  for (const field of fields) {
    const fieldStorageKey = getStorageKey(field, field);
    const fieldValuePacked = (valuePacked as any)[fieldStorageKey];
    if (fieldValuePacked == null) {
      continue;
    } else if (field.kind == TypeKind.CUSTOM_OBJECT || field.kind == TypeKind.PARTIAL_OBJECT) {
      if (options.recurseCustomObject) {
        const fieldValue = unpackValue(fieldValuePacked, field, {
          graph: options.graph,
          wrapScalar: false,
          recurseCustomObject: options.recurseCustomObject,
        });
        if (fieldValue != null) {
          value[fieldStorageKey] = fieldValue;
        }
      } else {
        value[fieldStorageKey] = fieldValuePacked; // keep packed as is
      }
    } else if (!field.isList) {
      value[fieldStorageKey] = unpackValueScalar(fieldValuePacked, field);
    } else {
      value[fieldStorageKey] = fieldValuePacked.map((v: any) => unpackValueScalar(v, field));
    }
  }

  // properties
  const properties = getCustomObjectProperties(type, value as any);
  if (properties != null) {
    for (const prop of properties) {
      const propStorageKey = prop.id.toString();
      const propValue = (valuePacked as any)[propStorageKey];
      const propType = getPropertyType(prop);
      if (propValue == null) {
        continue;
      } else if (!prop.isList) {
        value[propStorageKey] = unpackValueScalar(propValue, propType);
      } else {
        value[propStorageKey] = propValue.map((v: any) => unpackValueScalar(v, propType));
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
  options: { graph?: ReadNodeGraph; wrapScalar?: boolean; recurseCustomObject?: boolean } = {
    wrapScalar: true,
    recurseCustomObject: true,
  },
): JsonValue {
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_OBJECT) {
    // nested custom object
    if (options.graph == null) throw new Error(`missing graph to pack object type ${describeTypeIdentity(type)}`);
    if (value == null) {
      return null;
    } else if (!type.isList) {
      return packCustomObject(value, type, { graph: options.graph, recurseCustomObject: options.recurseCustomObject });
    } else {
      const valuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = packCustomObject(value[i], type, {
          graph: options.graph,
          recurseCustomObject: options.recurseCustomObject,
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
  options: { graph?: ReadNodeGraph; wrapScalar?: boolean; recurseCustomObject?: boolean } = {
    wrapScalar: true,
    recurseCustomObject: true,
  },
): any {
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_OBJECT) {
    // nested custom object
    if (options.graph == null) {
      throw new Error(
        `missing graph to unpack object type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`,
      );
    }
    if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackCustomObject(valuePacked, type, {
        graph: options.graph,
        recurseCustomObject: options.recurseCustomObject,
      });
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`);
      }
      return valuePacked!.map((v: any, i: number) =>
        unpackCustomObject(v, type, {
          graph: options.graph!,
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

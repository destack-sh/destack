import { FLOAT_EPSILON } from "@/language/const";
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
  FieldType,
  InputObjectData,
  InputObjectDataInfo,
  InputObjectProperty,
  MemberObjectDataInfo,
  MemberObjectProperty,
  NodeReferenceData,
  ObjectKind,
  ObjectType,
  OutputObjectDataInfo,
  OutputObjectProperty,
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
  VariableObjectDataInfo,
  VariableObjectProperty,
  type AnyNodeData,
  type AnyStructData,
  type AnyTypeMapping,
} from "@/proto/wire";
import { Duration } from "@/proto/wire/google/protobuf/duration";
import { isNodeRef, isStruct } from "@/proto/wiring";
import { timedeltaFromISOFormat, timedeltaToISOFormat } from "@/utils/time";

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

export const CUSTOM_OBJECT_PROPERTY_INFOS_BY_KIND: Partial<Record<ObjectKind, Record<any, PropertyInfo>>> = {
  [ObjectKind.VARIABLE]: VariableObjectDataInfo,
  [ObjectKind.MEMBER]: MemberObjectDataInfo,
  [ObjectKind.INPUT]: InputObjectDataInfo,
  [ObjectKind.OUTPUT]: OutputObjectDataInfo,
};
export const CUSTOM_OBJECT_PROPERTY_ENUM_BY_KIND: Partial<Record<ObjectKind, Record<any, any>>> = {
  [ObjectKind.VARIABLE]: VariableObjectProperty,
  [ObjectKind.MEMBER]: MemberObjectProperty,
  [ObjectKind.INPUT]: InputObjectProperty,
  [ObjectKind.OUTPUT]: OutputObjectProperty,
};
export const OBJECT_KIND_BY_FIELD_TYPE: Partial<Record<FieldType, ObjectKind>> = {
  [FieldType.VARIABLE]: ObjectKind.VARIABLE,
  [FieldType.MEMBER]: ObjectKind.MEMBER,
  [FieldType.INPUT]: ObjectKind.INPUT,
  [FieldType.OUTPUT]: ObjectKind.OUTPUT,
};
export const OBJECT_KIND_BY_OBJECT_TYPE: Partial<Record<ObjectType, ObjectKind>> = {
  [ObjectType.VARIABLE_OBJECT]: ObjectKind.VARIABLE,
  [ObjectType.MEMBER_OBJECT]: ObjectKind.MEMBER,
  [ObjectType.INPUT_OBJECT]: ObjectKind.INPUT,
  [ObjectType.OUTPUT_OBJECT]: ObjectKind.OUTPUT,
};

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
      throw new Error(`:Incomplete ${JSON.stringify(value)} for type ${describeTypeIdentity(type)}`);
    } else if (type.primitiveType == PrimitiveType.INTERVAL) {
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
      return TimeOfDay.fromJsDate(new Date(valuePacked as string));
    } else if (type.primitiveType == PrimitiveType.INTERVAL) {
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

// TODO :Test!: figure out how to test value packing on bench-web properly (ensure it's in sync with bench)

/** Packs a single object value into a packed & secret packed value. */
export function packCustomObject(
  kind: ObjectKind,
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
    } else if (field.kind == TypeKind.CUSTOM_OBJECT || field.kind == TypeKind.PARTIAL_NODE) {
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
  const propertiesInfos = CUSTOM_OBJECT_PROPERTY_INFOS_BY_KIND[kind];
  const propertiesEnum = CUSTOM_OBJECT_PROPERTY_ENUM_BY_KIND[kind];
  if (propertiesInfos != null && propertiesEnum != null) {
    for (const prop of Object.values(propertiesInfos)) {
      const propStorageKey = propertiesEnum[prop.id];
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
  kind: ObjectKind,
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
    } else if (field.kind == TypeKind.CUSTOM_OBJECT || field.kind == TypeKind.PARTIAL_NODE) {
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
  const propertiesInfos = CUSTOM_OBJECT_PROPERTY_INFOS_BY_KIND[kind];
  const propertiesEnum = CUSTOM_OBJECT_PROPERTY_ENUM_BY_KIND[kind];
  if (propertiesInfos != null && propertiesEnum != null) {
    for (const prop of Object.values(propertiesInfos)) {
      const propStorageKey = propertiesEnum[prop.id];
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

/** Unpacks a single custom object property. */
export function unpackCustomObjectProperty<
  T extends ObjectType.VARIABLE_OBJECT | ObjectType.MEMBER_OBJECT | ObjectType.INPUT_OBJECT | ObjectType.OUTPUT_OBJECT,
  K extends keyof AnyTypeMapping[T],
>(objectType: T, valuePacked: JsonValue, propertyName: K): AnyTypeMapping[T][K] | undefined {
  const kind = OBJECT_KIND_BY_OBJECT_TYPE[objectType];
  if (kind == null) throw new Error(`unknown object type ${objectType}`);
  const propertiesInfos = CUSTOM_OBJECT_PROPERTY_INFOS_BY_KIND[kind];
  const propertiesEnum = CUSTOM_OBJECT_PROPERTY_ENUM_BY_KIND[kind];
  if (propertiesInfos == null || propertiesEnum == null) throw new Error(`unknown object kind ${kind}`);
  const propId = propertiesEnum[propertyName];
  const propInfo = propertiesInfos[propId];
  const propType = getPropertyType(propInfo);
  const propValue = (valuePacked as any)?.[propId];
  if (propValue == null) {
    return undefined;
  } else if (!propInfo.isList) {
    return unpackValueScalar(propValue, propType) as AnyTypeMapping[T][K];
  } else {
    return propValue?.map((v: any) => unpackValueScalar(v, propType)) as AnyTypeMapping[T][K];
  }
}

/** Packs a custom object property. */
export function packCustomObjectProperty<
  T extends ObjectType.VARIABLE_OBJECT | ObjectType.MEMBER_OBJECT | ObjectType.INPUT_OBJECT | ObjectType.OUTPUT_OBJECT,
  K extends keyof AnyTypeMapping[T],
>(objectType: T, value: AnyTypeMapping[T][K], propertyName: K): JsonValue | null {
  const kind = OBJECT_KIND_BY_OBJECT_TYPE[objectType];
  if (kind == null) throw new Error(`unknown object type ${objectType}`);
  const propertiesInfos = CUSTOM_OBJECT_PROPERTY_INFOS_BY_KIND[kind];
  const propertiesEnum = CUSTOM_OBJECT_PROPERTY_ENUM_BY_KIND[kind];
  if (propertiesInfos == null || propertiesEnum == null) throw new Error(`unknown object kind ${kind}`);
  const propId = propertiesEnum[propertyName];
  const propInfo = propertiesInfos[propId];
  const propType = getPropertyType(propInfo);
  let propValuePacked;
  if (value == null) {
    propValuePacked = null;
  } else if (!propInfo.isList) {
    propValuePacked = packValueScalar(value, propType);
  } else {
    propValuePacked = (value as any[]).map((v: any) => packValueScalar(v, propType));
  }
  return { [propId]: propValuePacked };
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
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_NODE) {
    // nested custom object
    const objectKind = type.baseFieldType == null ? ObjectKind.BUILTIN : OBJECT_KIND_BY_FIELD_TYPE[type.baseFieldType];
    if (objectKind == null) throw new Error(`unknown object kind for field type ${describeTypeIdentity(type)}`);
    if (options.graph == null) throw new Error(`missing graph to pack object type ${describeTypeIdentity(type)}`);
    if (value == null) {
      return null;
    } else if (!type.isList) {
      return packCustomObject(objectKind, value, type, {
        graph: options.graph,
        recurseCustomObject: options.recurseCustomObject,
      });
    } else {
      const valuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = packCustomObject(objectKind, value[i], type, {
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
  if (type.kind == TypeKind.CUSTOM_OBJECT || type.kind == TypeKind.PARTIAL_NODE) {
    // nested custom object
    const objectKind = type.baseFieldType == null ? ObjectKind.BUILTIN : OBJECT_KIND_BY_FIELD_TYPE[type.baseFieldType];
    if (objectKind == null) throw new Error(`unknown object kind for field type ${describeTypeIdentity(type)}`);
    if (options.graph == null) {
      throw new Error(
        `missing graph to unpack object type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`,
      );
    }
    if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackCustomObject(objectKind, valuePacked, type, {
        graph: options.graph,
        recurseCustomObject: options.recurseCustomObject,
      });
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}: ${JSON.stringify(valuePacked)}`);
      }
      return valuePacked!.map((v: any, i: number) =>
        unpackCustomObject(objectKind, v, type, {
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

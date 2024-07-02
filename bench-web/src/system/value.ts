import {
  BenchType,
  BlockData,
  FieldData,
  FieldZone,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PrimitiveType,
  PropertyReferenceData,
  Struct as ProtoStruct,
  StructType,
  Timestamp,
  TypeKind,
  type AnyNodeData,
  type AnyStructData,
  type AnyTypeMapping,
  type PropertyInfo,
  type TypeInfoData,
} from "@/proto/wire";
import { describeNode, isStruct, makeDefaultStruct, propertyInfo } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";
import {
  CLASSY_BLOCK_TYPES,
  TK_LENGTH_B64,
  getTkB64FromCk,
  getTkB64FromPtr,
  padCkFromTkB64,
  toCamelName,
} from "@/system/lang";
import { decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";

export type TypeIdentity = Pick<
  TypeInfoData,
  | "kind"
  | "primitiveType"
  | "benchType"
  | "baseTypePtr"
  | "baseFieldZone"
  | "formatHint"
  | "isList"
  | "isSecret"
  | "constraint"
> & { id?: any; ck?: string };

export function describeTypeIdentity(type: TypeIdentity & Partial<AnyNodeData>): string {
  if (type.kind == null) return "<empty>";
  const typeParts: string[] = [];
  if ("id" in type) typeParts.push(`id=${type.id}`);
  if ("ck" in type) typeParts.push(`ck=${type.ck}`);
  if ("revision" in type) typeParts.push(`revision=${type.revision}`);
  if (type.primitiveType != null) typeParts.push(toCamelName(PrimitiveType, type.primitiveType!));
  if (type.benchType != null) typeParts.push(toCamelName(BenchType, type.benchType!));
  if (type.baseTypePtr != null) typeParts.push(`base=${describeNode(type.baseTypePtr)}`);
  if (type.isList) typeParts.push("list");
  if (type.isSecret) typeParts.push("secret");
  const kindName = toCamelName(TypeKind, type.kind);
  return `${kindName}[${typeParts.join(", ")}]`;
}

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | { [key: string]: JsonValue } | JsonValue[];
export type PrimitiveValue = JsonPrimitive | bigint | Timestamp;
export type ScalarValue = PrimitiveValue | ProtoStruct | AnyStructData | AnyNodeData;
export type SomeValue = ScalarValue | SomeValue[] | { [key: string]: SomeValue };

export function makeTypeInfo(partial: Partial<Omit<TypeInfoData, "metatype">>): TypeInfoData {
  return makeDefaultStruct({ ...partial, metatype: StructType.TYPE_INFO });
}

const _propertyTypeInfos: Record<string, TypeIdentity> = {};

export function getPropertyType(property: PropertyInfo | PropertyReferenceData): TypeIdentity {
  if (isStruct(property, StructType.PROPERTY_REFERENCE)) {
    property = propertyInfo(property.type as unknown as ObjectType, property.id);
  }
  const cacheKey = `${property.component}.${property.id}`;
  const cached = _propertyTypeInfos[cacheKey];
  if (cached == null) {
    let kind: TypeKind;
    let benchType: BenchType | undefined;
    let primitiveType: PrimitiveType | undefined;
    if ((property.referenceNodes?.length ?? 0) > 0) {
      kind = TypeKind.NODE;
      benchType = property.referenceNodes![0] as unknown as BenchType;
    } else if (property.referenceStruct != null) {
      kind = TypeKind.STRUCT;
      benchType = property.referenceStruct as unknown as BenchType;
    } else if (property.enumType != null) {
      kind = TypeKind.ENUM;
      benchType = property.enumType as unknown as BenchType;
    } else if (property.primitiveType != null) {
      kind = TypeKind.PRIMITIVE;
      primitiveType = property.primitiveType;
    } else {
      throw new Error(`cannot determine type info for ${JSON.stringify(property)}`);
    }

    const type: TypeIdentity = {
      kind,
      benchType,
      primitiveType,
      isList: property.isList ?? false,
      isSecret: property.isEncrypted ?? false,
    };
    _propertyTypeInfos[cacheKey] = type;
  }
  return _propertyTypeInfos[cacheKey]!;
}

export function propertyType(metatype: ObjectType, id: number, override?: Partial<TypeInfoData>) {
  const prop = propertyInfo(metatype, id);
  const type = getPropertyType(prop);
  if (override != null) {
    return { ...type, ...override };
  } else {
    return type;
  }
}

const LETTER_BY_TYPE_KIND: Partial<Record<TypeKind, string>> = {
  [TypeKind.PRIMITIVE]: "p",
  [TypeKind.STRUCT]: "s",
  [TypeKind.NODE]: "n",
  [TypeKind.ENUM]: "e",
  [TypeKind.BASED_NODE]: "b",
  [TypeKind.OBJECT]: "o",
};
const TYPE_KIND_BY_LETTER: Partial<Record<string, TypeKind>> = {
  p: TypeKind.PRIMITIVE,
  s: TypeKind.STRUCT,
  n: TypeKind.NODE,
  e: TypeKind.ENUM,
  b: TypeKind.BASED_NODE,
  o: TypeKind.OBJECT,
};

/**
 * Encodes the type identity into a key for storage & implicit typing.
 * Format is <kind>[id] (with id encoded as base64).
 * :TypeInfoEncoding
 */
export function encodeTypeIdentity(type: TypeIdentity): string {
  let value: string | null = null;
  if (type.kind == TypeKind.PRIMITIVE) {
    value = encodeB64VLQ(type.primitiveType!);
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.STRUCT || type.kind == TypeKind.ENUM) {
    value = encodeB64VLQ(type.benchType!);
  } else if (type.kind == TypeKind.BASED_NODE) {
    value = `${getTkB64FromPtr(type.baseTypePtr!)}${encodeB64VLQ(type.benchType!)}`;
  } else if (type.kind == TypeKind.OBJECT) {
    value = getTkB64FromPtr(type.baseTypePtr!);
  } else {
    throw new Error(`unsupported type kind ${type?.kind} in ${describeTypeIdentity(type)}`);
  }

  const prefix = type.isList ? LETTER_BY_TYPE_KIND[type.kind]!.toUpperCase() : LETTER_BY_TYPE_KIND[type.kind]!;
  if (type.isSecret) return `!${prefix}${value}`;
  else return `${prefix}${value}`;
}

/** Decodes the type-related info back from the identity key. See encode. :TypeInfoEncoding */
export function decodeTypeIdentity(key: string): TypeIdentity {
  let isSecret: boolean;
  if (key[0] === "!") {
    key = key.slice(1);
    isSecret = true;
  } else {
    isSecret = false;
  }
  let isList: boolean;
  let kind: TypeKind;
  if (key[0].toUpperCase() === key[0]) {
    isList = true;
    kind = TYPE_KIND_BY_LETTER[key[0].toLowerCase()]!;
  } else {
    isList = false;
    kind = TYPE_KIND_BY_LETTER[key[0]]!;
  }
  const value = key.slice(1);

  if (kind === TypeKind.PRIMITIVE) {
    return { kind, primitiveType: decodeB64VLQ(value) as PrimitiveType, isList, isSecret };
  } else if (kind === TypeKind.NODE || kind === TypeKind.STRUCT || kind === TypeKind.ENUM) {
    return { kind, benchType: decodeB64VLQ(value) as BenchType, isList, isSecret };
  } else if (kind === TypeKind.BASED_NODE) {
    const baseTypePtr = {
      metatype: ObjectType.NODE_REFERENCE,
      type: NodeType.BLOCK,
      ck: padCkFromTkB64(value.slice(0, TK_LENGTH_B64)),
    };
    const benchType = decodeB64VLQ(value.slice(TK_LENGTH_B64)) as BenchType;
    return { kind, baseTypePtr, benchType, isList, isSecret };
  } else if (kind === TypeKind.OBJECT) {
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, type: NodeType.BLOCK, ck: padCkFromTkB64(value) };
    return { kind, baseTypePtr, isList, isSecret };
  } else {
    throw new Error(`unsupported type kind ${kind}`);
  }
}

/** Gets the eternal storage key for values of this type identity. :FieldStorageKey */
export function getStorageKey(field: FieldData, fieldType?: TypeIdentity): string {
  fieldType = fieldType ?? field;
  if (field.ck == null) throw new Error(`missing ck for type ${describeTypeIdentity(field)}`);
  return `${getTkB64FromCk(field.ck)}${encodeTypeIdentity(fieldType)}`;
}

// NOTE :Architecture: :TypeResolution in frontend should probably happen reactively in a dedicated.. something.

/** Resolves the actual type identity :TypeResolution */
export function resolveType(type: TypeIdentity, graph: ReadNodeGraph): TypeIdentity {
  if (type.kind == TypeKind.ALIAS && type.baseTypePtr != null) {
    if (type.baseTypePtr.type == NodeType.STEP) {
      return makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: type.baseTypePtr });
    } else if (type.baseTypePtr.type == NodeType.BLOCK) {
      const block = graph.get(type.baseTypePtr) as BlockData | null;
      if (CLASSY_BLOCK_TYPES.includes(block?.type!)) {
        return makeTypeInfo({ kind: TypeKind.OBJECT, baseTypePtr: type.baseTypePtr });
      } else if (block?.valueType != null) {
        return block.valueType;
      }
    }
  } else {
    return type;
  }

  throw new Error(`unexpected base ${describeNode(type.baseTypePtr)} for type ${describeTypeIdentity(type)}`);
}

/** Resolves the actual fields of the given type. :TypeResolution */
export function resolveFields(type: TypeIdentity, graph: ReadNodeGraph): FieldData[] {
  if (type.baseTypePtr == null) return [];
  const fields = graph.getChildren(type.baseTypePtr, NodeType.FIELD);
  if (type.baseFieldZone == null) return fields.filter((f) => f.zone != FieldZone.OPTION);
  else return fields.filter((f) => f.zone == type.baseFieldZone);
}

// TODO :Architecture :Performance: encode/decode protoStruct/Json in connections (at the fetch/commit boundary) :ProtoStructMapping
//  Could either fork protobuf-ts or just switch to ts-proto?

export function isProtoJson(value: any): value is ProtoStruct {
  return typeof value == "object" && "fields" in value && !("metatype" in value);
}

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
      // auto-unpack proto json
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
    } else if ((value as NodeReferenceData).metatype != ObjectType.NODE_REFERENCE) {
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
export function packBuiltinObject(value: AnyStructData | AnyNodeData, options?: { only?: string[] }): Record<string, any> {
  const propertyEnum = PROPERTY_ENUM_BY_TYPE[value.metatype];
  const properties = PROPERTY_INFOS_BY_TYPE[value.metatype];
  if (propertyEnum == null || properties == null) throw new Error(`unexpected object type ${value.metatype}`);

  const valuePacked: Record<string, any> = {};
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propType = getPropertyType(prop);
    const propValue = (value as any)[propName];
    let propValuePacked;
    if (propValue == null || (prop.isList && propValue.length == 0)) {
      continue;
    } else if (prop.isList) {
      propValuePacked = propValue.map((v: any) => packValueScalar(v, propType));
    } else {
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

  const value = { metatype: objectType } as AnyTypeMapping[T];
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
        if (!prop.isRequired) {
          continue;
        } else {
          propValue = null;
        }
      } else {
        propValue = unpackValueScalarData(propValuePacked, propType);
      }
    }
    (value as any)[propName] = propValue;
  }

  return value;
}

// TODO :Test!: figure out how to test value packing on bench-web

/** Packs a single object value into a packed & secret packed value. */
function packValueObject(
  value: ScalarValue,
  type: TypeIdentity,
  graph: ReadNodeGraph,
): { valuePacked: JsonValue; secretValuePacked: JsonValue | undefined } {
  const fields = resolveFields(type, graph);
  const valuePacked: { [key: string]: JsonValue } = {};
  for (const field of fields) {
    const fieldType = resolveType(field, graph);
    const fieldStorageKey = getStorageKey(field, fieldType);
    const fieldValue = (value as any)[fieldStorageKey];
    if (fieldValue == null) {
      continue;
    } else if (fieldType.kind == TypeKind.OBJECT) {
      valuePacked[fieldStorageKey] = packValue(fieldValue, fieldType, graph).valuePacked; // :SecretValues
    } else if (!fieldType.isList) {
      valuePacked[fieldStorageKey] = packValueScalar(fieldValue, fieldType);
    } else {
      valuePacked[fieldStorageKey] = fieldValue.map((v: any) => packValueScalar(v, fieldType));
    }
  }
  return { valuePacked, secretValuePacked: undefined };
}

/** Unpacks a single packed & secret packed value into an object. */
function unpackValueObject(
  valuePacked: JsonValue,
  secretValuePacked: JsonValue | undefined,
  type: TypeIdentity,
  graph: ReadNodeGraph,
): SomeValue {
  const fields = resolveFields(type, graph);
  const value: { [key: string]: SomeValue } = {};
  for (const field of fields) {
    const fieldType = resolveType(field, graph);
    const fieldStorageKey = getStorageKey(field, fieldType);
    const fieldValuePacked = (valuePacked as any)[fieldStorageKey];
    if (fieldValuePacked == null) {
      continue;
    } else if (fieldType.kind == TypeKind.OBJECT) {
      const fieldValue = unpackValue({ valuePacked: fieldValuePacked, secretValuePacked }, fieldType, graph);
      if (fieldValue != null) {
        value[fieldStorageKey] = fieldValue;
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
 * TODO :Incomplete: handle :SecretValues
 * */
export function packValue(
  value: any,
  type: TypeIdentity,
  graph?: ReadNodeGraph | null,
  options: { wrapPrimitive: boolean } = { wrapPrimitive: true },
  previous?: { valuePacked?: JsonValue; secretValuePacked?: JsonValue | undefined },
): { valuePacked: JsonValue; secretValuePacked: JsonValue | undefined } {
  if (type.kind == TypeKind.ALIAS) {
    if (graph == null) throw new Error(`missing graph to resolve ${describeTypeIdentity(type)}`);
    type = resolveType(type, graph);
  }
  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested object
    if (graph == null) throw new Error(`missing graph to pack object type ${describeTypeIdentity(type)}`);
    if (value == null) {
      return { valuePacked: null, secretValuePacked: undefined };
    } else if (!type.isList) {
      return packValueObject(value, type, graph);
    } else {
      const valuePacked: JsonValue[] = [];
      const secretValuePacked: JsonValue[] = [];
      for (let i = 0; i < value.length; i++) {
        const packed = packValueObject(value[i], type, graph);
        valuePacked.push(packed.valuePacked);
        if (packed.secretValuePacked != null) secretValuePacked.push(packed.secretValuePacked);
      }
      return { valuePacked, secretValuePacked: secretValuePacked.length > 0 ? secretValuePacked : undefined };
    }
  } else {
    // wrap scalar
    let valuePacked;
    if (value == null) {
      valuePacked = null;
    } else if (!type.isList) {
      valuePacked = packValueScalar(value, type);
    } else {
      valuePacked = value.map((v: any) => packValueScalar(v, type));
    }
    if (options.wrapPrimitive) {
      valuePacked = { [encodeTypeIdentity(type)]: valuePacked };
    }
    return { valuePacked, secretValuePacked: undefined };
  }
}

export function packValueSimple(
  value: any,
  type: TypeIdentity,
  graph?: ReadNodeGraph,
  options: { wrapPrimitive: boolean } = { wrapPrimitive: true },
): JsonValue {
  return packValue(value, type, graph, options).valuePacked;
}

export function packValueSimpleStruct(
  value: any,
  type: TypeIdentity,
  graph?: ReadNodeGraph,
  options: { wrapPrimitive: boolean } = { wrapPrimitive: true },
): ProtoStruct {
  return ProtoStruct.fromJson(packValueSimple(value, type, graph, options));
}

/**
 * Unpack the value data from JSON wire format.
 * Graph is required if we're dealing with an alias or any object type.
 */
export function unpackValue(
  packed: { valuePacked?: JsonValue; secretValuePacked?: JsonValue },
  type: TypeIdentity,
  graph?: ReadNodeGraph,
): any {
  if (type.kind == TypeKind.ALIAS) {
    if (graph == null) throw new Error(`missing graph to resolve ${describeTypeIdentity(type)}`);
    type = resolveType(type, graph);
  }

  if (type.kind == TypeKind.ALIAS) {
    throw new Error(`unresolved type ${describeTypeIdentity(type)}`);
  } else if (type.kind == TypeKind.OBJECT) {
    // nested object
    if (graph == null) throw new Error(`missing graph to unpack object type ${describeTypeIdentity(type)}`);
    if (packed.valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackValueObject(packed.valuePacked, packed.secretValuePacked, type, graph);
    } else {
      if (!Array.isArray(packed.valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}`);
      }
      return packed.valuePacked!.map((v: any, i: number) =>
        unpackValueObject(v, (packed.secretValuePacked as Array<JsonValue>)?.[i], type, graph),
      );
    }
  } else {
    // unwrap scalar
    if (packed.valuePacked == null) {
      return null;
    } else if (typeof packed.valuePacked !== "object" || Array.isArray(packed.valuePacked)) {
      throw new Error(`expected object for scalar type ${describeTypeIdentity(type)}`);
    }
    const valuePacked = packed.valuePacked[encodeTypeIdentity(type)];
    if (valuePacked == null) {
      return null;
    } else if (!type.isList) {
      return unpackValueScalar(valuePacked, type);
    } else {
      if (!Array.isArray(valuePacked)) {
        throw new Error(`expected array for list type ${describeTypeIdentity(type)}`);
      }
      return valuePacked.map((v: any) => unpackValueScalar(v, type));
    }
  }
}

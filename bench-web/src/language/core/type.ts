import { supergraph } from "@/globals";
import { getTkB64FromCk, isNodeType, NAME_CONSTRAINT, toCamelName } from "@/language/core/const";
import { getEnumTitle } from "@/language/core/enum";
import { blockToTypeMaybe } from "@/language/source/block";
import { choiceToType } from "@/language/source/choice";
import { classToType } from "@/language/source/class";
import { databaseToType } from "@/language/source/database";
import { flowToType } from "@/language/source/flow";
import {
  BenchType,
  EnumType,
  FieldData,
  FieldType,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyReferenceData,
  StructType,
  TypeConstraintData,
  TypeData,
  TypeKind,
  type AnyNodeData,
  type PropertyInfo,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, makeDefaultObject, propertyInfo } from "@/proto/wiring";
import { decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";

export type TypeIdentity = Pick<
  TypeData,
  | "kind"
  | "primitiveType"
  | "benchType"
  | "baseTypePtr"
  | "isRequired"
  | "isList"
  | "isSecret"
  | "format"
  | "condition"
  | "constraint"
> &
  Partial<Pick<TypeData, "baseFieldTypes" | "propertyFieldTypes">> & { id?: any; ck?: string };

export const CK_LENGTH_B64 = 24; //  1.5 * CK_LENGTH_BYTES (must be integer)

const LETTER_BY_TYPE_KIND: Partial<Record<TypeKind, string>> = {
  [TypeKind.PRIMITIVE]: "p",
  [TypeKind.STRUCT]: "s",
  [TypeKind.NODE]: "n",
  [TypeKind.ENUM]: "e",
  [TypeKind.BASED_NODE]: "n", // shared with node
  [TypeKind.CUSTOM_OBJECT]: "o",
  [TypeKind.PARTIAL_OBJECT]: "r",
};
const TYPE_KIND_BY_LETTER: Partial<Record<string, TypeKind>> = {
  p: TypeKind.PRIMITIVE,
  s: TypeKind.STRUCT,
  n: TypeKind.NODE,
  e: TypeKind.ENUM,
  o: TypeKind.CUSTOM_OBJECT,
  r: TypeKind.PARTIAL_OBJECT,
};

export const LETTER_BY_FIELD_TYPE: Partial<Record<FieldType, string>> = {
  [FieldType.MEMBER]: "M",
  [FieldType.INPUT]: "I",
  [FieldType.OUTPUT]: "O",
};
export const FIELD_TYPE_BY_LETTER: Record<string, FieldType> = {
  M: FieldType.MEMBER,
  I: FieldType.INPUT,
  O: FieldType.OUTPUT,
};

export function describeTypeIdentity(type: TypeIdentity & Partial<AnyNodeData>): string {
  if (type.kind == null) return "<empty>";
  const typeParts: string[] = [];
  if ("id" in type) typeParts.push(`id=${type.id}`);
  if ("ck" in type) typeParts.push(`ck=${type.ck}`);
  if (type.primitiveType != null) typeParts.push(toCamelName(PrimitiveType, type.primitiveType!));
  if (type.benchType != null) typeParts.push(toCamelName(BenchType, type.benchType!));
  if (type.baseTypePtr != null) typeParts.push(`base=${describeNode(type.baseTypePtr)}`);
  if (type.isList) typeParts.push("list");
  if (type.isSecret) typeParts.push("secret");
  const kindName = toCamelName(TypeKind, type.kind);
  return `${kindName}[${typeParts.join(", ")}]`;
}

/** Checks whether the type identity contents are equal. */
export function typeIdentityEquals(a: TypeIdentity, b: TypeIdentity): boolean {
  if (a.primitiveType != null) {
    if (a.format != b.format) return false;
    return a.primitiveType === b.primitiveType;
  } else if (a.baseTypePtr != null) {
    return a.baseTypePtr.id === b.baseTypePtr?.id && a.benchType == b.benchType;
  } else if (a.benchType != null) {
    if (a.benchType != b.benchType) {
      return false;
    } else if ((a.constraint != null) != (b.constraint != null)) {
      return false;
    } else if (a.constraint != null && b.constraint != null) {
      return deepValueEquals(a.constraint.nodeSubtypes, b.constraint.nodeSubtypes);
    } else {
      return true;
    }
  } else {
    return false;
  }
}

export function makeTypeConstraint(partial: Partial<Omit<TypeConstraintData, "metatype">>): TypeConstraintData {
  return makeDefaultObject({ ...partial, metatype: ObjectType.TYPE_CONSTRAINT });
}

export function makeType(partial: Partial<Omit<TypeData, "metatype">>): TypeData {
  return makeDefaultObject({ ...partial, metatype: ObjectType.TYPE });
}

export const STRING_TYPE = makeType({ kind: TypeKind.PRIMITIVE, primitiveType: PrimitiveType.STRING });
export const NAME_TYPE = makeType({
  kind: TypeKind.PRIMITIVE,
  primitiveType: PrimitiveType.STRING,
  constraint: makeTypeConstraint(NAME_CONSTRAINT),
});
export const TITLE_TYPE = makeType({
  kind: TypeKind.PRIMITIVE,
  primitiveType: PrimitiveType.STRING,
});

const _propertyTypeInfos: Record<string, TypeIdentity> = {};

export function getPropertyType(prop: PropertyInfo | PropertyReferenceData): TypeIdentity {
  // cast to property info if necessary
  if (isStruct(prop, StructType.PROPERTY_REFERENCE)) {
    prop = propertyInfo(prop.objectType as unknown as ObjectType, prop.id);
  }

  // check cache
  let cacheKey: string;
  if (prop.componentSubtype != null) {
    cacheKey = `${prop.component}.${prop.id}.${prop.componentSubtype}`;
  } else {
    cacheKey = `${prop.component}.${prop.id}`;
  }
  const cached = _propertyTypeInfos[cacheKey];

  // get if cache miss
  if (cached == null) {
    let kind: TypeKind;
    let benchType: BenchType | undefined;
    let primitiveType: PrimitiveType | undefined;
    let constraint: TypeConstraintData | undefined =
      prop.constraint != null ? makeTypeConstraint(prop.constraint) : undefined;
    if ((prop.referenceNodes?.length ?? 0) > 0) {
      kind = TypeKind.NODE;
      if (prop.referenceNodes == "any") {
        benchType = undefined;
      } else if ((prop.referenceNodes?.length ?? 0) > 1) {
        benchType = undefined;
        constraint = makeTypeConstraint({ ...constraint, nodeSubtypes: prop.referenceNodes! });
      } else {
        benchType = prop.referenceNodes![0] as unknown as BenchType;
      }
    } else if (prop.referenceStruct != null) {
      kind = TypeKind.STRUCT;
      benchType = prop.referenceStruct as unknown as BenchType;
    } else if (prop.enumType != null) {
      kind = TypeKind.ENUM;
      benchType = prop.enumType as unknown as BenchType;
    } else if (prop.referenceIsNodeData) {
      kind = TypeKind.PRIMITIVE;
      primitiveType = PrimitiveType.JSON; // not sure what to put here, this is inaccessible outside of the system
    } else if (prop.valueIsPartial) {
      kind = TypeKind.PARTIAL_OBJECT;
      primitiveType = PrimitiveType.JSON;
    } else if (prop.primitiveType != null) {
      kind = TypeKind.PRIMITIVE;
      primitiveType = prop.primitiveType;
    } else {
      throw new Error(`cannot determine type info for ${JSON.stringify(prop)}`);
    }

    const type: TypeIdentity = {
      kind,
      benchType,
      primitiveType,
      isRequired: prop.isRequired ?? false,
      isList: prop.isList ?? false,
      isSecret: prop.isEncrypted ?? false,
      constraint: prop.constraint != null ? makeTypeConstraint(prop.constraint) : undefined,
    };
    _propertyTypeInfos[cacheKey] = type;
  }

  return _propertyTypeInfos[cacheKey]!;
}

export function propertyType(metatype: ObjectType, id: number, override?: Partial<TypeData>) {
  const prop = propertyInfo(metatype, id);
  const type = getPropertyType(prop);
  if (override != null) {
    return { ...type, ...override };
  } else {
    return type;
  }
}

/**
 * Encodes the type identity into a key for storage & implicit typing.
 * Format is <kind>[id] (with id encoded as base64).
 * :TypeInfoEncoding
 */
export function encodeTypeIdentity(type: TypeIdentity): string {
  let value: string;
  if (type.kind == TypeKind.PRIMITIVE) {
    value = encodeB64VLQ(type.primitiveType!);
  } else if (type.kind == TypeKind.NODE || type.kind == TypeKind.BASED_NODE) {
    value = ""; // joint identity for nodes
  } else if (type.kind == TypeKind.STRUCT || type.kind == TypeKind.ENUM) {
    value = encodeB64VLQ(type.benchType!);
  } else if (type.kind == TypeKind.CUSTOM_OBJECT) {
    const uuid = type.baseTypePtr!.ck ?? type.baseTypePtr!.id!;
    const uuidInt = BigInt("0x" + uuid.replace(/-/g, ""));
    value = encodeB64VLQ(uuidInt);
  } else if (type.kind == TypeKind.PARTIAL_OBJECT) {
    value = type.benchType != null ? encodeB64VLQ(type.benchType) : "";
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
    return { kind, primitiveType: decodeB64VLQ(value) as PrimitiveType, isRequired: false, isList, isSecret };
  } else if (kind == TypeKind.NODE || kind == TypeKind.BASED_NODE) {
    return { kind, isRequired: false, isList, isSecret };
  } else if (kind === TypeKind.STRUCT || kind === TypeKind.ENUM) {
    return { kind, benchType: decodeB64VLQ(value) as BenchType, isRequired: false, isList, isSecret };
  } else if (kind === TypeKind.CUSTOM_OBJECT) {
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, nodeType: NodeType.BLOCK, ck: value, id: value };
    return { kind, baseTypePtr, isRequired: false, isList, isSecret };
  } else if (kind == TypeKind.PARTIAL_OBJECT) {
    return {
      kind,
      benchType: value != "" ? (decodeB64VLQ(value) as BenchType) : undefined,
      isRequired: false,
      isList,
      isSecret,
    };
  } else {
    throw new Error(`unsupported type kind ${kind}`);
  }
}

/** Gets the eternal storage key for values of this type identity. :FieldStorageKey */
export function getStorageKey(field: FieldData, fieldType?: TypeIdentity): string {
  fieldType = fieldType ?? field;
  if (field.ck == null) throw new Error(`missing ck for type ${describeTypeIdentity(field)}`);
  return `${LETTER_BY_FIELD_TYPE[field.type]}${getTkB64FromCk(field.ck)}${encodeTypeIdentity(fieldType)}`;
}

export function getFieldType(storageKey: string): FieldType | null {
  const fieldType = FIELD_TYPE_BY_LETTER[storageKey[0]];
  if (fieldType == null) throw new Error(`invalid storage key: '${storageKey}'`);
  return fieldType;
}

/** Whether the value meets the type constraints */
export function nodeMatchesConstraint(node: AnyNodeData, constraint: TypeConstraintData): boolean {
  if (constraint.nodeSubtypes.length > 0) {
    return constraint.nodeSubtypes.includes((node as any).type);
  } else {
    return true; // no constraint
  }
}

/** Whether the given type supports lists :ListableTypes. */
export function typeSupportsList(type: { kind: TypeKind } & Partial<TypeData>): boolean {
  if (
    [TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.ENUM, TypeKind.CUSTOM_OBJECT, TypeKind.PARTIAL_OBJECT].includes(
      type.kind,
    )
  ) {
    return true;
  } else if (
    [
      PrimitiveType.INT32,
      PrimitiveType.INT64,
      PrimitiveType.FLOAT32,
      PrimitiveType.FLOAT64,
      PrimitiveType.STRING,
      PrimitiveType.UUID,
      PrimitiveType.DATETIME,
    ].includes(type.primitiveType!)
  ) {
    return true;
  } else {
    return false;
  }
}

/** Whether the type is some numeric type (int, float, etc.) */
export function typeIsNumeric(type: { kind: TypeKind } & Partial<TypeData>): boolean {
  return type.primitiveType != null && type.primitiveType >= 2 && type.primitiveType < 20;
}

/** Gets the Node.type enum type */
export function getSubtypeEnum(metatype: NodeType): EnumType | null {
  const nodeProperties = PROPERTY_INFOS_BY_TYPE[metatype];
  const nodePropertiesEnum = PROPERTY_ENUM_BY_TYPE[metatype];
  const subtypeProperty = nodeProperties[(nodePropertiesEnum as any)?.["type"]!];
  return subtypeProperty?.enumType ?? null;
}

/** Gets the implied subtype node name  */
export function getConstrainedTypeName(type: TypeIdentity): string | null {
  const metatypeName = toCamelName(BenchType, type.benchType);
  if (isNodeType(type.benchType) && type.constraint?.nodeSubtypes?.length == 1) {
    const enumType = getSubtypeEnum(type.benchType);
    const subtype = type.constraint.nodeSubtypes[0];
    if ((enumType as any)?.[subtype] != null) {
      return getEnumTitle(enumType!, subtype);
    } else {
      return metatypeName;
    }
  } else {
    return metatypeName;
  }
}

/** Gets the default Field name from a type  */
export function getTypeName(field: Partial<TypeData>): string {
  if (field == null) throw new Error(`missing type for field in ${describeNode(field)}`);
  if (field.kind == TypeKind.PRIMITIVE) {
    if (field.format != null) {
      return getEnumTitle(EnumType.TYPE_FORMAT, field.format);
    }
    return getEnumTitle(EnumType.PRIMITIVE_TYPE, field.primitiveType!);
  } else if (field.kind == TypeKind.STRUCT || field.kind == TypeKind.NODE || field.kind == TypeKind.ENUM) {
    if (isNodeType(field.benchType)) {
      const subtypeEnum = getSubtypeEnum(field.benchType);
      if (subtypeEnum != null && field.constraint?.nodeSubtypes?.length == 1) {
        return getEnumTitle(subtypeEnum, field.constraint.nodeSubtypes[0]);
      } else {
        return getEnumTitle(EnumType.BENCH_TYPE, field.benchType);
      }
    } else if (field.benchType != null) {
      return getEnumTitle(EnumType.BENCH_TYPE, field.benchType);
    } else {
      return getEnumTitle(EnumType.TYPE_KIND, field.kind);
    }
  } else if (field.kind == TypeKind.BASED_NODE || field.kind == TypeKind.CUSTOM_OBJECT) {
    if (field?.baseTypePtr == null) throw new Error(`missing base type for field in ${describeNode(field)}`);
    const baseType = supergraph.get(field.baseTypePtr);
    if ((baseType as any).name != null) {
      return (baseType as any).name;
    } else {
      return getEnumTitle(EnumType.BENCH_TYPE, field.benchType!);
    }
  } else if (field.kind == TypeKind.PARTIAL_OBJECT) {
    return field.benchType != null ? `${getEnumTitle(EnumType.BENCH_TYPE, field.benchType)} Partial` : "Partial";
  } else {
    throw new Error(`unexpected type kind: ${field.kind}`);
  }
}

/** Get the Type expressed by a Node. */
export function nodeToTypeMaybe(
  node: AnyNodeData,
  of?: "instance" | "value",
  fieldTypes?: FieldType[],
): TypeData | undefined {
  if (isNode(node, NodeType.CLASS)) {
    return classToType(node, of ?? "value", fieldTypes);
  } else if (isNode(node, NodeType.CHOICE)) {
    return choiceToType(node);
  } else if (isNode(node, NodeType.BLOCK)) {
    return blockToTypeMaybe(node, of, fieldTypes);
  } else if (isNode(node, NodeType.FLOW)) {
    return flowToType(node, of, fieldTypes);
  } else if (isNode(node, NodeType.DATABASE)) {
    return databaseToType(node);
  } else {
    return undefined;
  }
}

/** Get the Type expressed by a Node. */
export function nodeToType(node: AnyNodeData, of?: "instance" | "value", fieldTypes?: FieldType[]): TypeData {
  const type = nodeToTypeMaybe(node, of, fieldTypes);
  if (type == null) {
    throw new Error(`node has no type: ${describeNode(node)}`);
  }
  return type;
}

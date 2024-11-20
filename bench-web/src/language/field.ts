import {
  getTkB64FromCk,
  getTkB64FromPtr,
  NAME_CONSTRAINT,
  padCkFromTkB64,
  RUNNABLE_BLOCK_TYPES,
  toCamelName,
  TYPE_BLOCK_TYPES,
} from "@/language/const";
import { getEnumTitle } from "@/language/enum";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName } from "@/language/node";
import { getOrderKey } from "@/language/order";
import type { Transaction } from "@/language/transaction";
import {
  BenchType,
  BlockData,
  BlockType,
  ColorType,
  EnumType,
  FieldData,
  FieldType,
  FileType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PropertyReferenceData,
  StepData,
  StepType,
  StructType,
  TypeConstraintData,
  TypeInfoData,
  TypeKind,
  type AnyNodeData,
  type PropertyInfo,
} from "@/proto/wire";
import {
  contentEquals,
  describeNode,
  isNode,
  isStruct,
  makeDefaultObject,
  propertyInfo,
  toPlainNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import type { ActionBuiltinId } from "@/ui/action";
import { getNodeIcon, getTypeIcon, makeIcon } from "@/ui/icon";
import { getRandomColorType } from "@/ui/style";
import { assertNever, decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";

export const FIELD_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "common.edit.rename",
  "common.edit.morph",
  "common.edit.duplicate",
  "common.edit.delete",
  "common.create.above",
  "common.create.below",
  "type.edit.isRequired",
  "type.edit.isList",
];

export type TypeIdentity = Pick<
  TypeInfoData,
  | "kind"
  | "primitiveType"
  | "benchType"
  | "baseTypePtr"
  | "baseFieldType"
  | "isRequired"
  | "isList"
  | "isSecret"
  | "format"
  | "condition"
  | "constraint"
> & { id?: any; ck?: string };

export function describeTypeIdentity(type: TypeIdentity & Partial<AnyNodeData>): string {
  if (type.kind == null) return "<empty>";
  const typeParts: string[] = [];
  if ("id" in type) typeParts.push(`id=${type.id}`);
  if ("ck" in type) typeParts.push(`ck=${type.ck}`);
  if ("updatedEpoch" in type) typeParts.push(`updatedEpoch=${type.updatedEpoch}`);
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
      return (
        deepValueEquals(a.constraint.blockTypes, b.constraint.blockTypes) &&
        deepValueEquals(a.constraint.stepTypes, b.constraint.stepTypes) &&
        deepValueEquals(a.constraint.fileTypes, b.constraint.fileTypes) &&
        deepValueEquals(a.constraint.fileFormats, b.constraint.fileFormats)
      );
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

export function makeTypeInfo(partial: Partial<Omit<TypeInfoData, "metatype">>): TypeInfoData {
  return makeDefaultObject({ ...partial, metatype: ObjectType.TYPE_INFO });
}

export const STRING_TYPE = makeTypeInfo({ kind: TypeKind.PRIMITIVE, primitiveType: PrimitiveType.STRING });
export const NAME_TYPE = makeTypeInfo({
  kind: TypeKind.PRIMITIVE,
  primitiveType: PrimitiveType.STRING,
  constraint: makeTypeConstraint(NAME_CONSTRAINT),
});
export const TITLE_TYPE = makeTypeInfo({
  kind: TypeKind.PRIMITIVE,
  primitiveType: PrimitiveType.STRING,
  constraint: makeTypeConstraint(NAME_CONSTRAINT),
});

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
    } else if (property.referenceIsNodeData) {
      kind = TypeKind.PRIMITIVE;
      primitiveType = PrimitiveType.JSON; // not sure what to put here, this is inaccessible outside of the system
    } else {
      throw new Error(`cannot determine type info for ${JSON.stringify(property)}`);
    }

    const type: TypeIdentity = {
      kind,
      benchType,
      primitiveType,
      isRequired: property.isRequired ?? false,
      isList: property.isList ?? false,
      isSecret: property.isEncrypted ?? false,
      constraint: property.constraint != null ? makeTypeConstraint(property.constraint) : undefined,
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
  [TypeKind.BASED_NODE]: "n", // shared with node
  [TypeKind.OBJECT]: "o",
};
const TYPE_KIND_BY_LETTER: Partial<Record<string, TypeKind>> = {
  p: TypeKind.PRIMITIVE,
  s: TypeKind.STRUCT,
  n: TypeKind.NODE,
  e: TypeKind.ENUM,
  o: TypeKind.OBJECT,
};

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
    return { kind, primitiveType: decodeB64VLQ(value) as PrimitiveType, isRequired: false, isList, isSecret };
  } else if (kind == TypeKind.NODE || kind == TypeKind.BASED_NODE) {
    return { kind, isRequired: false, isList, isSecret };
  } else if (kind === TypeKind.STRUCT || kind === TypeKind.ENUM) {
    return { kind, benchType: decodeB64VLQ(value) as BenchType, isRequired: false, isList, isSecret };
  } else if (kind === TypeKind.OBJECT) {
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, nodeType: NodeType.BLOCK, ck: padCkFromTkB64(value) };
    return { kind, baseTypePtr, isRequired: false, isList, isSecret };
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

/** Gets the implied subtype node name  */
export function getConstrainedTypeName(type: TypeIdentity): string | null {
  const metatypeName = toCamelName(BenchType, type.benchType);
  if (type.constraint?.blockTypes?.length == 1) {
    const subtypeName = toCamelName(BlockType, type.constraint!.blockTypes[0]);
    return `${subtypeName} ${metatypeName}`;
  } else if (type.constraint?.stepTypes?.length == 1) {
    const subtypeName = toCamelName(StepType, type.constraint!.stepTypes[0]);
    return `${subtypeName} ${metatypeName}`;
  } else if (type.constraint?.fileTypes?.length == 1) {
    const subtypeName = toCamelName(FileType, type.constraint!.fileTypes[0]);
    return `${subtypeName} ${metatypeName}`;
  } else {
    return metatypeName;
  }
}

/** Whether the value meets the type constraints */
export function nodeMatchesConstraint(node: AnyNodeData, constraint: TypeConstraintData): boolean {
  if (constraint.blockTypes.length > 0) {
    return isNode(node, NodeType.BLOCK) && constraint.blockTypes.includes(node.type);
  } else if (constraint.stepTypes.length > 0) {
    return isNode(node, NodeType.STEP) && constraint.stepTypes.includes(node.type);
  } else if (constraint.fileTypes.length > 0 && isNode(node, NodeType.FILE)) {
    if (constraint.fileFormats.length > 0 && node.format != null && !constraint.fileFormats.includes(node.format)) {
      return false;
    } else if (constraint.fileTypes.length > 0 && !constraint.fileTypes.includes(node.type)) {
      return false;
    } else {
      return true;
    }
  } else {
    return true; // no constraint
  }
}

/** Whether the given type supports lists :ListableTypes. */
export function typeSupportsList(type: { kind: TypeKind } & Partial<TypeInfoData>): boolean {
  if ([TypeKind.NODE, TypeKind.BASED_NODE, TypeKind.ENUM, TypeKind.OBJECT].includes(type.kind)) return true;
  else if (
    [
      PrimitiveType.INT32,
      PrimitiveType.INT64,
      PrimitiveType.FLOAT32,
      PrimitiveType.FLOAT64,
      PrimitiveType.STRING,
      PrimitiveType.UUID,
      PrimitiveType.DATETIME,
    ].includes(type.primitiveType!)
  )
    return true;
  else return false;
}

/** Whether the type is some numeric type (int, float, etc.) */
export function typeIsNumeric(type: { kind: TypeKind } & Partial<TypeInfoData>): boolean {
  return type.primitiveType != null && type.primitiveType >= 2 && type.primitiveType < 20;
}

// NOTE :Architecture: :TypeResolution in frontend should probably happen reactively in a dedicated.. something.

/** Resolves the actual fields of the given type. :TypeResolution */
export function resolveFields(type: TypeIdentity, graph: ReadNodeGraph): FieldData[] {
  if (type.baseTypePtr == null) return [];
  const fields = graph.getChildren(type.baseTypePtr, NodeType.FIELD);
  if (type.baseFieldType == null) return fields.filter((f) => f.type != FieldType.OPTION);
  else return fields.filter((f) => f.type == type.baseFieldType);
}

function getFieldNameFromType(graph: ReadNodeGraph, field: Partial<FieldData>): string {
  if (field == null) throw new Error(`missing type for field in ${describeNode(field)}`);
  if (field.kind == TypeKind.PRIMITIVE) {
    if (field.format != null) {
      return getEnumTitle(EnumType.TYPE_FORMAT, field.format);
    }
    return getEnumTitle(EnumType.PRIMITIVE_TYPE, field.primitiveType!);
  } else if (field.kind == TypeKind.STRUCT || field.kind == TypeKind.NODE || field.kind == TypeKind.ENUM) {
    if (field.benchType == BenchType.FILE && field.constraint?.fileFormats?.length == 1) {
      return getEnumTitle(EnumType.FILE_FORMAT, field.constraint.fileFormats[0]);
    } else if (field.benchType == BenchType.FILE && field.constraint?.fileTypes?.length == 1) {
      return getEnumTitle(EnumType.FILE_TYPE, field.constraint.fileTypes[0]);
    } else if (field.benchType != null) {
      return getEnumTitle(EnumType.BENCH_TYPE, field.benchType);
    } else {
      return getEnumTitle(EnumType.TYPE_KIND, field.kind);
    }
  } else if (field.kind == TypeKind.BASED_NODE || field.kind == TypeKind.OBJECT) {
    if (field?.baseTypePtr == null) throw new Error(`missing base type for field in ${describeNode(field)}`);
    const baseType = graph.getOrError(field.baseTypePtr);
    if ((baseType as any).name != null) {
      return (baseType as any).name;
    } else {
      return getEnumTitle(EnumType.BENCH_TYPE, field.benchType!);
    }
  } else {
    throw new Error(`unexpected type kind: ${field.kind}`);
  }
}

/** Create a Field relative to some Field-containing node. */
export function createField(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    field?: Partial<FieldData>;
    anchor: "before" | "above" | "after" | "below" | "inside" | "start" | "end" | "center";
    target:
      | FieldData
      | TypedNodeReferenceData<NodeType.FIELD>
      | BlockData
      | TypedNodeReferenceData<NodeType.BLOCK>
      | StepData
      | TypedNodeReferenceData<NodeType.STEP>;
  },
): FieldData {
  // eslint-disable-next-line prefer-const
  let { anchor, field: fieldIn } = options;
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);

  // position in graph
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let type: FieldType;
  let kind: TypeKind | null = fieldIn?.kind ?? null;
  let siblings: FieldData[];
  if (isNode(target, NodeType.BLOCK)) {
    if (anchor != "inside" && anchor != "center") throw new Error(`unexpected anchor for block: ${anchor}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toPlainNodeRef(target);
    orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    // figure out field kind based on block type
    if (target.type == BlockType.CHOICE) {
      type = FieldType.OPTION;
      kind = TypeKind.LITERAL;
    } else if (TYPE_BLOCK_TYPES.includes(target.type)) {
      type = FieldType.MEMBER;
    } else if (RUNNABLE_BLOCK_TYPES.includes(target.type)) {
      type = FieldType.INPUT;
    } else {
      type = FieldType.VARIABLE;
    }
  } else if (isNode(target, NodeType.STEP)) {
    if (fieldIn?.type == null) throw new Error(`missing zone for step field: ${describeNode(target)}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toPlainNodeRef(target);
    if (anchor == "start") {
      orderKey = getOrderKey({ position: "before", reference: siblings[0], nodes: siblings });
    } else {
      orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    }
    type = fieldIn.type;
  } else if (isNode(target, NodeType.FIELD)) {
    if (anchor == "inside" || anchor == "center") throw new Error(`unexpected anchor for field: ${anchor}`);
    siblings = graph.getChildren(target.parentPtr!, NodeType.FIELD);
    parentPtr = target.parentPtr!;
    orderKey = getOrderKey({ position: anchor == "start" ? "before" : "after", reference: target, nodes: siblings });
    type = target.type;
    // copy kind if none given
    kind = fieldIn?.kind ?? (target as FieldData).kind;
  } else {
    assertNever(target, `unexpected target node type: ${describeNode(target)}`);
  }

  // type
  if (type != FieldType.OPTION && fieldIn?.kind == null) {
    // default to Text if no type given
    fieldIn = { ...fieldIn, kind: TypeKind.STRUCT, benchType: BenchType.TEXT };
  } else if (kind != null) {
    // override kind if forced
    fieldIn = { ...fieldIn, kind };
  }

  // name
  let name: string;
  if (fieldIn?.name != null) {
    name = fieldIn.name;
  } else if (type != FieldType.OPTION) {
    // make name unique (bumping number if needed)
    name = getFieldNameFromType(graph, fieldIn!);
    const siblings = graph.getChildren(parentPtr, NodeType.FIELD);
    let i = 2;
    while (siblings.some((s) => s.name == name)) {
      name = `${name}${i++}`;
    }
  } else {
    name = makeNodeName(graph, { metatype: ObjectType.FIELD, parentPtr, type: type });
  }

  // assign color icon if it's an option :FieldIcon
  if (fieldIn?.icon == null) {
    if (type == FieldType.OPTION) {
      const occupiedColors = siblings.map((f) => f.icon?.color?.type ?? ColorType.GRAY);
      const colorType = getRandomColorType({ except: occupiedColors });
      fieldIn = { ...fieldIn, icon: makeIcon({ faName: "fas fa-circle-small", color: colorType }) };
    } else {
      fieldIn = {
        ...fieldIn,
        icon: getNodeIcon({ metatype: ObjectType.FIELD, ...fieldIn }, { defaultToUndefined: true }),
      };
    }
  }

  const field = tx.create({
    name,
    type: type,
    ...fieldIn,
    // overwrite non-required properties
    id: undefined,
    ck: undefined,
    metatype: NodeType.FIELD,
    parentPtr,
    packagePtr: target.packagePtr,
    orderKey,
  });
  return field;
}

/** Updates the field type to a new type identity. */
export function updateFieldType(tx: Transaction, graph: ReadNodeGraph, field: FieldData, type: TypeIdentity | null) {
  const update: Partial<FieldData> = {};
  // update changed properties
  for (const key of [
    "kind",
    "primitiveType",
    "benchType",
    "baseTypePtr",
    "format",
    "condition",
    "constraint",
  ] as (keyof TypeIdentity)[]) {
    if (field[key] != type?.[key]) {
      update[key] = type?.[key];
    }
  }

  if (type != null) {
    // update name if it was generated
    const oldName = getFieldNameFromType(graph, field);
    if (field.name.startsWith(oldName)) {
      // make it unique (bumping number if needed)
      update.name = getFieldNameFromType(graph, type);
      const siblings = graph.getChildren(field.parentPtr!, NodeType.FIELD);
      let i = 2;
      while (siblings.some((s) => s.name == update.name)) {
        update.name = `${update.name}${i++}`;
      }
    }

    // update icon if it was the default one
    const oldIcon = getNodeIcon(field);
    if (field.icon == null || (oldIcon != null && contentEquals(field.icon, oldIcon))) {
      update.icon = getTypeIcon(type);
    }
  }

  tx.update(field, update, { debounce: "tick" });
}

/** Gets the most appropriate 'title' field from the given fields. Can be text, number, or anything simple to render. */
export function getTitleField(fields: FieldData[]): { field: FieldData | undefined; idx: number | undefined } {
  let titleIdx = fields.findIndex((f) => f.primitiveType == PrimitiveType.STRING);
  if (titleIdx == null) titleIdx = fields.findIndex((f) => typeIsNumeric(f));
  return { field: titleIdx != null ? fields[titleIdx] : undefined, idx: titleIdx };
}

import { blockToType } from "@/language/block";
import {
  getTkB64FromCk,
  getTkB64FromPtr,
  isNodeType,
  NAME_CONSTRAINT,
  padCkFromTkB64,
  RUNNABLE_BLOCK_TYPES,
  TITLE_CONSTRAINT,
  toCamelName,
  TYPE_BLOCK_TYPES,
} from "@/language/const";
import { getEnumTitle } from "@/language/enum";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName, moveNode } from "@/language/node";
import { getOrderKey } from "@/language/order";
import { newChangeId, type Transaction } from "@/language/transaction";
import {
  BenchType,
  BlockData,
  BlockType,
  ColorType,
  EnumType,
  FieldData,
  FieldType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyReferenceData,
  ActionData,
  StructType,
  TypeConstraintData,
  TypeData,
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
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { supergraph } from "@/system/globals";
import { DragContent, MultiAnchor } from "@/ui/drag";
import { getNodeIcon, getTypeIcon, makeIcon } from "@/ui/icon";
import { getRandomColorType } from "@/ui/style";
import { assertNever, decodeB64VLQ, encodeB64VLQ } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
import { Ref } from "vue";

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
  constraint: makeTypeConstraint(TITLE_CONSTRAINT),
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
    value = getTkB64FromPtr(type.baseTypePtr!);
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
    const baseTypePtr = { metatype: ObjectType.NODE_REFERENCE, nodeType: NodeType.BLOCK, ck: padCkFromTkB64(value) };
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

export const LETTER_BY_FIELD_TYPE: Partial<Record<FieldType, string>> = {
  [FieldType.INPUT]: "I",
  [FieldType.OUTPUT]: "O",
  [FieldType.VARIABLE]: "V",
  [FieldType.MEMBER]: "M",
};
export const FIELD_TYPE_BY_LETTER: Record<string, FieldType> = {
  I: FieldType.INPUT,
  O: FieldType.OUTPUT,
  V: FieldType.VARIABLE,
  M: FieldType.MEMBER,
};

/** Gets the eternal storage key for values of this type identity. :FieldStorageKey */
export function getStorageKey(field: FieldData, fieldType?: TypeIdentity): string {
  fieldType = fieldType ?? field;
  if (field.ck == null) throw new Error(`missing ck for type ${describeTypeIdentity(field)}`);
  return `${LETTER_BY_FIELD_TYPE[field.type]}${getTkB64FromCk(field.ck)}${encodeTypeIdentity(fieldType)}`;
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
  )
    return true;
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
export function typeIsNumeric(type: { kind: TypeKind } & Partial<TypeData>): boolean {
  return type.primitiveType != null && type.primitiveType >= 2 && type.primitiveType < 20;
}

// NOTE :Architecture: :TypeResolution in frontend should probably happen reactively in a dedicated.. something.

/** Resolves the actual fields of the given type. :TypeResolution */
export function resolveFields(type: TypeIdentity, graph: ReadNodeGraph): FieldData[] {
  if (type.baseTypePtr == null) return [];
  const fields = graph.getChildren(type.baseTypePtr, NodeType.FIELD);
  if (type.baseFieldTypes == null || type.baseFieldTypes.length == 0) {
    return fields.filter((f) => f.type != FieldType.OPTION);
  } else {
    return fields.filter((f) => type.baseFieldTypes!.includes(f.type));
  }
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
      | ActionData
      | TypedNodeReferenceData<NodeType.ACTION>;
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
    parentPtr = toNodeRef(target);
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
  } else if (isNode(target, NodeType.ACTION)) {
    if (fieldIn?.type == null) throw new Error(`missing type for action field: ${describeNode(target)}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toNodeRef(target);
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
    name = getTypeName(fieldIn!);
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

/** Gets the update that would be applied to a field to update its type. */
export function getFieldTypeUpdate(
  graph: ReadNodeGraph,
  field: FieldData,
  type: TypeIdentity | null,
): Partial<FieldData> {
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
    "isList",
    "isRequired",
  ] as (keyof TypeIdentity)[]) {
    if (field[key] != type?.[key]) {
      update[key] = type?.[key];
    }
  }

  if (type != null) {
    // update name if it was generated
    const oldName = getTypeName(field);
    if (field.name.startsWith(oldName)) {
      // make it unique (bumping number if needed)
      update.name = getTypeName(type);
      const siblings = graph.getChildren(field.parentPtr!, NodeType.FIELD);
      let i = 2;
      while (siblings.some((s) => s.name == update.name)) {
        update.name = `${update.name}${i++}`;
      }
    }
  }

  return update;
}

/** Updates the field type to a new type identity. */
export function updateFieldType(tx: Transaction, graph: ReadNodeGraph, field: FieldData, type: TypeIdentity | null) {
  const update = getFieldTypeUpdate(graph, field, type);
  tx.update(field, update, { debounce: "tick" });
}

/** Gets the most appropriate 'title' field from the given fields. Can be text, number, or anything simple to render. */
export function getTitleField(fields: FieldData[]): { field: FieldData | undefined; idx: number | undefined } {
  let titleIdx = fields.findIndex((f) => f.primitiveType == PrimitiveType.STRING);
  if (titleIdx == null) titleIdx = fields.findIndex((f) => typeIsNumeric(f));
  return { field: titleIdx != null ? fields[titleIdx] : undefined, idx: titleIdx };
}

/** Handle drag/drop for Fields. */
export function useFieldList(options: {
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
  fieldType: Ref<FieldType>;
  base: Ref<BlockData | ActionData | null>;
}) {
  const { graph, txFactory, fieldType, base: block } = options;

  /** Whether the given drag content is allowed to be dropped on the target. */
  function allowDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent): boolean {
    if (dragged.kind != "node" && dragged.kind != "selection") return false;
    return dragged.nodes.every((node) => {
      node = graph.getOrError(node);
      if (isNode(node, NodeType.FIELD) && (node.type == FieldType.OPTION) == (fieldType.value == FieldType.OPTION)) {
        return true;
      } else if (
        isNode(node, NodeType.BLOCK) &&
        block.value?.type != BlockType.CHOICE &&
        TYPE_BLOCK_TYPES.includes(node.type)
      ) {
        return true;
      } else {
        return false;
      }
    });
  }

  function onDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
    if (dragged.kind != "node" && dragged.kind != "selection") return;
    const tx = txFactory().with({ change: { key: newChangeId(), title: "Move" } });
    const target = targetId != null ? graph.get({ id: targetId }) : null;
    for (let i = 0; i < dragged.nodes.length; i++) {
      const node = graph.getOrError(dragged.nodes[i]);
      if (isNode(node, NodeType.FIELD)) {
        // move field
        if (target != null) {
          if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
          moveNode(tx, graph, node, {
            anchor: i == 0 ? anchor : "after",
            target: i == 0 ? target : graph.getOrError(dragged.nodes[i - 1]),
          });
        } else {
          moveNode(tx, graph, node, { anchor: "center", target: block.value! });
        }
        if (node.type != fieldType.value) {
          tx.update(node, { type: fieldType.value ?? undefined }, { debounce: "tick" });
        }
      } else if (isNode(node, NodeType.BLOCK)) {
        // add field with block type
        const type = blockToType(node);
        const fieldIn = { ...type, type: fieldType.value! };
        if (target != null) {
          if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
          createField(tx, graph, {
            field: fieldIn,
            anchor: i == 0 ? anchor : "after",
            target: i == 0 ? target : (graph.getOrError(dragged.nodes[i - 1]) as FieldData),
          });
        } else {
          createField(tx, graph, { field: fieldIn, anchor: "inside", target: block.value! });
        }
      }
    }
  }

  return { allowDrop, onDrop };
}

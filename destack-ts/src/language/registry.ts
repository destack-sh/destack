import type {
  EnumClass,
  EnumType,
  NodeClass,
  NodeType,
  StoreType,
  StructClass,
  StructType,
  TraitClass,
  TraitType,
} from "@destack/language/core/builtin";

export const NODE_CLASS_BY_TYPE: Record<NodeType, NodeClass> = {} as any;
export const NODE_TYPE_BY_CLASS: Map<NodeClass, NodeType> = new Map();
export function registerNodeClass(nodeType: NodeType, nodeClass: NodeClass): void {
  NODE_CLASS_BY_TYPE[nodeType] = nodeClass;
  NODE_TYPE_BY_CLASS.set(nodeClass, nodeType);
}

export const NODE_TYPES_BY_PRIMARY_STORE_TYPE: Record<StoreType, NodeType[]> = {} as any;
export const NODE_TYPES_BY_TRAIT_TYPE: Record<TraitType, NodeType[]> = {} as any;

export const TRAIT_CLASS_BY_TYPE: Record<TraitType, TraitClass> = {} as any;
export const TRAIT_TYPE_BY_CLASS: Map<TraitClass, TraitType> = new Map();
export function registerTraitClass(traitType: TraitType, traitClass: TraitClass): void {
  TRAIT_CLASS_BY_TYPE[traitType] = traitClass;
  TRAIT_TYPE_BY_CLASS.set(traitClass, traitType);
}

export const STRUCT_CLASS_BY_TYPE: Record<StructType, StructClass> = {} as any;
export const STRUCT_TYPE_BY_CLASS: Map<StructClass, StructType> = new Map();
export function registerStructClass(structType: StructType, structClass: StructClass): void {
  STRUCT_CLASS_BY_TYPE[structType] = structClass;
  STRUCT_TYPE_BY_CLASS.set(structClass, structType);
}

export const ENUM_CLASS_BY_TYPE: Record<EnumType, EnumClass> = {} as any;
export const ENUM_TYPE_BY_CLASS: Map<EnumClass, EnumType> = new Map();
export function registerEnumClass(enumType: EnumType, enumClass: EnumClass): void {
  ENUM_CLASS_BY_TYPE[enumType] = enumClass;
  ENUM_TYPE_BY_CLASS.set(enumClass, enumType);
}

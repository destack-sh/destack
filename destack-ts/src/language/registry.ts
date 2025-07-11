import type {
  EnumClass,
  EnumType,
  NodeClass,
  NodeDefinitionReference,
  NodeType,
  StoreKey,
  StructClass,
  StructType,
  TraitClass,
  TraitType,
  Type,
} from "@destack/language";

// basic class/type mappings

export const NODE_CLASS_BY_TYPE: Record<NodeType, NodeClass> = {} as any;
export const NODE_TYPE_BY_CLASS: Map<NodeClass, NodeType> = new Map();
export function registerNodeClass(nodeType: NodeType, nodeClass: NodeClass): void {
  NODE_CLASS_BY_TYPE[nodeType] = nodeClass;
  NODE_TYPE_BY_CLASS.set(nodeClass, nodeType);
}

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

// extra computed stuff

export const NODE_TYPES_BY_PRIMARY_STORE_KEY: Record<StoreKey, NodeType[]> = {} as any;
export const NODE_TYPES_BY_TRAIT_TYPE: Record<TraitType, NodeType[]> = {} as any;

export const PARENT_TYPES_BY_NODE_TYPE: Record<NodeType, NodeType[]> = {} as any;

export const NODE_TYPE_SCALAR_BY_TYPE: Record<NodeType, Type> = {} as any;

/** Get the known NodeTypes for a set of StoreKeys. */
export function getNodeTypesForStores(storeKeys: StoreKey[]): NodeType[] {
  const nodeTypes: NodeType[] = [];
  for (const type of storeKeys) {
    for (const nodeType of NODE_TYPES_BY_PRIMARY_STORE_KEY[type]) {
      if (!nodeTypes.includes(nodeType)) {
        nodeTypes.push(nodeType);
      }
    }
  }
  return nodeTypes;
}

/** Get the known (inherited, concrete) subdefinitions for a NodeType (including self). */
export function getSubdefinitionsForNodeType(nodeType: NodeType): NodeDefinitionReference[] {
  const nodeClass = NODE_CLASS_BY_TYPE[nodeType];
  const nodeDefinition = nodeClass.__definition__;
  if (nodeDefinition.inheritedBy.length === 0) {
    return [nodeClass.__definitionReference];
  }
  const subdefinitions: NodeDefinitionReference[] = [];
  if (!nodeDefinition.isAbstract) {
    subdefinitions.push(nodeClass.__definitionReference);
  }
  for (const subnodeType of nodeDefinition.inheritedBy) {
    const subnodeClass = NODE_CLASS_BY_TYPE[subnodeType];
    const subnodeDefinition = subnodeClass.__definition__;
    if (!subnodeDefinition.isAbstract) {
      subdefinitions.push(subnodeClass.__definitionReference);
    }
  }
  return subdefinitions;
}

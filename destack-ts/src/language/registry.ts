import { EnumClass, EnumType, NodeClass, NodeType, StructClass, StructType } from "@destack/language/core/builtin";

export const NODE_CLASS_BY_TYPE: Record<NodeType, NodeClass> = {} as any;
export function registerNodeClass(nodeType: NodeType, nodeClass: NodeClass): void {
  if (NODE_CLASS_BY_TYPE[nodeType]) {
    throw new Error(`duplicate node class for ${nodeType}`);
  }
  NODE_CLASS_BY_TYPE[nodeType] = nodeClass;
}

export const STRUCT_CLASS_BY_TYPE: Record<StructType, StructClass> = {} as any;
export function registerStructClass(structType: StructType, structClass: StructClass): void {
  if (STRUCT_CLASS_BY_TYPE[structType]) {
    throw new Error(`duplicate struct class for ${structType}`);
  }
  STRUCT_CLASS_BY_TYPE[structType] = structClass;
}

export const ENUM_CLASS_BY_TYPE: Record<EnumType, EnumClass> = {} as any;
export function registerEnumClass(enumType: EnumType, enumClass: EnumClass): void {
  if (ENUM_CLASS_BY_TYPE[enumType]) {
    throw new Error(`duplicate enum class for ${enumType}`);
  }
  ENUM_CLASS_BY_TYPE[enumType] = enumClass;
}

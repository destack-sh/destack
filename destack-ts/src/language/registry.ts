import { EnumClass, EnumType, NodeClass, NodeType, StructClass, StructType } from "@destack/language/core/builtin";

export const NODE_CLASS_BY_TYPE: Record<NodeType, NodeClass> = {} as any;
export function registerNodeClass(nodeType: NodeType, nodeClass: NodeClass): void {
  NODE_CLASS_BY_TYPE[nodeType] = nodeClass;
}

export const STRUCT_CLASS_BY_TYPE: Record<StructType, StructClass> = {} as any;
export function registerStructClass(structType: StructType, structClass: StructClass): void {
  STRUCT_CLASS_BY_TYPE[structType] = structClass;
}

export const ENUM_CLASS_BY_TYPE: Record<EnumType, EnumClass> = {} as any;
export function registerEnumClass(enumType: EnumType, enumClass: EnumClass): void {
  ENUM_CLASS_BY_TYPE[enumType] = enumClass;
}

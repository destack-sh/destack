import {
  BenchType,
  NodeType,
  SomeNodeData,
  StructType,
  type AnyNodeData,
  type AnyStructData,
  type StructTypeMapping,
  type NodeTypeMapping,
  NodeSource,
  NodeReferenceData,
  type AnyPropertyType,
  NODE_PROPERTY_ENUM_BY_TYPE,
} from "@/proto/wire";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/system/lang";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { v4 } from "uuid";
import { toRef, type MaybeRef, type Ref, computed } from "vue";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const BENCH_TYPE_NAME: Record<BenchType, string> = reverseRecord(BenchType);

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

export function makeStruct<T extends StructType>(
  data: Omit<StructTypeMapping[T], "metatype" | "id"> & { metatype: T },
): StructTypeMapping[T] {
  const struct = {
    id: newStructId(),
    ...data,
  };
  return struct as unknown as StructTypeMapping[T];
}

export function makeDefaultStruct<T extends StructType>(metatype: T): StructTypeMapping[T] {
  throw new Error("not yet implemented");
}

export function newNodeCk(): string {
  return v4();
}

export function makeNode<T extends NodeType>(
  data: Omit<NodeTypeMapping[T], "metatype" | "id" | "ck" | "revision" | "source" | "setProperties"> & { metatype: T },
): NodeTypeMapping[T] {
  const node = {
    ...data,
    source: NodeSource.STORE,
    setProperties: [],
  };
  return node as unknown as NodeTypeMapping[T];
}

export function isNode(value: AnyNodeData | AnyStructData): value is AnyNodeData {
  return value.metatype < 500;
}

export function isStruct(value: AnyNodeData | AnyStructData): value is AnyStructData {
  return value.metatype >= 500;
}

export function nodeReference<T extends NodeType>(nodeType: T, id: string): NodeReferenceData {
  return { metatype: BenchType.NODE_REFERENCE, type: nodeType, id };
}


export function toNodeReference(node: null): null;
export function toNodeReference(node: AnyNodeData): NodeReferenceData;
export function toNodeReference(node: AnyNodeData | null): NodeReferenceData | null {
  if (!node) return null;
  const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const reference: NodeReferenceData = {
    metatype: BenchType.NODE_REFERENCE,
    type: node.metatype as unknown as NodeType,
    id: node.id,
  };
  if ("bench" in allProperties && node.parentPtr) {
    reference.benchId = node.parentPtr.benchId;
  }
  if ("ck" in allProperties) {
    reference.ck = (node as { ck: string }).ck;
    if (node.metatype in BASED_NODE_TYPES) {
      const base = getBaseFromNode(node);
      if (base != null) {
        reference.baseCk = base.ck;
        reference.baseBenchId = base.benchId;
      }
    }
  }
  return reference;
}

export function toNodeReferenceRef(node: MaybeRef<AnyNodeData | null>): Ref<NodeReferenceData | null> {
  const nodeRef = toRef(node) as Ref<AnyNodeData | null>;
  return computed(() => toNodeReference(nodeRef.value!)); // TODO :Cleanup: shouldn't have to ! to type check here?
}

export function toBenchType(type: NodeType | StructType): BenchType {
  return type as unknown as BenchType;
}

export function toProtoOneOf<T extends object>(value: T): T & { oneofKind: keyof T } {
  /** Turn { [key]: value } into { key: value, oneofKind: key } for protobuf unions */
  const key = Object.keys(value)[0] as keyof T;
  return { ...value, oneofKind: key };
}

export function wrapSomeNode(node: AnyNodeData): SomeNodeData {
  const fieldName = toCasing(BENCH_TYPE_NAME[node.metatype], Casing.SNAKE);
  return { node: { [fieldName]: node, oneofKind: fieldName as any } };
}

export function unwrapSomeNode(node: SomeNodeData): AnyNodeData {
  const oneOfKind = node.node.oneofKind;
  if (!oneOfKind) throw new Error("missing oneofKind");
  return (node.node as any)[oneOfKind];
}

export function toRobustJson(value: AnyStructData | AnyNodeData): { [key: string]: any } {
  throw new Error("not yet implemented");
}

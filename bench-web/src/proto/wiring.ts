import {
  BenchType,
  MESSAGE_TYPE_BY_BENCH_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeSource,
  NodeType,
  PROPERTY_ENUM_BY_TYPE,
  STRUCT_PROPERTY_ENUM_BY_TYPE,
  SomeNodeData,
  StructType,
  type AnyNodeData,
  type AnyPropertyType,
  type AnyStructData,
  type AnyTypeMapping,
  type NodeTypeMapping,
  type StructTypeMapping,
} from "@/proto/wire";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/system/lang";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { ScalarType, type FieldInfo } from "@protobuf-ts/runtime";
import { v4, v5 } from "uuid";
import { computed, toRef, type MaybeRef, type Ref } from "vue";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const BENCH_TYPE_NAME: Record<BenchType, string> = reverseRecord(BenchType);

export type TypedNodeReferenceData<T extends NodeType> = NodeReferenceData & { type: T };

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

export function makeStruct<T extends StructType>(
  data: Omit<StructTypeMapping[T], "metatype" | "id"> & { metatype: T },
): StructTypeMapping[T] {
  const properties = STRUCT_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as BenchType]!;
  let struct;
  if ("id" in properties) {
    struct = {
      id: newStructId(),
      ...data,
    };
  } else {
    struct = { ...data };
  }
  return struct as unknown as StructTypeMapping[T];
}

export const SCALAR_DEFAULTS: Partial<Record<ScalarType, any>> = {
  [ScalarType.DOUBLE]: 0,
  [ScalarType.FLOAT]: 0,
  [ScalarType.INT32]: 0,
  [ScalarType.INT64]: 0,
  [ScalarType.UINT32]: 0,
  [ScalarType.UINT64]: 0,
  [ScalarType.SINT32]: 0,
  [ScalarType.SINT64]: 0,
  [ScalarType.BOOL]: false,
  [ScalarType.STRING]: "",
  [ScalarType.BYTES]: new Uint8Array(),
};

/**
 * Initializes the proto with default values so it can be serialized to a protobuf message.
 * NOTE: proto default values are not semantically correct, this is just for to patch not-semantically-required fields.
 */
export function makeDefaultProto<T extends BenchType>(metatype: T): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType = PROPERTY_ENUM_BY_TYPE[metatype as unknown as BenchType]!;
  const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[metatype as unknown as BenchType]!;
  let ord = 0;
  const proto = { metatype } as AnyTypeMapping[T];
  for (const propName of Object.keys(allProperties)) {
    if (!isNaN(Number(propName))) continue; // skip numeric keys
    if (propName == "metatype") continue; // already set
    const field = messageType.fields[ord];
    (proto as any)[propName] = getDefaultProtoValue(field);
    ord += 1;
  }
  return proto;
}

export function getDefaultProtoValue(field: FieldInfo): any {
  if (field.repeat) {
    return [];
  } else if (field.kind == "scalar") {
    return SCALAR_DEFAULTS[field.T];
  } else {
    return undefined; // is this correct?
  }
}

export function newNodeCk(): string {
  return v4();
}

export function newNodeId(): string {
  return v4();
}

export function newNodeIdFromCk(packageId: string, ck: string): string {
  return v5(packageId, ck);
}

/**
 * Create a node from the given data and assign it a new id (and ck if in package).
 */
export function makeNode<T extends NodeType>(
  data: Omit<NodeTypeMapping[T], "metatype" | "id" | "ck" | "revision" | "source" | "setProperties"> & { metatype: T },
): NodeTypeMapping[T] {
  const node = {
    ...data,
    source: NodeSource.STORE,
    setProperties: [],
  } as unknown as NodeTypeMapping[T];
  const properties = NODE_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as BenchType]!;
  if ("packagePtr" in properties) {
    if (!("packagePtr" in data) || data.packagePtr == null)
      throw new Error(`missing packagePtr to make in-package node ${NodeType[data.metatype]}`);
    if ((node as any).ck == null) (node as any).ck = newNodeCk();
    node.id = newNodeIdFromCk((data.packagePtr as NodeReferenceData).id!, (node as any).ck);
  } else {
    node.id = newNodeId();
  }

  return node;
}

export function isNode(value: AnyNodeData | AnyStructData): value is AnyNodeData {
  return value.metatype < 500;
}

export function isStruct(value: AnyNodeData | AnyStructData): value is AnyStructData {
  return value.metatype >= 500;
}

export function nodeReference<T extends NodeType>(
  nodeType: T,
  id: string,
  benchId?: string,
): TypedNodeReferenceData<T> {
  return { metatype: BenchType.NODE_REFERENCE, type: nodeType, id, benchId };
}

export function toNodeReference(node: null): null;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T]): TypedNodeReferenceData<T>;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T] | null): TypedNodeReferenceData<T> | null {
  if (!node) return null;
  const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const reference: TypedNodeReferenceData<T> = {
    metatype: BenchType.NODE_REFERENCE,
    type: node.metatype as unknown as T,
    id: node.id,
  };
  if ("packagePtr" in allProperties && "packagePtr" in node) {
    reference.benchId = node.packagePtr?.benchId;
  } else {
    reference.benchId = node.parentPtr?.benchId;
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

export function toNodeReferenceRef<T extends NodeType>(
  node: MaybeRef<NodeTypeMapping[T] | null>,
): Ref<TypedNodeReferenceData<T> | null> {
  const nodeRef = toRef(node) as Ref<NodeTypeMapping[T] | null>;
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

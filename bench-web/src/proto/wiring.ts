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
  PropertyReferenceData,
} from "@/proto/wire";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/system/lang";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { ScalarType, type FieldInfo, type IMessageType, MessageType } from "@protobuf-ts/runtime";
import { v4, v5 } from "uuid";
import { computed, toRef, type MaybeRef, type Ref } from "vue";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const BENCH_TYPE_NAME: Record<BenchType, string> = reverseRecord(BenchType);

export type TypedNodeReferenceData<T extends NodeType> = NodeReferenceData & { type: T };
export type AnyNodeReferenceData = NodeReferenceData | TypedNodeReferenceData<NodeType>;

/** Short string representation of the node (pointer) */
export function describeNode(node: AnyNodeData | NodeReferenceData | TypedNodeReferenceData<any>): string {
  const nodeParts: string[] = [`id=${node.id}`];
  if ("ck" in node) nodeParts.push(`ck=${node.ck}`);
  if ("revision" in node) nodeParts.push(`r=${node.revision}`);
  if ("name" in node) nodeParts.push(`name=${node.name}`);
  if ("slug" in node) nodeParts.push(`slug=${node.slug}`);
  if ("title" in node) nodeParts.push(`title=${node.title}`);
  const type = node.metatype == BenchType.NODE_REFERENCE ? (node as NodeReferenceData).type : node.metatype;
  const typeName = toCasing(NodeType[type], Casing.CAMEL);
  return `${typeName}:[${nodeParts.join(", ")}]`;
}

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

/** Makes a struct with an identity (if required) */
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

/** Makes a struct from partial properties */
export function makeDefaultStruct<T extends StructType>(
  data: Partial<Omit<StructTypeMapping[T], "metatype" | "id">> & { metatype: T },
): StructTypeMapping[T] {
  const allProperties: AnyPropertyType = STRUCT_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as BenchType]!;
  const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[data.metatype as unknown as BenchType]!;
  let ord = 1; // skip metatype
  const struct = { ...data } as unknown as StructTypeMapping[T];
  for (const propName of Object.keys(allProperties)) {
    if (!isNaN(Number(propName))) continue; // skip numeric keys
    if (propName == "metatype") continue; // already set
    if ((struct as any)[propName] == null) {
      const field = messageType.fields[ord];
      (struct as any)[propName] = getDefaultProtoValue(field);
    }
    ord += 1;
  }
  return struct;
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

/** Initializes the Bench type proto with default proto values. */
export function makeDefaultBenchProto<T extends BenchType>(metatype: T): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType = PROPERTY_ENUM_BY_TYPE[metatype as unknown as BenchType]!;
  const messageType = MESSAGE_TYPE_BY_BENCH_TYPE[metatype as unknown as BenchType]!;
  let ord = 1; // skip metatype
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

/** Initializes any proto message with default proto values. */
export function makeDefaultProto<T extends object>(messageType: MessageType<T>): T {
  const proto = {} as T;
  for (const field of messageType.fields) {
    (proto as any)[field.name] = getDefaultProtoValue(field);
  }
  return proto;
}

/** Gets the default 'empty' value for the property of a proto message */
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
 * Create a node from the given data and assign it an id (and ck if in package).
 * NOTE: id/ck are only assigned if not present. To copy, use copyNode.
 */
export function makeNode<T extends NodeType>(
  data: Omit<NodeTypeMapping[T], "metatype" | "id" | "ck" | "revision" | "source" | "setProperties"> & { metatype: T },
  options?: { omit: (keyof NodeTypeMapping[T])[] },
): NodeTypeMapping[T] {
  const node = {
    ...data,
    source: NodeSource.STORE,
    revision: 0,
    setProperties: [],
  } as unknown as NodeTypeMapping[T];

  if (!options?.omit?.includes("id")) {
    const properties = NODE_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as BenchType]!;
    if ("packagePtr" in properties) {
      if (!("packagePtr" in data) || data.packagePtr == null)
        throw new Error(`missing packagePtr to make in-package node ${NodeType[data.metatype]}`);
      if ((node as any).ck == null) (node as any).ck = newNodeCk();
      node.id = newNodeIdFromCk((data.packagePtr as NodeReferenceData).id!, (node as any).ck);
    } else {
      node.id = newNodeId();
    }
  }

  return node;
}

/**
 * Copies all data properties of the node with a new identity.
 */
export function copyNode<T extends AnyNodeData>(node: T): T {
  const copy = { ...node, id: undefined, ck: undefined, source: NodeSource.STORE, revision: 0, setProperties: [] };
  return makeNode(copy) as T;
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
  meta?: {
    ck?: string;
    benchId?: string;
    baseCk?: string;
    baseBenchId?: string;
  },
): TypedNodeReferenceData<T> {
  const ptr = { metatype: BenchType.NODE_REFERENCE, type: nodeType, ...meta, id };
  if (nodeType == NodeType.BENCH && ptr.benchId == null) ptr.benchId = id;
  return ptr;
}

export function propertyReference<T extends BenchType>(metatype: T, id: number): PropertyReferenceData {
  return { metatype: BenchType.PROPERTY_REFERENCE, type: metatype, id };
}

export function toNodeReference(node: null): null;
export function toNodeReference<T extends NodeType>(node: TypedNodeReferenceData<T>): TypedNodeReferenceData<T>;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T]): TypedNodeReferenceData<T>;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T] | null): TypedNodeReferenceData<T> | null {
  if (!node) return null;
  if (node.metatype == BenchType.NODE_REFERENCE) return node as unknown as TypedNodeReferenceData<T>;
  const allProperties: AnyPropertyType = NODE_PROPERTY_ENUM_BY_TYPE[node.metatype]!;
  const reference: TypedNodeReferenceData<T> = {
    metatype: BenchType.NODE_REFERENCE,
    type: node.metatype as unknown as T,
    id: node.id,
  };
  // benchId
  if (node.metatype == BenchType.BENCH) {
    reference.benchId = node.id;
  } else if ("packagePtr" in allProperties && "packagePtr" in node) {
    reference.benchId = node.packagePtr?.benchId;
  } else {
    reference.benchId = node.parentPtr?.benchId;
  }
  // ck
  if ("ck" in allProperties) {
    reference.ck = (node as { ck: string }).ck;
  }
  // base
  if (node.metatype in BASED_NODE_TYPES) {
    const base = getBaseFromNode(node);
    if (base != null) {
      reference.baseCk = base.ck;
      reference.baseBenchId = base.benchId;
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

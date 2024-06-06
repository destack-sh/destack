import {
  ObjectType,
  MESSAGE_TYPE_BY_OBJECT_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  PROPERTY_ENUM_BY_TYPE,
  PropertyReferenceData,
  STRUCT_PROPERTY_ENUM_BY_TYPE,
  SomeNodeData,
  StructType,
  type AnyNodeData,
  type AnyPropertyType,
  type AnyStructData,
  type AnyTypeMapping,
  type NodeTypeMapping,
  type StructTypeMapping,
  PackageData,
  EditType,
  EditData,
} from "@/proto/wire";
import { BASED_NODE_TYPES, TIMED_NODE_TYPES, getBaseFromNode, toCamelName } from "@/system/lang";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { uuidt } from "@/utils/uuidt";
import { MessageType, ScalarType, type FieldInfo } from "@protobuf-ts/runtime";
import { v4, v5 } from "uuid";
import { computed, toRef, type MaybeRef, type Ref } from "vue";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const OBJECT_TYPE_NAME: Record<ObjectType, string> = reverseRecord(ObjectType);

export type TypedNodeReferenceData<T extends NodeType> = NodeReferenceData & { type: T };
export type AnyNodeReferenceData = NodeReferenceData | TypedNodeReferenceData<NodeType>;
export type SomeNodeReferenceData<T extends NodeType> = NodeReferenceData | TypedNodeReferenceData<T>;

/** Short string representation of the node (pointer) */
export function describeNode(node: {
  metatype?: NodeType | ObjectType | any | null;
  parentPtr?: NodeReferenceData | null;
  type?: NodeType | ObjectType | any | null;
  id?: string | null;
  ck?: string | null;
  slug?: string | null;
  name?: string | null;
  title?: string | null;
}): string {
  const nodeParts: string[] = [`id=${node.id}`];
  const nodeType = node.metatype == ObjectType.NODE_REFERENCE ? (node as NodeReferenceData).type : node.metatype;
  if ("ck" in node) nodeParts.push(`ck=${node.ck}`);
  if ("revision" in node) nodeParts.push(`revision=${node.revision}`);
  if (node.name) nodeParts.push(`name='${node.name}'`);
  if (node.slug) nodeParts.push(`slug=${node.slug}`);
  if (node.metatype != ObjectType.NODE_REFERENCE && nodeType != null && node.type != null) {
    // coerce 'type' property into actual name
    const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[nodeType as unknown as ObjectType];
    if (messageType != null) {
      const field = messageType.fields.find((f) => f.name == "type");
      if (field?.kind == "enum") {
        nodeParts.push(`type=${field.T()[1][node.type] ?? node.type}`);
      }
    }
  }
  if (node.title) nodeParts.push(`title='${node.title}'`);
  if (node.parentPtr) nodeParts.push(`parent=${toCamelName(NodeType, node.parentPtr.type)}:${node.parentPtr.id}`);
  if ("benchId" in node) nodeParts.push(`benchId=${node.benchId}`);
  if ("benchCk" in node) nodeParts.push(`benchId=${node.benchCk}`);
  const typeName = nodeType == null ? "Node" : toCamelName(NodeType, nodeType);
  return `${typeName}:[${nodeParts.join(", ")}]`;
}

export function describeEdit(edit: Pick<EditData, "type" | "nodePtr" | "revision" | "properties">) {
  const editParts: string[] = [EditType[edit.type]];
  if (edit.nodePtr) editParts.push(describeNode(edit.nodePtr));
  if (edit.revision) editParts.push(`revision=${edit.revision}`);
  if (edit.properties) editParts.push(`properties=${Object.keys(edit.properties).join(",")}`);
  return editParts.join(" ");
}

export function newStructId(): number {
  /** Generates a positive 32-bit random integer */
  return Math.floor(Math.random() * 0x7fffffff);
}

/** Makes a struct with an identity (if required) */
export function makeStruct<T extends StructType>(
  data: Omit<StructTypeMapping[T], "metatype" | "id"> & { metatype: T },
): StructTypeMapping[T] {
  const properties = STRUCT_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as ObjectType]!;
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
  const allProperties = STRUCT_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as ObjectType];
  if (allProperties == null)
    throw new Error(`missing properties for struct type: ${data.metatype} (${typeof data.metatype})`);
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[data.metatype as unknown as ObjectType]!;
  let ord = 1; // skip metatype
  const struct = { ...data } as unknown as StructTypeMapping[T];
  for (const propName of Object.keys(allProperties)) {
    if (!isNaN(Number(propName))) continue; // skip numeric keys
    if (propName == "metatype") continue; // already set
    if ((struct as any)[propName] == null) {
      const field = messageType.fields[ord];
      const defaultValue = getDefaultProtoValue(field);
      if (defaultValue !== undefined) (struct as any)[propName] = defaultValue;
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
export function makeDefaultBenchProto<T extends ObjectType>(metatype: T): AnyTypeMapping[T] {
  const allProperties: AnyPropertyType = PROPERTY_ENUM_BY_TYPE[metatype as unknown as ObjectType]!;
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[metatype as unknown as ObjectType]!;
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
  } else if (field.opt) {
    return undefined;
  } else if (field.kind == "scalar") {
    return SCALAR_DEFAULTS[field.T];
  } else if (field.kind == "enum") {
    return 0;
  } else {
    return undefined; // NOTE :Robustness: is this correct?
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
  data: Partial<Omit<NodeTypeMapping[T], "metatype" | "id" | "ck" | "revision" | "source" | "setProperties">> & {
    metatype: T;
  },
  options?: { omit: (keyof NodeTypeMapping[T])[] },
): NodeTypeMapping[T] {
  const node = {
    ...data,
    revision: 0,
    setProperties: [],
  } as unknown as NodeTypeMapping[T];
  const properties = NODE_PROPERTY_ENUM_BY_TYPE[data.metatype as unknown as ObjectType]!;

  // assign id/ck/scope
  if (!options?.omit?.includes("id")) {
    if ("packagePtr" in properties) {
      if (!("packagePtr" in data) || data.packagePtr == null) {
        throw new Error(`missing packagePtr to make sub-package node ${NodeType[data.metatype]}`);
      }
      if ("ck" in properties) {
        // regular node in package
        if ((node as any).ck == null) {
          (node as any).ck = newNodeCk();
        }
        node.id = newNodeIdFromCk((data.packagePtr as NodeReferenceData).id!, (node as any).ck);
      } else {
        // 'timed' node with UUIDT
        if (!TIMED_NODE_TYPES.includes(data.metatype)) {
          throw new Error(`unexpected in-package node type ${NodeType[data.metatype]} without ck`);
        }
        node.id = uuidt();
      }
    } else {
      // out-of-package node
      node.id = newNodeId();
    }
  }
  if ("benchPtr" in properties && !Object.prototype.hasOwnProperty.call(node, "benchPtr")) {
    const benchId = node.parentPtr?.benchId ?? (node as any).packagePtr?.benchId;
    if (benchId == null) throw new Error(`missing benchId to make in-bench node ${NodeType[data.metatype]}`);
    (node as any).benchPtr = nodeReference(NodeType.BENCH, benchId);
  }

  // assign default values to unset properties
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[data.metatype as unknown as ObjectType]!;
  let ord = 1; // skip metatype
  for (const propName of Object.keys(properties)) {
    if (!isNaN(Number(propName))) continue; // skip numeric keys
    if (propName == "metatype") continue; // already set
    if (!Object.prototype.hasOwnProperty.call(node, propName)) {
      const field = messageType.fields[ord];
      const value = getDefaultProtoValue(field);
      if (value !== undefined) (node as any)[propName] = value;
    }
    ord += 1;
  }

  return node;
}

/**
 * Copies all data properties of the node with a new identity.
 */
export function copyNode<T extends AnyNodeData>(node: T): T {
  const copy = { ...node, id: undefined, ck: undefined, revision: 0, setProperties: [] };
  return makeNode(copy) as T;
}

export function isNode<T extends NodeType = NodeType>(
  value: any | null | undefined,
  type?: T,
): value is NodeTypeMapping[T] {
  if (value == null || typeof value != "object") return false;
  else if (type != null) return value.metatype == (type as unknown as ObjectType);
  else return value.metatype < 500;
}

export function isStruct<T extends StructType = StructType>(
  value: any | null | undefined,
  type?: T,
): value is StructTypeMapping[T] {
  if (value == null || typeof value != "object") return false;
  else if (type != null) return value.metatype == (type as unknown as ObjectType);
  else return value.metatype >= 500;
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
  const ptr = { metatype: ObjectType.NODE_REFERENCE, type: nodeType, ...meta, id, ck: meta?.ck ?? id };
  if (nodeType == NodeType.BENCH && ptr.benchId == null) ptr.benchId = id;
  return ptr;
}

export function typeNodeReference<T extends NodeType>(nodeType: T, ref: NodeReferenceData): TypedNodeReferenceData<T> {
  if (ref.type != nodeType) throw new Error(`expected ${NodeType[nodeType]}, got ${NodeType[ref.type]}`);
  return ref as TypedNodeReferenceData<T>;
}

export function typeNodeReferenceMaybe<T extends NodeType>(
  nodeType: T,
  ref: NodeReferenceData | undefined | null,
): TypedNodeReferenceData<T> | null {
  if (ref == null) return null;
  return typeNodeReference(nodeType, ref);
}

export function propertyReference<T extends ObjectType>(metatype: T, id: number): PropertyReferenceData {
  return { metatype: ObjectType.PROPERTY_REFERENCE, type: metatype, id };
}

export function toNodeReference(node: null): null;
export function toNodeReference<T extends NodeType>(node: TypedNodeReferenceData<T>): TypedNodeReferenceData<T>;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T]): TypedNodeReferenceData<T>;
export function toNodeReference<T extends NodeType>(node: NodeTypeMapping[T] | null): TypedNodeReferenceData<T> | null {
  if (!node) return null;
  if (node.metatype == ObjectType.NODE_REFERENCE) return node as unknown as TypedNodeReferenceData<T>;
  const reference: TypedNodeReferenceData<T> = {
    metatype: ObjectType.NODE_REFERENCE,
    type: node.metatype as unknown as T,
    id: node.id,
    ck: (node as any).ck ?? node.id,
  };
  // benchId
  if (node.metatype == ObjectType.BENCH) {
    reference.benchId = node.id;
  } else if ("packagePtr" in node) {
    reference.benchId = node.packagePtr?.benchId;
  } else if ("benchPtr" in node) {
    reference.benchId = node.benchPtr?.id;
  } else {
    reference.benchId = node.parentPtr?.benchId;
  }
  // base
  if (BASED_NODE_TYPES.includes(node.metatype as unknown as NodeType)) {
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
  return computed(() => toNodeReference(nodeRef.value!)); // NOTE :Cleanup: shouldn't have to ! to type check here?
}

export function toNodeReferenceInPackage<T extends NodeType>(
  ref: TypedNodeReferenceData<T> | NodeReferenceData,
  pkg: string | TypedNodeReferenceData<NodeType.PACKAGE> | NodeReferenceData | PackageData,
): TypedNodeReferenceData<T> {
  const packageId = typeof pkg == "string" ? pkg : pkg.id!;
  if (ref.ck == null) return ref as TypedNodeReferenceData<T>;
  else return { ...ref, id: newNodeIdFromCk(packageId, ref.ck), ck: ref.ck } as TypedNodeReferenceData<T>;
}

export function getNodeType(node: AnyNodeData | AnyNodeReferenceData): NodeType {
  if (node.metatype == ObjectType.NODE_REFERENCE) return (node as NodeReferenceData).type;
  else return node.metatype as unknown as NodeType;
}

export function toObjectType(type: NodeType | StructType): ObjectType {
  return type as unknown as ObjectType;
}

export function toProtoOneOf<T extends object>(value: T): T & { oneofKind: keyof T } {
  /** Turn { [key]: value } into { key: value, oneofKind: key } for protobuf unions */
  const key = Object.keys(value)[0] as keyof T;
  return { ...value, oneofKind: key };
}

export function wrapSomeNode(node: AnyNodeData): SomeNodeData {
  const fieldName = toCasing(OBJECT_TYPE_NAME[node.metatype], Casing.SNAKE);
  return { node: { [fieldName]: node, oneofKind: fieldName as any } };
}

export function unwrapSomeNode(node: SomeNodeData): AnyNodeData {
  const oneOfKind = node.node.oneofKind;
  if (!oneOfKind) throw new Error("missing oneofKind");
  return (node.node as any)[oneOfKind];
}

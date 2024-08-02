import {
  EditData,
  EditType,
  FileData,
  FileInfoProperty,
  FileReferenceData,
  GraphScopeData,
  MESSAGE_TYPE_BY_OBJECT_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyReferenceData,
  STRUCT_PROPERTY_ENUM_BY_TYPE,
  SecretData,
  SecretProperty,
  SecretReferenceData,
  SomeNodeData,
  StructType,
  ViewData,
  type AnyNodeData,
  type AnyPropertyType,
  type AnyStructData,
  type AnyTypeMapping,
  type NodeTypeMapping,
  type PropertyInfo,
  type StructTypeMapping,
} from "@/proto/wire";
import { BASED_NODE_TYPES, getBaseFromNode, toCamelName } from "@/language/utils";
import { reverseRecord } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { MessageType, reflectionMergePartial, ScalarType, type FieldInfo } from "@protobuf-ts/runtime";
import { v4 } from "uuid";

export const NODE_TYPE_NAME: Record<NodeType, string> = reverseRecord(NodeType);
export const STRUCT_TYPE_NAME: Record<StructType, string> = reverseRecord(StructType);
export const OBJECT_TYPE_NAME: Record<ObjectType, string> = reverseRecord(ObjectType);

export type TypedNodeReferenceData<T extends NodeType> = NodeReferenceData & { type: T };
export type AnyNodeReferenceData = NodeReferenceData | TypedNodeReferenceData<NodeType>;

export function makeScope(scope: Partial<GraphScopeData>): GraphScopeData {
  return { metatype: ObjectType.GRAPH_SCOPE, ...scope };
}

export const EMPTY_SCOPE = makeScope({});

export function describeScope(scope: GraphScopeData): string {
  const scopeParts: string[] = [];
  if (scope.benchId) scopeParts.push(`bench=${scope.benchId}`);
  if (scope.packageId) scopeParts.push(`package=${scope.packageId}`);
  return `[${scopeParts.join(", ")}]`;
}

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
  const nodeType = isNodeRef(node) ? node.type : node.metatype;
  if ("ck" in node) nodeParts.push(`ck=${node.ck}`);
  if ("revision" in node) nodeParts.push(`revision=${node.revision}`);
  if (node.name) nodeParts.push(`name='${node.name}'`);
  if (node.slug) nodeParts.push(`slug=${node.slug}`);
  if (!isNodeRef(node) && nodeType != null && node.type != null) {
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
    struct = { id: newStructId(), ...data };
  } else {
    struct = { ...data };
  }
  return struct as unknown as StructTypeMapping[T];
}

/** Fills unset properties in the given object with default values. */
export function fillDefaultObject<T extends AnyNodeData | AnyStructData>(obj: T) {
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[obj.metatype as unknown as ObjectType]!;
  return messageType.create(obj);
}

/** Makes an object from partial properties */
export function makeDefaultObject<T extends ObjectType>(
  data: Partial<Omit<AnyTypeMapping[T], "metatype" | "id">> & { metatype: T },
): AnyTypeMapping[T] {
  const messageType = MESSAGE_TYPE_BY_OBJECT_TYPE[data.metatype as unknown as ObjectType]!;
  return messageType.create(data);
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

/** Initializes any proto message with default proto values. */
export function makeDefaultProto<T extends object>(messageType: MessageType<T>): T {
  return messageType.create();
}

export function newNodeCk(): string {
  return v4();
}

export function newNodeId(): string {
  return v4();
}

export function isNode<T extends NodeType = NodeType>(
  value: any | null | undefined,
  type?: T,
): value is NodeTypeMapping[T] {
  if (value == null || typeof value != "object") return false;
  else if (type != null) return value.metatype == (type as unknown as ObjectType);
  else return NodeType[value.metatype] != null;
}

export function isStruct<T extends StructType = StructType>(
  value: any | null | undefined,
  type?: T,
): value is StructTypeMapping[T] {
  if (value == null || typeof value != "object") return false;
  else if (type != null) return value.metatype == (type as unknown as ObjectType);
  else return StructType[value.metatype] != null;
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

export function propertyInfo(metatype: ObjectType, id: number): PropertyInfo {
  return PROPERTY_INFOS_BY_TYPE[metatype]![id];
}

// references to some nodes have 'rich' metadata :RichReferences
export const NODE_REFERENCE_TYPES_BY_NODE_TYPE: Partial<Record<NodeType, StructType>> = {
  [NodeType.FILE]: StructType.FILE_REFERENCE,
  [NodeType.SECRET]: StructType.SECRET_REFERENCE,
};
export const NODE_REFERENCE_TYPES = [StructType.NODE_REFERENCE, StructType.FILE_REFERENCE, StructType.SECRET_REFERENCE];
export type SomeNodeReferenceData = NodeReferenceData | FileReferenceData | SecretReferenceData;

export function isPlainNodeRef(value: any | null | undefined): value is NodeReferenceData {
  return typeof value == "object" && value.metatype == ObjectType.NODE_REFERENCE;
}
export function isNodeRef(value: any | null | undefined): value is SomeNodeReferenceData {
  return typeof value == "object" && NODE_REFERENCE_TYPES.includes((value as any).metatype);
}

export function isNodeOrRef<T extends NodeType>(
  value: any | null | undefined,
  nodeType?: T,
): value is TypedNodeReferenceData<T> | NodeTypeMapping[T] {
  return typeof value == "object" && ((isNodeRef(value) && value.type == nodeType) || isNode(value, NodeType.BLOCK));
}

export function toPlainNodeRef(node: null): null;
export function toPlainNodeRef<T extends NodeType>(node: TypedNodeReferenceData<T>): TypedNodeReferenceData<T>;
export function toPlainNodeRef<T extends NodeType>(node: SomeNodeReferenceData): TypedNodeReferenceData<T>;
export function toPlainNodeRef<T extends NodeType>(node: NodeTypeMapping[T]): TypedNodeReferenceData<T>;
export function toPlainNodeRef<T extends NodeType>(node: NodeTypeMapping[T] | null): TypedNodeReferenceData<T> | null {
  if (!node) return null;
  if (isPlainNodeRef(node)) return node as unknown as TypedNodeReferenceData<T>;

  // coerce rich reference into plain
  if (isNodeRef(node)) {
    const ref: NodeReferenceData = {
      metatype: ObjectType.NODE_REFERENCE,
      type: node.type,
      id: node.id,
      ck: node.ck,
      benchId: node.benchId,
      baseCk: node.baseCk,
      baseBenchId: node.baseBenchId,
    };
    return ref as TypedNodeReferenceData<T>;
  }

  const ref: TypedNodeReferenceData<T> = {
    metatype: ObjectType.NODE_REFERENCE,
    type: node.metatype as unknown as T,
    id: node.id,
    ck: (node as any).ck ?? node.id,
  };
  // benchId
  if (node.metatype == ObjectType.BENCH) {
    ref.benchId = node.id;
  } else if ("packagePtr" in node) {
    ref.benchId = node.packagePtr?.benchId;
  } else if ("benchPtr" in node) {
    ref.benchId = node.benchPtr?.id;
  } else {
    ref.benchId = node.parentPtr?.benchId;
  }
  // base
  if (BASED_NODE_TYPES.includes(node.metatype as unknown as NodeType)) {
    const base = getBaseFromNode(node);
    if (base != null) {
      ref.baseCk = base.ck;
      ref.baseBenchId = base.benchId;
    }
  }
  return ref;
}

export function toNodeRef(node: null): null;
export function toNodeRef(node: FileData): FileReferenceData;
export function toNodeRef(node: SecretData): SecretReferenceData;
export function toNodeRef<T extends NodeType>(node: TypedNodeReferenceData<T>): TypedNodeReferenceData<T>;
export function toNodeRef<T extends NodeType>(node: NodeTypeMapping[T]): TypedNodeReferenceData<T>;
export function toNodeRef<T extends NodeType>(
  node: NodeTypeMapping[T] | null,
): TypedNodeReferenceData<T> | FileReferenceData | SecretReferenceData | null {
  if (!node) return null;
  if (isNodeRef(node)) return node as unknown as TypedNodeReferenceData<T>;

  const ref = toPlainNodeRef(node);

  // enrich reference properties :RichReferences
  if (NODE_REFERENCE_TYPES_BY_NODE_TYPE[node.metatype as unknown as NodeType] != null) {
    const properties = PROPERTY_ENUM_BY_TYPE[node.metatype as unknown as ObjectType]!;
    ref.metatype = NODE_REFERENCE_TYPES_BY_NODE_TYPE[node.metatype as unknown as NodeType] as unknown as ObjectType;
    for (const prop of Object.values(properties)) {
      if (typeof prop == "string" || prop < 40) continue; // ignore identity/tracking properties
      const propName = properties[prop];
      if ((node as any)[propName] != null) {
        (ref as any)[propName] = (node as any)[propName];
      }
    }
  }

  return ref;
}

export function getNodeType(node: AnyNodeData | SomeNodeReferenceData): NodeType {
  if (isNodeRef(node)) return (node as NodeReferenceData).type;
  else return node.metatype as unknown as NodeType;
}

export function toNodeRefOneOf(node: AnyNodeData | SomeNodeReferenceData): ViewData["nodePtr"] {
  if (!isNodeRef(node)) node = toNodeRef(node);
  if (!isNodeRef(node)) throw new Error(`expected Node, got ${JSON.stringify(node)}`);

  if (node.type == NodeType.FILE) {
    return { oneofKind: "nodePtrFile", nodePtrFile: node as FileReferenceData };
  } else if (node.type == NodeType.SECRET) {
    return { oneofKind: "nodePtrSecret", nodePtrSecret: node as SecretReferenceData };
  } else {
    return { oneofKind: "nodePtrNode", nodePtrNode: node as NodeReferenceData };
  }
}

/** Turn { [key]: value } into { key: value, oneofKind: key } for protobuf unions */
export function wrapProtoOneOf<T extends object>(value: T): T & { oneofKind: keyof T } {
  const key = Object.keys(value)[0] as keyof T;
  return { ...value, oneofKind: key };
}

type OneOfUnion = { oneofKind: string; [key: string]: any } | { oneofKind: undefined };
type UnwrappedOneOf<T extends OneOfUnion> = T extends { oneofKind: infer K }
  ? K extends keyof T
    ? Exclude<T[K], undefined>
    : never
  : never;
/** Turn union of { key: value, oneofKind: key } into { [key]: value } */
export function unwrapProtoOneOf<T extends OneOfUnion>(value: T | undefined): UnwrappedOneOf<T> | undefined {
  if (value == null) return undefined;
  const { oneofKind, ...rest } = value;
  if (oneofKind == null) return undefined;
  return (rest as any)[oneofKind] as UnwrappedOneOf<T>;
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

/** Checks whether the content properties of two builtin objects are equal */
export function contentEquals<T extends AnyNodeData | AnyStructData>(object: T, other: T): boolean {
  const allProperties = PROPERTY_ENUM_BY_TYPE[object.metatype as unknown as ObjectType];
  if (allProperties == null)
    throw new Error(`missing properties for ${NodeType[object.metatype]} (${object.metatype})`);
  for (const propId of Object.keys(allProperties)) {
    if (isNaN(Number(propId))) {
      continue; // skip non-numeric keys
    } else if ((propId as unknown as number) < 30) {
      continue; // skip non-content properties
    } else {
      const propName = allProperties[propId as unknown as number];
      if (!deepContentEquals((object as any)[propName], (other as any)[propName])) {
        return false;
      }
    }
  }
  return true;
}

/**
 * Deep checks whether two arbitrary objects are equal, only comparing content properties for our objects
 * (we just use the presence of the 'metatype' property to determine if it's one of our objects)
 */
export function deepContentEquals(object: any, other: any): boolean {
  if (object == null || other == null) {
    return object === other;
  } else if (typeof object != "object" || typeof other != "object") {
    return object === other;
  } else if ("metatype" in object && "metatype" in other) {
    return contentEquals(object, other);
  } else if (Array.isArray(object) && Array.isArray(other)) {
    return object.length == other.length && object.every((v, i) => deepContentEquals(v, other[i]));
  } else {
    for (const key of Object.keys(object)) {
      if (!deepContentEquals(object[key], other[key])) return false;
    }
    return true;
  }
}

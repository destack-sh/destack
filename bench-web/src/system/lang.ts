/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  Anchor,
  BenchType,
  BlockDataInfo,
  BlockProperty,
  BlockType,
  ColorType,
  ENUM_BY_TYPE,
  EditType,
  EnumType,
  FieldProperty,
  FieldZone,
  IconData,
  MESSAGE_TYPE_BY_OBJECT_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  NotificationData,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PackageData,
  PrimitiveType,
  RecordData,
  RunData,
  RunStatus,
  SignalData,
  StepType,
  StructType,
  Timestamp,
  TypeKind,
  ViewDataInfo,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  type AnyStructData,
  type BlockData,
  type EnumTypeMapping,
  type FieldData,
  type NodeTypeMapping,
  type PropertyInfo,
} from "@/proto/wire";
import {
  describeNode,
  fillDefaultObject,
  getDefaultProtoValue,
  isNode,
  newNodeCk,
  newNodeId,
  nodeReference,
  toNodeReference,
  type AnyNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { isDescendantOf, resolveNode, type ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE, getNodeIcon, makeIcon } from "@/system/icon";
import type { Transaction } from "@/system/transaction";
import { makeTypeInfo, type TypeIdentity } from "@/system/value";
import { getViewForValueType } from "@/system/view";
import { generateOrderKey, generateOrderKeys, isValidOrderKey } from "@/utils/fractional";
import { log } from "@/utils/log";
import { Casing, toCasing } from "@/utils/string";
import { getRandomColorType } from "@/utils/style";
import { uuidt } from "@/utils/uuidt";
import type { ViewProps } from "@/views/common";
import { computed, type Ref } from "vue";

export const NODE_TYPES = Object.values(NodeType).filter((v) => typeof v == "number" && v > 0) as NodeType[];
export const NODE_TYPES_SET = new Set(NODE_TYPES);
export const STRUCT_TYPES = Object.values(StructType).filter((v) => typeof v == "number" && v > 0) as StructType[];
export const STRUCT_TYPES_SET = new Set(STRUCT_TYPES);
export const OBJECT_TYPES = Object.values(ObjectType).filter((v) => typeof v == "number" && v > 0) as ObjectType[];
export const OBJECT_TYPES_SET = new Set(OBJECT_TYPES);
export const ENUM_TYPES = Object.values(EnumType).filter((v) => typeof v == "number" && v > 0) as EnumType[];
export const ENUM_TYPES_SET = new Set(ENUM_TYPES);

export function isNodeType(object: any): object is NodeType {
  return typeof object == "number" && NODE_TYPES_SET.has(object);
}

export function isStructType(object: any): object is StructType {
  return typeof object == "number" && STRUCT_TYPES_SET.has(object);
}

export function isObjectType(object: any): object is ObjectType {
  return typeof object == "number" && OBJECT_TYPES_SET.has(object);
}

export function isEnumType(object: any): object is EnumType {
  return typeof object == "number" && ENUM_TYPES_SET.has(object);
}

// :NodeTypes
export const ROOT_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH];
export const BASED_NODE_TYPES = [
  // :HasBase
  NodeType.FIELD,
  NodeType.RECORD,
  NodeType.MESSAGE,
  NodeType.RUN,
  NodeType.SIGNAL,
  NodeType.NOTIFICATION,
];
export const RUNTIME_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 1200 && nt < 1300);
export const TIMED_NODE_TYPES = [
  NodeType.SESSION,
  NodeType.RUN,
  NodeType.SIGNAL,
  NodeType.LOG,
  NodeType.NOTIFICATION,
  NodeType.MESSAGE,
];
export const ETERNAL_NODE_TYPES = [NodeType.SESSION, NodeType.RUN, NodeType.SIGNAL, NodeType.LOG];
export const RESOURCE_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 500 && nt < 600);
export const SOURCE_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 1000 && nt < 1100);
export const REMOTE_NODE_TYPES = SOURCE_NODE_TYPES.filter((nt) => nt >= 1100 && nt < 1200);

export const PAGE_BLOCK_TYPES = [BlockType.PAGE, BlockType.FLOW, BlockType.DATABASE, BlockType.VIEW];
export const TYPE_BLOCK_TYPES = [BlockType.CLASS, BlockType.CHOICE, BlockType.SIGNAL, BlockType.DATABASE];
export const RUNNABLE_BLOCK_TYPES = [BlockType.TEXT, BlockType.CODE, BlockType.FLOW];
export const CLASSY_BLOCK_TYPES = [
  BlockType.CLASS,
  BlockType.SIGNAL,
  ...RUNNABLE_BLOCK_TYPES,
  BlockType.VARIABLE,
  BlockType.DATABASE,
];

export const TERMINAL_RUN_STATUSES = [RunStatus.CANCELLED, RunStatus.ABORTED, RunStatus.FAILED, RunStatus.COMPLETED];
export const ACTIVE_RUN_STATUSES = [RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.SUSPENDED];
export const HALTED_RUN_STATUSES = [RunStatus.PAUSED, RunStatus.SUSPENDED];

export const VIEW_TYPES = Object.values(ViewType).filter((v) => typeof v == "number" && v > 0) as ViewType[];
export const ROOT_VIEW_TYPES = new Set([ViewType.WINDOW, ViewType.TAB, ViewType.SPLIT]);
export const NODE_VIEW_TYPES = new Set(VIEW_TYPES.filter((vt) => vt >= 200 && vt < 400));
export const HELPER_VIEW_TYPES = new Set(VIEW_TYPES.filter((vt) => vt >= 400 && vt < 600));

export const NAME_CONSTRAINT = BlockDataInfo[BlockProperty.name].constraint!;
export const TITLE_CONSTRAINT = ViewDataInfo[ViewProperty.title].constraint!;

/**
 * Gets the 'base' node defining a certain node. See :HasBase.
 */
export function getBaseFromNode(node: AnyNodeData): NodeReferenceData | null {
  if (node.metatype == ObjectType.RECORD) {
    return (node as RecordData).parentPtr ?? null;
  } else if (node.metatype == ObjectType.FIELD) {
    return (node as FieldData).parentPtr ?? null;
  } else if (node.metatype == ObjectType.RUN) {
    return (node as RunData).blockPtr ?? null;
  } else if (node.metatype == ObjectType.SIGNAL || node.metatype == ObjectType.NOTIFICATION) {
    return (node as SignalData | NotificationData).typePtr ?? null;
  } else {
    return null;
  }
}

export const TK_LENGTH_BYTES = 8;
export const TK_LENGTH_HEX = TK_LENGTH_BYTES * 2;
export const TK_LENGHT_IN_CK = TK_LENGTH_HEX + 2; // 2 for the dashes
export const TK_LENGTH_B64 = 12; // 8 * 1.5

export function getTkFromCk(ck: string) {
  return ck.slice(0, TK_LENGTH_HEX);
}

export function getTkFromPtr(ptr: NodeReferenceData) {
  if (ptr.ck) return ptr.ck.slice(0, TK_LENGTH_HEX);
  else if (ptr.id) return ptr.id.slice(0, TK_LENGTH_HEX);
  else throw new Error(`invalid ptr: ${ptr}`);
}

export function getTkFromPtrMaybe(ptr: NodeReferenceData | undefined | null) {
  if (ptr == null) return null;
  if (ptr.ck) return ptr.ck.slice(0, TK_LENGTH_HEX);
  else if (ptr.id) return ptr.id.slice(0, TK_LENGTH_HEX);
  else throw new Error(`invalid ptr: ${ptr}`);
}

function hexToBase64(hex: string) {
  const bytes = [];
  for (let i = 0; i < hex.length; i += 2) {
    bytes.push(parseInt(hex.substr(i, 2), 16));
  }
  const str = String.fromCharCode(...bytes);
  return btoa(str);
}

/** Gets the 8-byte template key in base64 from a node reference pointer */
export function getTkB64FromPtr(ptr: NodeReferenceData) {
  const ck = ptr.ck ?? ptr.id;
  if (!ck) throw new Error(`Invalid pointer: ${describeNode(ptr)}`);
  const hex = ck.replace(/-/g, "");
  return hexToBase64(hex.slice(0, TK_LENGTH_BYTES * 2));
}

/** Gets the 8-byte template key in base64 from a node ck */
export function getTkB64FromCk(ck: string) {
  const hex = ck.replace(/-/g, "");
  return hexToBase64(hex.slice(0, TK_LENGTH_BYTES * 2));
}

/** Gets the padded ck from its b64-encoded template key part */
export function padCkFromTkB64(tkB64: string) {
  const str = atob(tkB64);
  const bytes = new Uint8Array(str.length);
  for (let i = 0; i < str.length; i++) {
    bytes[i] = str.charCodeAt(i);
  }
  const padded = new Uint8Array(16);
  bytes.forEach((byte, index) => (padded[index] = byte));
  const hex = Array.from(padded, (byte) => byte.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

/** Sorts the given nodes using explicit order keys if available, createdAt otherwise, then id. */
export function defaultSortNode<T extends AnyNodeData>(nodes: T[]): void {
  nodes.sort((a, b) => {
    if ((a as any).orderKey != null && (b as any).orderKey != null && (a as any).orderKey != (b as any).orderKey) {
      return (a as any).orderKey > (b as any).orderKey ? 1 : -1;
    } else if (a.createdAt != null && b.createdAt != null) {
      if (a.createdAt.seconds != b.createdAt.seconds) {
        return Number(a.createdAt.seconds - b.createdAt.seconds);
      } else if (a.createdAt.nanos != b.createdAt.nanos) {
        return Number(a.createdAt.nanos - b.createdAt.nanos);
      }
    }
    return a.id > b.id ? 1 : -1;
  });
}

/** Sorts the given structs using explicit order keys if available, otherwise retains input order. */
export function defaultSortStruct<T extends AnyStructData>(structs: T[]): void {
  if (structs.length == 0) return;
  if ("orderKey" in structs[0]) {
    structs.sort((a, b) => {
      return (a as any).orderKey > (b as any).orderKey ? 1 : -1;
    });
  } else {
    // retain input order
  }
}

/**
 * Sets the node order keys so that the target is position relative to the reference. Nodes must be in order.
 * Also applies any 'fixes' due to duplicate order keys in the same transaction.
 */
export function updateOrder<T extends AnyNodeData & { orderKey: string }>(order: {
  tx: Transaction;
  node: T;
  position: "before" | "after";
  reference: string | T | null;
  getNodes: () => T[];
}) {
  let orderKey;
  const getReference = (nodes: T[]) => {
    if (order.reference == null) return order.position == "before" ? null : nodes[nodes.length - 1];
    else if (typeof order.reference == "string") return nodes.find((n) => n.id == order.reference) ?? null;
    else return order.reference;
  };
  try {
    const nodes = order.getNodes();
    orderKey = getOrderKey<T>({ nodes, position: order.position, reference: getReference(nodes) });
  } catch (e) {
    // 'fix' order keys if we couldn't generate one
    //  (usually because of duplicates, we don't enforce uniqueness per order key in backend for simplicity)
    let nodes = order.getNodes();
    fixOrderKeys(order.tx, nodes);
    nodes = order.getNodes(); // 'refresh' to apply tx changes
    orderKey = getOrderKey<T>({
      nodes,
      position: order.position,
      reference: getReference(nodes),
    });
  }
  // @ts-ignore: orderKey must exist
  order.tx.update(order.node, { orderKey }, { debounce: "tick" });
}

/**
 * Gets the order key relative to the reference. Nodes must be in order.
 */
export function getOrderKey<T extends { id: string; orderKey: string }>(order: {
  position: "before" | "above" | "after" | "below";
  reference: T | null;
  nodes: T[];
}) {
  let orderKey;
  if (order.position == "before" || order.position == "above") {
    const a =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) - 1];
    orderKey = generateOrderKey(a?.orderKey ?? null, order.reference?.orderKey ?? null);
  } else {
    const b =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) + 1];
    orderKey = generateOrderKey(order.reference?.orderKey ?? null, b?.orderKey ?? null);
  }
  return orderKey;
}

/**
 * Patches any 'broken' order keys to put the nodes in the given order.
 */
export function fixOrderKeys<T extends AnyNodeData & { orderKey: string }>(tx: Transaction, nodes: T[]) {
  // ensure nodes are in current order
  defaultSortNode(nodes);

  // scan for successive duplicates (they must be successive now)
  let i = 0;
  while (i < nodes.length) {
    const prevOrderKey = i == 0 ? null : nodes[i - 1].orderKey;
    const node = nodes[i];
    if (!isValidOrderKey(node.orderKey)) {
      // just patch in place
      const orderKey = generateOrderKey(prevOrderKey, nodes[i + 1]?.orderKey ?? null);
      // @ts-ignore: orderKey must exist
      tx.update(node, { orderKey }, { debounce: "tick" });
    } else if (node.orderKey == prevOrderKey) {
      // find all duplicates with same key from here and fix them in one go
      const numDuplicates = nodes.slice(i).filter((n) => n.orderKey == node.orderKey).length;
      const duplicates = nodes.slice(i, i + numDuplicates);
      const orderKeys = generateOrderKeys(prevOrderKey, nodes[i + numDuplicates]?.orderKey ?? null, numDuplicates);
      for (let j = 0; j < numDuplicates; j++) {
        // @ts-ignore: orderKey must exist
        tx.update(duplicates[j], { orderKey: orderKeys[j] }, { debounce: "tick" });
      }
      i += numDuplicates;
    } else {
      i++;
    }
  }
}

export function toCamelName<T extends object>(cls: T, key: any) {
  const name = cls[key as keyof T] as string;
  if (name == null) throw new Error(`invalid key into ${cls}: ${key}`);
  return toCasing(name, Casing.CAMEL, true);
}

/** Extracts the last (potentially multi-digit) characters as an integer */
export function extractNameId(name: string): number | null {
  const match = name.match(/\d+$/);
  return match ? parseInt(match[0]) : null;
}

const NODE_NAME_DISCRIMINATORS: Partial<Record<NodeType, string>> = {
  [NodeType.FIELD]: "zone",
  [NodeType.BLOCK]: "type",
  [NodeType.VIEW]: "type",
  [NodeType.STEP]: "type",
  [NodeType.FILE]: "coarseType",
};

function getNodeDiscriminator(node: { metatype: ObjectType } & Partial<AnyNodeData>): any | undefined {
  const key = NODE_NAME_DISCRIMINATORS[node.metatype as unknown as NodeType];
  if (key != null) return (node as any)[key];
  else return undefined;
}

/** Gets the discriminating subtype for a node, if any */
export function getNodeSubtype(node: AnyNodeData): FieldZone | BlockType | ViewType | StepType | any {
  const key = NODE_NAME_DISCRIMINATORS[node.metatype as unknown as NodeType];
  if (key != null) return (node as any)[key];
  else return null;
}

/** Gets the proper name for the discriminating subtype for a node, if any */
export function getNodeSubtypeName(metatype: NodeType, value?: number): string | null {
  const discriminator = NODE_NAME_DISCRIMINATORS[metatype];
  if (discriminator != null) {
    if (value == null) throw new Error(`value is required for discriminator ${discriminator}`);
    const properties = PROPERTY_ENUM_BY_TYPE[metatype as unknown as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype as unknown as ObjectType];
    const enumType = ENUM_BY_TYPE[propertyInfos[properties![discriminator as any]]?.enumType!];
    let typeName = enumType[value];
    if (typeof typeName != "string")
      throw new Error(`unknown type ${value} for ${NodeType[metatype]}.${discriminator}`);
    typeName = toCasing(typeName, Casing.CAMEL);
    return typeName;
  } else {
    return null;
  }
}

/** Generates a node name for our :AutoNaming. */
export function generateNodeName(metatype: NodeType, siblings: AnyNodeData[], value?: number): string {
  const discriminator = NODE_NAME_DISCRIMINATORS[metatype];
  if (discriminator != null && value != null) {
    if (value == null) throw new Error(`value is required for discriminator ${discriminator}`);
    const subtypeName = getNodeSubtypeName(metatype, value);
    const maxId = Math.max(
      ...siblings.filter((n) => (n as any)[discriminator!] == value).map((n) => extractNameId((n as any).name) ?? 0),
      0,
    );
    return `${subtypeName}${maxId + 1}`;
  }

  // default to no subtype
  const metatypeName = toCamelName(NodeType, metatype);
  const maxId = Math.max(...siblings.map((n) => extractNameId((n as any).name) ?? 0), 0);
  return `${metatypeName}${maxId + 1}`;
}

/** Checks whether the node name was likely generated */
export function isGeneratedNodeName(metatype: NodeType, name: string): boolean {
  // match name as <type><id> (groups)
  const match = name.match(/([a-zA-Z]+)(\d+)/);
  if (match == null) return false;
  const typeName = toCasing(match[1], Casing.ALL_CAPS);
  const key = NODE_NAME_DISCRIMINATORS[metatype];
  if (key != null) {
    const properties = PROPERTY_ENUM_BY_TYPE[metatype as unknown as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype as unknown as ObjectType];
    const enumType = ENUM_BY_TYPE[propertyInfos[properties![key as any]]?.enumType!];
    return enumType[typeName as any] != null || NodeType[typeName as any] != null;
  } else {
    return NodeType[typeName as any] != null;
  }
}

/** Generates the name for a node in the given graph */
export function makeNodeName(graph: ReadNodeGraph, node: { metatype: ObjectType } & Partial<AnyNodeData>): string {
  if (node.parentPtr == null) throw new Error("parentPtr is required");
  const siblings = graph.getChildren(node.parentPtr, node.metatype as unknown as NodeType);
  return generateNodeName(node.metatype as unknown as NodeType, siblings, getNodeDiscriminator(node));
}

/**
 * Auto-update any discriminator derived properties like generated name or additional flags.
 *  (e.g. from Choice1 to Variable2, or Input3 to Output2)
 **/
export function onNodeMorphed(tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData) {
  node = graph.getOrError({ id: node.id, ck: (node as any).ck }); // 'refresh' from graph with any optimistic changes

  // auto update node name
  if ("name" in node && node.name != null && isGeneratedNodeName(node.metatype as unknown as NodeType, node.name)) {
    const siblings = graph
      .getChildren(node.parentPtr!, node.metatype as unknown as NodeType)
      .filter((n) => n.id != node.id);
    const name = generateNodeName(node.metatype as unknown as NodeType, siblings, getNodeDiscriminator(node));
    if (name != node.name) tx.update(node, { name }, { debounce: "tick" });
  }

  // auto update block flags
  // ...
}

/**
 * Create a node from the given data and assign it an id (and ck if in package).
 * NOTE: id/ck are only assigned if not present. To copy, use copyNode.
 */
export function makeNode<T extends NodeType>(
  data: Partial<
    Omit<
      NodeTypeMapping[T],
      "metatype" | "id" | "ck" | "createdAt" | "updatedAt" | "revision" | "source" | "setProperties"
    >
  > & {
    metatype: T;
  },
  options?: { omit: (keyof NodeTypeMapping[T])[] },
): NodeTypeMapping[T] {
  const now = Timestamp.now();
  const node = {
    ...data,
    createdAt: now,
    updatedAt: now,
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
      if ("ck" in properties && (node as any).ck == null) {
        (node as any).ck = newNodeCk();
      }
      if (node.id == null) {
        if (TIMED_NODE_TYPES.includes(node.metatype as unknown as NodeType)) {
          node.id = uuidt();
        } else {
          node.id = newNodeId();
        }
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
  fillDefaultObject(node);

  return node;
}

/**
 * Creates a clone of this struct and its nested structs with the same content (and different identity)
 */
export function cloneStruct<T extends AnyStructData>(struct: T): T {
  const clone = { metatype: struct.metatype } as Record<string, any>;
  const allProperties = PROPERTY_ENUM_BY_TYPE[struct.metatype as unknown as ObjectType]!;
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[struct.metatype as unknown as StructType];
  for (const prop of Object.values(propertyInfos)) {
    if (prop.id < 30) continue; // ignore identity/tracking properties
    const propName = allProperties[prop.id];
    const propValue = (struct as any)[propName];
    if (prop.referenceStruct) {
      if (prop.isList) {
        clone[propName] = propValue.map((v: any) => cloneStruct(v));
      } else if (propValue) {
        clone[propName] = cloneStruct(propValue);
      }
    } else {
      if (prop.isList) {
        clone[propName] = propValue.slice();
      } else {
        clone[propName] = propValue;
      }
    }
  }
  fillDefaultObject(clone as T);
  return clone as T;
}

/** Clones a node directly with a new identity. */
function _cloneNode<T extends AnyNodeData>(node: T, now: Timestamp): T {
  const clone = cloneStruct(node);
  clone.id = newNodeId();
  if ("ck" in node) (clone as any).ck = newNodeCk();
  clone.parentPtr = node.parentPtr;
  if ("packagePtr" in node) (clone as any).packagePtr = node.packagePtr;
  if ("benchPtr" in node) (clone as any).benchPtr = node.benchPtr;
  if ("templatePtr" in node) (clone as any).templatePtr = node.templatePtr;
  if ("templatedEpoch" in node) (clone as any).templatedEpoch = node.templatedEpoch;
  if ("orderKey" in node) (clone as any).orderKey = node.orderKey;
  clone.revision = BigInt(0);
  clone.createdAt = now;
  clone.updatedAt = now;
  clone.deletedAt = undefined;
  clone.archivedAt = undefined;
  return clone;
}

/**
 * Creates a clone of this node and its node descendants with the same content (and different identity)
 * The new node will be appended after the current node in its parent.
 **/
export function cloneNode<T extends AnyNodeData>(
  tx: Transaction,
  graph: ReadNodeGraph,
  node: T,
  options: { includeChildren?: boolean; now?: Timestamp; set?: Partial<T>; _isNested?: boolean } = {
    includeChildren: true,
  },
): T {
  const now = options?.now ?? Timestamp.now();
  const clone = _cloneNode(node, now);
  if (options?.set) Object.assign(clone, options.set);
  if ("name" in clone && !options?._isNested) {
    if (isGeneratedNodeName(node.metatype as unknown as NodeType, (clone as any).name)) {
      // bump generated node name
      const siblings = graph.getChildren(node.parentPtr!, node.metatype as unknown as NodeType);
      clone.name = generateNodeName(node.metatype as unknown as NodeType, siblings, (clone as any).type);
    } else {
      // bump digit at end (or add 2) if already exists
      const seq = (clone as any).name.match(/\d+$/);
      if (seq != null) {
        const num = parseInt(seq[0]);
        clone.name = clone.name.replace(/\d+$/, (num + 1).toString());
      } else {
        clone.name += "2";
      }
    }
  }

  // actually create node
  tx.create(clone);
  // put clone after 'node'
  // (technically we could get the new orderKey before we create it, avoiding an update,
  //   but that only works if the order doesn't require fixing up the siblings, which updateOrder may do)
  if ((node as any).orderKey && !options?._isNested) {
    updateOrder({
      tx,
      node: clone as T & { orderKey: string },
      position: "after",
      reference: node.id!,
      getNodes: () =>
        graph.getChildren(node.parentPtr!, node.metatype as unknown as NodeType) as Array<T & { orderKey: string }>,
    });
  }

  // clone all children (recursively)
  if (options?.includeChildren) {
    const clonePtr = toNodeReference(clone);
    const children = graph.getChildren(node);
    for (const child of children) {
      cloneNode(tx, graph, child, { includeChildren: true, now, set: { parentPtr: clonePtr }, _isNested: true });
    }
  }

  return clone;
}

/**
 * Moves the given node around.
 * Except for 'up'/'down' requires a target as reference.
 * If the node has an 'orderKey' we try to respect the anchor.
 **/
export function moveNode(
  tx: Transaction,
  graph: ReadNodeGraph,
  node: AnyNodeData | AnyNodeReferenceData,
  options: {
    anchor: "start" | "center" | "end" | "before" | "after" | "up" | "down";
    target?: AnyNodeData | AnyNodeReferenceData;
  },
) {
  const { anchor } = options;
  node = resolveNode(graph, node);
  const target = options.target != null ? resolveNode(graph, options.target) : undefined;
  if (node?.id == target?.id) {
    return; // no-op
  } else if (target != null && isDescendantOf(graph, target, node)) {
    throw new Error(`move ${describeNode(node)} to ${anchor} ${describeNode(target)} would be circular`);
  }

  if (anchor == "up" || anchor == "down") {
    throw new Error(`not yet implemented`);
  } else if (anchor == "start" || anchor == "end" || anchor == "before" || anchor == "after") {
    // move before target (in its parent's children = target siblings)
    if (target == null) throw new Error(`target required to move node ${anchor} ${describeNode(node)}`);
    const targetParent = graph.getOrError(target.parentPtr!);
    if ("orderKey" in node && "orderKey" in target) {
      updateOrder({
        tx,
        node: node as AnyNodeData & { orderKey: string },
        position: anchor == "start" || anchor == "before" ? "before" : "after",
        reference: target as AnyNodeData & { orderKey: string },
        getNodes: () => graph.getChildren(targetParent, target!.metatype as unknown as NodeType) as any,
      });
    }
    tx.move(node, { parentPtr: target.parentPtr! }, { debounce: "tick" });
  } else if (anchor == "center") {
    // move to end of target's children of that type
    if (target == null) throw new Error(`target required to move node ${anchor} ${describeNode(node)}`);
    if ("orderKey" in node) {
      updateOrder({
        tx,
        node: node as AnyNodeData & { orderKey: string },
        position: "after",
        reference: null,
        getNodes: () => graph.getChildren(target!, node.metatype as unknown as NodeType) as any,
      });
    }
    tx.move(node, { parentPtr: toNodeReference(target) }, { debounce: "tick" });
  } else {
    throw new Error(`unexpected anchor: ${anchor}`);
  }
}

/** Create a Block relative to another. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    block: { type: BlockType; isPage?: boolean; isProtocol?: boolean };
    anchor: "before" | "after" | "inside";
    target: BlockData | TypedNodeReferenceData<NodeType.BLOCK> | PackageData | TypedNodeReferenceData<NodeType.PACKAGE>;
  },
): BlockData {
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);
  const packagePtr = isNode(target, NodeType.PACKAGE) ? toNodeReference(target) : target.packagePtr;

  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let siblings: BlockData[];
  if (options.anchor == "inside") {
    parentPtr = toNodeReference(target);
    siblings = graph.getChildren(target, NodeType.BLOCK);
    orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
  } else {
    if (isNode(target, NodeType.PACKAGE)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
    parentPtr = target.parentPtr!;
    siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
    orderKey = getOrderKey({ position: options.anchor, reference: target, nodes: siblings });
  }

  const block = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr,
    packagePtr,
    type: options.block.type,
    orderKey,
    name: makeNodeName(graph, { metatype: ObjectType.BLOCK, type: options.block.type, parentPtr }),
  });
  return block;
}

/** Create a Field relative to a Field or a Block. */
export function createField(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    field?: Partial<FieldData>;
    anchor: "before" | "above" | "after" | "below" | "inside" | "center";
    target: FieldData | TypedNodeReferenceData<NodeType.FIELD> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
  },
): FieldData {
  // eslint-disable-next-line prefer-const
  let { anchor, field: fieldIn } = options;
  const target = isNode(options.target) ? options.target : graph.getOrError(options.target);

  // get position within parent
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let zone: FieldZone;
  let kind: TypeKind | null = null;
  let siblings: FieldData[];
  if (isNode(target, NodeType.BLOCK)) {
    if (anchor != "inside" && anchor != "center") throw new Error(`unexpected anchor for block: ${anchor}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toNodeReference(target);
    orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    // figure out field kind based on block type
    if (target.type == BlockType.CHOICE) {
      zone = FieldZone.OPTION;
      kind = TypeKind.LITERAL;
    } else if (TYPE_BLOCK_TYPES.includes(target.type)) {
      zone = FieldZone.MEMBER;
    } else if (RUNNABLE_BLOCK_TYPES.includes(target.type)) {
      zone = FieldZone.INPUT;
    } else {
      zone = FieldZone.VARIABLE;
    }
  } else if (isNode(target, NodeType.FIELD)) {
    if (anchor == "inside" || anchor == "center") throw new Error(`unexpected anchor for field: ${anchor}`);
    siblings = graph.getChildren(target.parentPtr!, NodeType.FIELD);
    parentPtr = target.parentPtr!;
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
    zone = target.zone;
    // copy kind if none given
    kind = fieldIn?.kind ?? (target as FieldData).kind;
  } else {
    throw new Error(`unexpected target node type: ${describeNode(target)}`);
  }

  // default to Text if no type given
  if (zone != FieldZone.OPTION && fieldIn?.kind == null) {
    fieldIn = { ...fieldIn, kind: TypeKind.STRUCT, benchType: BenchType.TEXT };
  }
  // assign kind if forced
  if (kind != null) {
    fieldIn = { ...fieldIn, kind };
  }

  // reset icon if it's the default one (so we can easily change the type & icon will auto-change too)
  if (
    fieldIn?.icon != null &&
    fieldIn?.icon?.faName == getNodeIcon({ metatype: ObjectType.FIELD, ...fieldIn })?.faName
  ) {
    fieldIn = { ...fieldIn, icon: undefined };
  }

  // assign color if option
  if (zone == FieldZone.OPTION && !(fieldIn != null && "icon" in fieldIn)) {
    const occupiedColors = siblings.map((f) => f.icon?.color?.type ?? ColorType.GRAY);
    const colorType = getRandomColorType({ except: occupiedColors });
    fieldIn = { ...fieldIn, icon: makeIcon({ faName: "fas fa-circle-small", color: colorType }) };
  }

  const field = tx.create({
    name: makeNodeName(graph, { metatype: ObjectType.FIELD, parentPtr, zone: zone }),
    zone,
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

/** Whether the given node is runnable */
export function isRunnable(node: AnyNodeData, graph: ReadNodeGraph, fields?: FieldData[]): boolean {
  if (isNode(node, NodeType.STEP)) {
    return true;
  } else if (isNode(node, NodeType.BLOCK)) {
    if (node.type == BlockType.CODE || node.type == BlockType.FLOW) {
      return true;
    } else if (node.type == BlockType.TEXT) {
      fields = fields ?? graph.getChildren(node, NodeType.FIELD);
      return fields.some((f) => f.zone == FieldZone.INPUT) && fields.some((f) => f.zone == FieldZone.OUTPUT);
    } else {
      return false;
    }
  } else {
    return false;
  }
}

export function isRunnableRef(
  node: Ref<AnyNodeData | null | undefined>,
  graph: ReadNodeGraph,
  fields?: Ref<FieldData[]>,
): Ref<boolean> {
  fields = fields ?? graph.getChildrenRef(node, NodeType.FIELD);
  return computed(() => node.value != null && isRunnable(node.value!, graph, fields?.value));
}

//
// Enums
//

export const EDIT_TYPE_PRESENT_VERB: Record<EditType, string> = {
  [EditType.UNSPECIFIED]: "???",
  [EditType.CREATE]: "creates",
  [EditType.UPSERT]: "upserts",
  [EditType.UPDATE]: "updates",
  [EditType.MOVE]: "moves",
  [EditType.ARCHIVE]: "archives",
  [EditType.UNARCHIVE]: "unarchives",
  [EditType.DELETE]: "deletes",
  [EditType.RESTORE]: "restores",
  [EditType.ERASE]: "erases",
};
export const EDIT_TYPE_PAST_VERB: Record<EditType, string> = {
  [EditType.UNSPECIFIED]: "???",
  [EditType.CREATE]: "created",
  [EditType.UPSERT]: "upserted",
  [EditType.UPDATE]: "updated",
  [EditType.MOVE]: "moved",
  [EditType.ARCHIVE]: "archived",
  [EditType.UNARCHIVE]: "unarchived",
  [EditType.DELETE]: "deleted",
  [EditType.RESTORE]: "restored",
  [EditType.ERASE]: "erased",
};

// NOTE: we soft-limit the subset of available enum options in bench-web
//  (in code and backend the entire ranges are available)
export const EXPOSED_BLOCK_TYPES = [
  BlockType.PAGE,
  BlockType.CLASS,
  BlockType.CHOICE,
  BlockType.TEXT,
  BlockType.CODE,
  BlockType.VARIABLE,
  BlockType.IDENTITY,
];
export const EXPOSED_STRUCT_TYPES = [
  // core
  StructType.PATH,
  StructType.TYPE_INFO,
  StructType.SCHEDULE,
  StructType.TRIGGER_INFO,
  // files
  StructType.ICON,
  // code
  StructType.CODE,
  // expressions
  StructType.EXPRESSION,
  StructType.SELECTION,
  // views
  StructType.COLOR,
  StructType.FONT,
  StructType.OFFSET,
  StructType.BOX,
  // access
  StructType.POLICY,
  StructType.POLICY_RULE,
  // flow
  StructType.PIPE,
  // text
  StructType.TEXT,
  // run
  StructType.RUN_OPTIONS,
  StructType.RUN_ATTEMPT,
  StructType.RUN_ERROR,
  StructType.RUN_FRAME,
  StructType.RUN_TRACE,
  StructType.BREAKPOINT,
  StructType.MODEL_OPTIONS,
  StructType.LOG_INFO,
];
export const EXPOSED_PRIMITIVE_TYPES = [
  PrimitiveType.BOOLEAN,
  PrimitiveType.INT64,
  PrimitiveType.FLOAT64,
  PrimitiveType.STRING,
  PrimitiveType.JSON,
  PrimitiveType.BYTES,
  PrimitiveType.UUID,
  PrimitiveType.DATETIME,
];
export const EXPOSED_ANCHORS = [
  // the rest are exposed too but as additional flags (start/end)
  Anchor.LEFT,
  Anchor.TOP,
  Anchor.RIGHT,
  Anchor.BOTTOM,
];
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
  [EnumType.ANCHOR]: EXPOSED_ANCHORS,
};
export const ENUM_TITLE_BY_TYPE: Partial<Record<EnumType, Record<any, string>>> = {
  [EnumType.PRIMITIVE_TYPE]: {
    [PrimitiveType.INT64]: "Integer",
    [PrimitiveType.FLOAT64]: "Number",
  },
};

export type EnumOption<T extends EnumType = EnumType> = {
  id: string;
  icon?: IconData;
  title: string;
  value: EnumTypeMapping[T];
  isHidden?: boolean;
};

const ENUM_OPTIONS_BY_TYPE: Record<EnumType, EnumOption[]> = Object.fromEntries(
  ENUM_TYPES.map((enumType) => [enumType, makeEnumOptions(enumType)]),
) as Record<EnumType, EnumOption[]>;

function makeEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  const protoEnum = ENUM_BY_TYPE[enumType];
  const icons = ENUM_ICONS_BY_TYPE[enumType];
  const titles = ENUM_TITLE_BY_TYPE[enumType];
  const availableEnums =
    FILTERED_ENUMS[enumType] ?? Object.values(protoEnum).filter((v) => typeof v == "number" && v > 0);
  const options: EnumOption<T>[] = availableEnums.map((value) => {
    const icon = icons?.[value];
    const name = protoEnum[value] as string;
    if (name == null) throw new Error(`missing enum option ${value} in ${EnumType[enumType]}`);
    const title = titles?.[value] ?? toCasing(name, Casing.CAMEL, true);
    const option: EnumOption<T> = { id: value.toString(), icon, title, value: value as EnumTypeMapping[T] };
    return option;
  });
  return options;
}

export function getEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  return ENUM_OPTIONS_BY_TYPE[enumType] as EnumOption<T>[];
}

/** Gets a random value from an enum, ignoring the number keys (which are for protobuf). */
export function getRandomEnumOption<T extends EnumType>(enumType: T): EnumTypeMapping[T] {
  const options = getEnumOptions(enumType);
  return options[Math.floor(Math.random() * options.length)].value;
}

export function getPropertyTitle(property: PropertyInfo): string {
  let pythonName = property.name;
  if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
  if (pythonName.endsWith("_packed")) pythonName = pythonName.slice(0, -7);
  const title = toCasing(pythonName, Casing.CAMEL, true);
  return title;
}

//
// Inspection
//

type InspectionCategory = (
  | { from?: number; to?: number; excluding?: number[] }
  | {
      from: number;
      to: number;
      replace: (properties: PropertyInfo[]) => InspectedPropertyIn;
    }
  | number
)[];

type InspectedProperty = {
  title: string;
  protoName?: string;
  category: string;
  property: PropertyInfo;
  viewType?: ViewType;
  props?: ViewProps;
  isFullWidth?: boolean;
  read?: (node: AnyNodeData) => any;
  write?: (tx: Transaction, node: AnyNodeData, value: any) => void;
};
type InspectedPropertyIn = Pick<InspectedProperty, "title" | "viewType" | "props" | "isFullWidth" | "read" | "write">;
type InspectionLayout = {
  properties: InspectedProperty[];
  onWrite?: (tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) => void;
};

// NOTE: we (try to) only use metatype/type to avoid recomputing inspection layouts on every change (might have to revisit)
// NOTE :Architecture: the inspection layout generation is a bit clumsy
function getInspectionInfo(metatype: ObjectType, type: any): Record<string, InspectionCategory> | null {
  if (metatype == ObjectType.FIELD) {
    if (type == FieldZone.OPTION) {
      return { Common: [FieldProperty.zone, FieldProperty.text] };
    }
    const properties = {
      Common: [
        FieldProperty.zone,
        {
          from: 40,
          to: 43,
          replace: () => ({
            title: "Type",
            viewType: ViewType.PICKER,
            props: { valueType: makeTypeInfo({ isRequired: true, benchType: BenchType.TYPE_INFO }) },
            read: (node: FieldData) => node,
            write: (tx: Transaction, node: FieldData, value: TypeIdentity | null) => {
              tx.update(
                node,
                {
                  kind: value?.kind,
                  primitiveType: value?.primitiveType,
                  benchType: value?.benchType,
                  baseTypePtr: value?.baseTypePtr,
                  constraint: value?.constraint,
                },
                { debounce: "tick" },
              );
            },
          }),
        },
        { from: 30, to: 43, excluding: [FieldProperty.valuePacked] },
      ],
      Constraint: [{ from: 60 }],
    };
    return properties;
  } else if (metatype == ObjectType.BLOCK) {
    const properties: Record<string, InspectionCategory> = {
      Common: [BlockProperty.type],
      Run: [],
    };
    if (RUNNABLE_BLOCK_TYPES.includes(type) || PAGE_BLOCK_TYPES.includes(type)) {
      properties.Run.push(BlockProperty.identityPtr);
      properties.Run.push(BlockProperty.isPaused);
    }
    if (type == BlockType.VARIABLE) {
      // value type
      properties.Common.push({
        from: BlockProperty.valueType,
        to: BlockProperty.valueType + 1,
        replace: () => ({
          title: "Value Type",
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
          read: (node: AnyNodeData) => (node as BlockData).valueType,
          write: (tx: Transaction, node: AnyNodeData, value: TypeIdentity | null) => {
            const valueType =
              value == null
                ? undefined
                : makeTypeInfo({
                    kind: value.kind,
                    primitiveType: value.primitiveType,
                    benchType: value.benchType,
                    baseTypePtr: value.baseTypePtr,
                    constraint: value.constraint,
                  });
            tx.update(node as BlockData, { valueType }, { debounce: "tick" });
          },
        }),
      });
    }
    return properties;
  } else if (metatype == ObjectType.VIEW) {
    const properties = {
      Common: [{ to: 40 }, ViewProperty.isInput],
      Content: [{ from: 40, to: 50 }],
      Style: [{ from: 50, to: 60 }],
      Layout: [{ from: 60, to: 70 }],
      Behavior: [{ from: 70, to: 80 }],
    };
    return properties;
  } else {
    return null;
  }
}

export const FULL_WIDTH_VIEW_TYPES = [ViewType.TEXT, ViewType.CODE];
const ALWAYS_EXCLUDED_PROPERTIES: string[] = ["order_key"];

/** Generates the inspection layout for an object metatype. */
export function getInspectionLayout(
  metatype: ObjectType,
  type: any,
  options?: { exclude?: string[] },
): InspectionLayout {
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype];
  const seenProperties: Record<number, PropertyInfo> = {};
  const inspectedProperties: InspectedProperty[] = [];
  const excluded = ALWAYS_EXCLUDED_PROPERTIES.concat(options?.exclude ?? []);

  const allProperties = PROPERTY_ENUM_BY_TYPE[metatype] ?? [];
  const categories: Record<string, InspectionCategory> = getInspectionInfo(metatype, type) ?? {
    Common: [{ from: undefined, to: undefined }],
  };
  for (const category of Object.keys(categories)) {
    const categoryProperties = categories[category as keyof typeof categories];
    // assemble all properties in category
    for (const range of categoryProperties) {
      let propertiesInRange;
      if (typeof range == "object") {
        propertiesInRange = Object.values(propertyInfos).filter((property) => {
          if ((range.from != null && property.id < range.from) || (range.to != null && property.id >= range.to)) {
            return false;
          } else if ("excluding" in range && range.excluding != null && range.excluding.includes(property.id)) {
            return false;
          } else {
            return true;
          }
        });
      } else {
        propertiesInRange = Object.values(propertyInfos).filter((property) => property.id == range);
      }

      // filter & map
      if (typeof range == "object" && "replace" in range) {
        for (const property of propertiesInRange) {
          seenProperties[property.id] = property;
        }
        const replaced = range.replace(propertiesInRange);
        const inspectedProperty: InspectedProperty = { ...replaced, property: propertiesInRange[0], category };
        inspectedProperties.push(inspectedProperty);
        continue; // already handled
      }
      for (const property of propertiesInRange) {
        if (seenProperties[property.id]) continue;
        if (property.id < 30 || property.isAutoset || property.isComputed || property.isSystem) continue;
        if (excluded.includes(property.name)) continue;
        seenProperties[property.id] = property;

        // map properties to components
        const title = getPropertyTitle(property);
        const protoName = allProperties[property.id];
        const inspectedProperty: InspectedProperty = { title, protoName, category, property };
        const valueView = getViewForValueType({
          primitiveType: property.primitiveType,
          benchType: (property.enumType ?? property.referenceNodes?.[0] ?? property.referenceStruct) as unknown as
            | BenchType
            | undefined,
          isRequired: property.isRequired ?? false,
          isList: property.isList ?? false,
          isSecret: property.isEncrypted ?? false,
          constraint:
            property.constraint != null ? { metatype: ObjectType.TYPE_CONSTRAINT, ...property.constraint } : undefined,
        });
        if (valueView == null) {
          log.warn("lang.missingView", property); // will indicate no view for value in UI
          continue;
        }
        inspectedProperty.viewType = valueView.viewType;
        inspectedProperty.props = valueView.props;
        inspectedProperty.isFullWidth = FULL_WIDTH_VIEW_TYPES.includes(valueView.viewType);
        inspectedProperties.push(inspectedProperty);
      }
    }
  }

  const discriminator = NODE_NAME_DISCRIMINATORS[metatype as unknown as NodeType];
  function onWrite(tx: Transaction, graph: ReadNodeGraph, node: AnyNodeData, property: PropertyInfo) {
    // trigger morph
    if (discriminator == property.name) {
      onNodeMorphed(tx, graph, node);
    }
  }

  return { properties: inspectedProperties, onWrite };
}

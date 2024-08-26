/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  NODE_SUBSUBTYPE_BY_TYPE,
  NODE_SUBTYPE_BY_TYPE,
  RUNNABLE_BLOCK_TYPES,
  TIMED_NODE_TYPES,
  toCamelName,
  TYPE_BLOCK_TYPES,
} from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import { FLOW_GRID_STEP_X, FLOW_GRID_STEP_Y } from "@/language/flow";
import { isDescendantOf, resolveNode, type ReadNodeGraph } from "@/language/graph";
import { getOrderKey, updateOrder } from "@/language/order";
import { newChangeId, type Transaction } from "@/language/transaction";
import {
  BenchType,
  BlockType,
  ColorType,
  ENUM_BY_TYPE,
  EnumType,
  FieldZone,
  FileFormat,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PackageData,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  StepType,
  StructType,
  Timestamp,
  TypeKind,
  ViewType,
  type AnyNodeData,
  type AnyStructData,
  type BlockData,
  type FieldData,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  describeNode,
  fillDefaultObject,
  isNode,
  isNodeRef,
  newNodeCk,
  newNodeId,
  nodeReference,
  toPlainNodeRef,
  type AnyNodeReferenceData,
  type SomeNodeReferenceData,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { getNodeIcon, getTypeIcon, makeIcon } from "@/ui/icon";
import { getEnumTitle } from "@/ui/inspect";
import { getRandomColorType } from "@/ui/style";
import { addVector2 } from "@/ui/view";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { uuidt } from "@/utils/uuidt";
import { computed, type Ref } from "vue";

/** Extracts the last (potentially multi-digit) characters as an integer */
export function extractNameId(name: string): number | null {
  const match = name.match(/\d+$/);
  return match ? parseInt(match[0]) : null;
}
/** Gets the node type for a node or reference */
export function getNodeType(node: AnyNodeData | SomeNodeReferenceData): NodeType {
  if (isNodeRef(node)) return node.type;
  else return node.metatype as unknown as NodeType;
}

/** Gets the discriminating subtype for a node, if any :NodeSubtype */
export function getNodeSubtype(node: Partial<AnyNodeData>): FieldZone | BlockType | ViewType | StepType | any {
  const key = NODE_SUBTYPE_BY_TYPE[node.metatype as unknown as NodeType];
  if (key != null) return (node as any)[key];
  else return null;
}

/** Gets the discriminating subsubtype for a node, if any :NodeSubtype */
export function getNodeSubsubtype(node: AnyNodeData): TypeKind | FileFormat | any {
  const key = NODE_SUBSUBTYPE_BY_TYPE[node.metatype as unknown as NodeType];
  if (key != null) return (node as any)[key];
  else return null;
}

/** Gets the proper name for the discriminating subtype for a node, if any */
export function getNodeSubtypeName(metatype: NodeType, value?: number): string | null {
  const discriminator = NODE_SUBTYPE_BY_TYPE[metatype];
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
  const discriminator = NODE_SUBTYPE_BY_TYPE[metatype];
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
  const key = NODE_SUBTYPE_BY_TYPE[metatype];
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
  return generateNodeName(node.metatype as unknown as NodeType, siblings, getNodeSubtype(node));
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
    const name = generateNodeName(node.metatype as unknown as NodeType, siblings, getNodeSubtype(node));
    if (name != node.name) tx.update(node, { name }, { debounce: "tick" });
  }

  // auto update block flags
  // ...
}

/**
 * Make a node from the given data and assign it an id (and ck if in package).
 * NOTE: id/ck are only assigned if not present. To copy, use copyNode.
 */
export function makeNode<T extends NodeType>(
  data: Partial<
    Omit<NodeTypeMapping[T], "metatype" | "createdAt" | "updatedAt" | "revision" | "source" | "setProperties">
  > & {
    metatype: T;
  },
  options?: { omit: (keyof NodeTypeMapping[T])[] },
): NodeTypeMapping[T] {
  const now = Timestamp.now();
  let node = {
    ...data,
    createdAt: now,
    updatedAt: now,
    revision: 0,
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
  node = fillDefaultObject(node);

  return node;
}

/**
 * Creates a clone of this struct and its nested structs with the same content (and different identity)
 */
export function cloneStruct<T extends AnyStructData>(struct: T): T {
  if (struct?.metatype == null) throw new Error(`missing metatype for ${JSON.stringify(struct)}`);
  let clone = { metatype: struct.metatype } as Record<string, any>;
  const allProperties = PROPERTY_ENUM_BY_TYPE[struct.metatype as unknown as ObjectType]!;
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[struct.metatype as unknown as StructType];
  if (propertyInfos == null) throw new Error(`no property info for ${struct.metatype}`);
  for (const prop of Object.values(propertyInfos)) {
    if (prop.id < 30) continue; // ignore identity/tracking properties
    const propName = allProperties[prop.id];
    const propValue = (struct as any)[propName];
    if (prop.referenceStruct) {
      if (prop.isList) {
        clone[propName] = propValue.map((v: any) => cloneStruct(v));
      } else if (propValue) {
        if (prop.referenceIsRich) {
          clone[propName] = propValue; // one of struct, and no need to clone anyway
        } else {
          clone[propName] = cloneStruct(propValue);
        }
      }
    } else {
      if (prop.isList) {
        clone[propName] = propValue.slice();
      } else {
        clone[propName] = propValue;
      }
    }
  }
  if ("orderKey" in struct) (clone as any).orderKey = struct.orderKey;
  clone = fillDefaultObject(clone as T);
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
 * NOTE :Broken: cloneNode should (but doesn't) keep inner references consistent :CloneNodeReferences
 **/
export function cloneNode<T extends AnyNodeData>(
  tx: Transaction,
  graph: ReadNodeGraph,
  node: T,
  options: { includeChildren?: boolean; now?: Timestamp; set?: Partial<T>; _isNested?: boolean } = {
    includeChildren: true,
  },
): T {
  // ensure clone is bundled into a change
  if (tx.change?.key == null) tx = tx.with({ change: { key: newChangeId(), title: "Clone" } });

  // clone this node
  const now = options?.now ?? Timestamp.now();
  const clone = _cloneNode(node, now);
  if (options?.set) Object.assign(clone, options.set);

  // update derived properties
  if ("name" in clone && !options?._isNested) {
    // update name
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
  if (isNode(clone, NodeType.STEP)) {
    // update position
    clone.position = addVector2(clone.position, { x: FLOW_GRID_STEP_X, y: FLOW_GRID_STEP_Y });
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
    const clonePtr = toPlainNodeRef(clone);
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
    tx.move(node, { parentPtr: toPlainNodeRef(target) }, { debounce: "tick" });
  } else {
    throw new Error(`unexpected anchor: ${anchor}`);
  }
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

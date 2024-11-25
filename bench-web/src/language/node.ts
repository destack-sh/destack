/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import { TIMED_NODE_TYPES, toCamelName } from "@/language/const";
import { FLOW_GRID_STEP } from "@/language/flow";
import { isDescendantOf, resolveNode, type ReadNodeGraph } from "@/language/graph";
import { updateOrder } from "@/language/order";
import { newChangeId, type Transaction } from "@/language/transaction";
import { JsonValue, packBuiltinObjectProperty, unpackBuiltinObjectProperty } from "@/language/value";
import {
  ENUM_BY_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeSubtypeMapping,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_SUBTYPE,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_SUBTYPE,
  PROPERTY_INFOS_BY_TYPE,
  PropertyInfo,
  StructType,
  Timestamp,
  type AnyNodeData,
  type AnyStructData,
  type NodeTypeMapping,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  isNodeRef,
  makeDefaultObject,
  newNodeCk,
  newNodeId,
  nodeReference,
  toNodeRef,
} from "@/proto/wiring";
import { addVector2 } from "@/ui/view";
import { Casing, toCasing } from "@/utils/string";
import { uuidt } from "@/utils/uuidt";
import { computed, Ref } from "vue";

/** Extracts the last (potentially multi-digit) characters as an integer */
export function extractNameId(name: string): number | null {
  const match = name.match(/\d+$/);
  return match ? parseInt(match[0]) : null;
}
/** Gets the node type for a node or reference */
export function getNodeType(node: AnyNodeData | NodeReferenceData): NodeType {
  if (isNodeRef(node)) return node.nodeType;
  else return node.metatype as unknown as NodeType;
}

/** Generates a node name for our :AutoNaming. */
export function generateNodeName(node: Partial<AnyNodeData>, siblings: AnyNodeData[]): string {
  if ((node as any).type != null) {
    // node subtype
    const properties = PROPERTY_ENUM_BY_TYPE[node.metatype as unknown as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype as unknown as ObjectType];
    const enumType = ENUM_BY_TYPE[propertyInfos[properties!["type" as any]]?.enumType!];
    const subtypeName = toCasing(enumType[(node as any).type] as string, Casing.CAMEL);
    const typeName = toCamelName(NodeType, node.metatype);
    const maxId = Math.max(
      ...siblings.filter((n) => (n as any).type == (node as any).type).map((n) => extractNameId((n as any).name) ?? 0),
      1,
    );
    return `${subtypeName}${siblings.length == 0 ? "" : maxId + 1}`;
  } else {
    // node type
    const typeName = toCamelName(NodeType, node.metatype);
    const maxId = Math.max(...siblings.map((n) => extractNameId((n as any).name) ?? 0), 1);
    return `${typeName}${siblings.length == 0 ? "" : maxId + 1}`;
  }
}

/** Checks whether the node name was likely generated */
export function isGeneratedNodeName(metatype: NodeType | ObjectType, name: string): boolean {
  // match name as <type><id> (groups)
  const match = name.match(/([a-zA-Z]+)(\d+)?/);
  if (match == null) return false;
  const typeParts = toCasing(match[1], Casing.ALL_CAPS).split("_");
  const typeName = typeParts.at(0);
  if (NodeType[typeName as any] != null) return true;
  const subtypeName = typeParts.at(-1);
  if (subtypeName != null) {
    const properties = PROPERTY_ENUM_BY_TYPE[metatype as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype as unknown as ObjectType];
    if (properties == null || propertyInfos == null) return false;
    const enumType = ENUM_BY_TYPE[propertyInfos[properties["type" as any]]?.enumType!];
    if (enumType?.[subtypeName] != null) return true;
  }
  return false;
}

/** Generates the name for a node in the given graph */
export function makeNodeName(graph: ReadNodeGraph, node: { metatype: ObjectType } & Partial<AnyNodeData>): string {
  if (node.parentPtr == null) throw new Error("parentPtr is required");
  const siblings = graph.getChildren(node.parentPtr, node.metatype as unknown as NodeType);
  return generateNodeName(node, siblings);
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
    const name = generateNodeName(node, siblings);
    if (name != node.name) tx.update(node, { name }, { debounce: "tick" });
  }

  // auto update block flags
  // ...
}

/** A Node 'in' type for mapping subnode correctly given a metatype & optional type. */
export type NodeIn<T extends NodeType> = NodeTypeMapping[T] extends { type: infer ST }
  ? ST extends keyof NodeSubtypeMapping[T]
    ? {
        metatype: T | ObjectType;
        type: ST;
        subnode?: Partial<NodeSubtypeMapping[T][ST]>;
      } & Partial<Omit<NodeTypeMapping[T], "metatype" | "type">>
    : {
        metatype: T | ObjectType;
      } & Partial<Omit<NodeTypeMapping[T], "metatype">>
  : {
      metatype: T | ObjectType;
    } & Partial<Omit<NodeTypeMapping[T], "metatype">>;

/**
 * Make a node from the given data and assign it an id (and ck if in package).
 * NOTE: id/ck are only assigned if not present. To copy, use copyNode.
 */
export function makeNode<T extends NodeType>(
  nodeIn: NodeIn<T>,
  options?: { omit: (keyof NodeTypeMapping[T])[] },
): NodeTypeMapping[T] {
  const now = Timestamp.now();
  let node = { ...nodeIn, createdAt: now, updatedAt: now } as unknown as NodeTypeMapping[T];
  const properties = NODE_PROPERTY_ENUM_BY_TYPE[nodeIn.metatype as unknown as ObjectType]!;

  // assign id/ck/scope
  if (!options?.omit?.includes("id")) {
    if ("packagePtr" in properties) {
      if (!("packagePtr" in nodeIn) || nodeIn.packagePtr == null) {
        throw new Error(`missing packagePtr to make sub-package node ${NodeType[nodeIn.metatype]}`);
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
  if (
    "benchPtr" in properties &&
    nodeIn.metatype != NodeType.BENCH &&
    !Object.prototype.hasOwnProperty.call(node, "benchPtr")
  ) {
    const benchId = node.parentPtr?.benchId ?? (node as any).packagePtr?.benchId;
    if (benchId == null) throw new Error(`missing benchId to make in-bench node ${NodeType[nodeIn.metatype]}`);
    (node as any).benchPtr = nodeReference(NodeType.BENCH, benchId);
  }

  // pack subnode
  if ("type" in properties && "subnode" in nodeIn) {
    node.subnodePacked = packSubnode(nodeIn.metatype as T, nodeIn.type as _NodeSubtype<T>, nodeIn.subnode as any);
  }

  // assign default values to unset properties
  node = makeDefaultObject(node) as NodeTypeMapping[T];

  return node;
}

type _NodeSubtype<T extends NodeType> = NodeTypeMapping[T] extends { type: infer U }
  ? U extends keyof NodeSubtypeMapping[T]
    ? U
    : never
  : never;
type _NodeSubnodeProperty<
  T extends NodeType,
  ST extends keyof NodeSubtypeMapping[T],
  P extends keyof NodeSubtypeMapping[T][ST],
> = NodeSubtypeMapping[T][ST][P];
type _NodeSubnodeProperties<T extends NodeType, ST extends _NodeSubtype<T>> = ST extends keyof NodeSubtypeMapping[T]
  ? keyof NodeSubtypeMapping[T][ST]
  : never;

/** Packs the subnode properties of a Node */
export function packSubnode<T extends NodeType, ST extends _NodeSubtype<T>>(
  nodeType: T,
  type: ST,
  subnode: ST extends keyof NodeSubtypeMapping[T] ? Partial<NodeSubtypeMapping[T][ST]> : never,
): JsonValue {
  if (subnode == null || typeof subnode != "object" || Object.keys(subnode).length == 0) {
    return {}; // empty subnode
  }

  const propertiesEnum = PROPERTY_ENUM_BY_SUBTYPE[nodeType]?.[type];
  const properties = PROPERTY_INFOS_BY_SUBTYPE[nodeType]?.[type];
  if (propertiesEnum == null || properties == null) {
    throw new Error(`no properties for ${NodeType[nodeType]}.${type.toString()}`);
  }

  const subnodePacked: Record<string, any> = {};
  for (const prop of Object.values(properties)) {
    const propName = propertiesEnum[prop.id];
    const propValue = (subnode as any)[propName];
    const propValuePacked = packBuiltinObjectProperty(propValue, prop);
    if (propValuePacked != null) {
      subnodePacked[prop.id.toString()] = propValuePacked;
    }
  }

  return { [type.toString()]: subnodePacked };
}

/** Unpacks the subnode properties of a Node */
export function unpackSubnode<T extends NodeType, ST extends _NodeSubtype<T>>(
  nodeType: T,
  type: ST,
  subnodePacked: JsonValue | undefined,
): ST extends keyof NodeSubtypeMapping[T] ? NodeSubtypeMapping[T][ST] : never {
  const propertyEnum = PROPERTY_ENUM_BY_SUBTYPE[nodeType]?.[type];
  const properties = PROPERTY_INFOS_BY_SUBTYPE[nodeType]?.[type];
  if (propertyEnum == null || properties == null)
    throw new Error(`no properties for ${NodeType[nodeType]}.${type.toString()}`);

  subnodePacked = (subnodePacked as any)?.[type.toString()] as Record<string, any>;
  if (subnodePacked == null || typeof subnodePacked != "object") {
    // nothing here, empty subnode
    return {} as any;
  }

  const subnode: Record<string, any> = {};
  for (const prop of Object.values(properties)) {
    const propName = propertyEnum[prop.id];
    const propValuePacked = (subnodePacked as any)[prop.id.toString()];
    const propValue = propValuePacked != null ? unpackBuiltinObjectProperty(propValuePacked, prop) : undefined;
    if (propValue != null) {
      subnode[propName] = propValue;
    }
  }

  return subnode as any;
}

/** Unpacks a specific subnode property */
export function unpackSubnodeProperty<
  T extends NodeType,
  ST extends _NodeSubtype<T>,
  P extends _NodeSubnodeProperties<T, ST>,
>(
  nodeType: T,
  type: ST,
  subnodePacked: JsonValue | undefined,
  propertyName: P,
): ST extends keyof NodeSubtypeMapping[T] ? _NodeSubnodeProperty<T, ST, P> : never {
  const propertyEnum = PROPERTY_ENUM_BY_SUBTYPE[nodeType]?.[type];
  const properties = PROPERTY_INFOS_BY_SUBTYPE[nodeType]?.[type];
  if (propertyEnum == null || properties == null)
    throw new Error(`no properties for ${NodeType[nodeType]}.${type.toString()}`);
  const propertyId: number = propertyEnum[propertyName];
  const property: PropertyInfo = properties[propertyId];
  const propValuePacked = (subnodePacked as any)?.[type.toString()]?.[propertyId.toString()];
  if (propValuePacked == null) {
    return undefined as any;
  } else {
    return unpackBuiltinObjectProperty(propValuePacked, property) as any;
  }
}

/** Unpacks a subnode reactively */
export function useSubnode<T extends NodeType, ST extends _NodeSubtype<T>>(
  nodeType: T,
  type: ST,
  subnodePacked: Ref<NodeTypeMapping[T] | JsonValue | undefined>,
): Ref<ST extends keyof NodeSubtypeMapping[T] ? NodeSubtypeMapping[T][ST] : never> {
  const subnode = computed(() => {
    if (subnodePacked.value == null) return null;
    if (isNode(subnodePacked.value, nodeType)) {
      return unpackSubnode(nodeType, type, subnodePacked.value.subnodePacked);
    } else {
      return unpackSubnode(nodeType, type, subnodePacked.value as JsonValue);
    }
  });
  return subnode as Ref<any>;
}

/** Unpacks a specific subnode property reactively */
export function useSubnodeProperty<
  T extends NodeType,
  ST extends _NodeSubtype<T>,
  P extends _NodeSubnodeProperties<T, ST>,
>(
  nodeType: T,
  type: ST,
  subnodePacked: Ref<NodeTypeMapping[T] | JsonValue | undefined>,
  propertyName: P,
): Ref<ST extends keyof NodeSubtypeMapping[T] ? _NodeSubnodeProperty<T, ST, P> : never> {
  const property = computed(() => {
    if (subnodePacked.value == null) return null;
    if (isNode(subnodePacked.value, nodeType)) {
      return unpackSubnodeProperty(nodeType, type, subnodePacked.value.subnodePacked, propertyName);
    } else {
      return unpackSubnodeProperty(nodeType, type, subnodePacked.value as JsonValue, propertyName);
    }
  });
  return property as Ref<any>;
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
  if ("orderKey" in struct) (clone as any).orderKey = struct.orderKey;
  clone = makeDefaultObject(clone as T);
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
  clone.createdAt = now;
  clone.updatedAt = now;
  clone.deletedAt = undefined;
  return clone;
}

/**
 * Creates a clone of this node and its node descendants with the same content (and different identity)
 * The new node will be appended after the current node in its parent.
 * TODO :Broken: cloneNode should (but doesn't) keep inner references consistent :CloneNodeReferences
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
      clone.name = generateNodeName(node, siblings);
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
    clone.position = addVector2(clone.position, { x: 0, y: FLOW_GRID_STEP * 4 });
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
    const clonePtr = toNodeRef(clone);
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
  nodeOrRef: AnyNodeData | NodeReferenceData,
  options: {
    anchor: "start" | "center" | "end" | "before" | "after" | "up" | "down";
    target?: AnyNodeData | NodeReferenceData;
  },
) {
  const { anchor } = options;
  const node = resolveNode(graph, nodeOrRef);
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
    tx.move(node, { parentPtr: toNodeRef(target) }, { debounce: "tick" });
  } else {
    throw new Error(`unexpected anchor: ${anchor}`);
  }
}

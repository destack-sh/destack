/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import { supergraph } from "@/globals";
import { isInlineNode, TIMED_NODE_TYPES, toCamelName } from "@/language/core/const";
import { getNextSibling, isDescendantOf, resolveNode, type ReadNodeGraph } from "@/language/core/graph";
import { getOrderKey, updateOrder } from "@/language/core/order";
import { JsonValue, packBuiltinObjectProperty, unpackBuiltinObjectProperty } from "@/language/core/value";
import { DebounceLevel, newChangeId, type Transaction } from "@/language/runtime/transaction";
import { createBlock, unwrapBlockDefinition } from "@/language/source/block";
import {
  BlockData,
  ENUM_BY_TYPE,
  NODE_PROPERTY_ENUM_BY_TYPE,
  NodeReferenceData,
  NodeSubtypeMapping,
  NodeType,
  ObjectType,
  PackageData,
  PageData,
  PARENT_NODE_TYPES,
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
import { FLOW_GRID_STEP } from "@/ui/flow";
import { addVector2 } from "@/ui/view";
import { generateOrderKey, INTEGER_ZERO } from "@/utils/fractional";
import { assertNever, groupByList } from "@/utils/functools";
import { Casing, toCasing } from "@/utils/string";
import { uuidt } from "@/utils/uuidt";
import { computed, Ref } from "vue";

/** Extracts the last (potentially multi-digit) characters as an integer */
export function extractNameId(name: string): number | null {
  const match = name.match(/\d+$/);
  return match ? parseInt(match[0]) : null;
}

/** Generates a node name for our :AutoNaming. */
export function generateNodeName(node: Partial<AnyNodeData>, siblings: AnyNodeData[]): string {
  if ((node as any).type != null) {
    // node subtype
    siblings = siblings.filter((n) => (n as any).type == (node as any).type);
    const properties = PROPERTY_ENUM_BY_TYPE[node.metatype as unknown as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype as unknown as ObjectType];
    const enumType = ENUM_BY_TYPE[propertyInfos[properties!["type" as any]]?.enumType!];
    const subtypeName = toCasing(enumType[(node as any).type] as string, Casing.CAMEL, true);
    const maxId = Math.max(...siblings.map((n) => extractNameId((n as any).name) ?? 0), 1);
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
      if (nodeIn.metatype == NodeType.PACKAGE) {
        (node as PackageData).packagePtr = toNodeRef(node);
      } else if (!("packagePtr" in nodeIn) || nodeIn.packagePtr == null) {
        throw new Error(`missing packagePtr to make sub-package node ${NodeType[nodeIn.metatype]}`);
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

  // assign orderKey
  if ("orderKey" in properties && (node as any).orderKey == null) {
    (node as any).orderKey = INTEGER_ZERO;
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

/** Returns the root nodes of the given nodes (without parent in the given nodes). */
export function getRootNodes<T extends AnyNodeData>(nodes: T[]): T[] {
  const nodesByParentId: Record<string, AnyNodeData[]> = groupByList(nodes, (node) => node.parentPtr!.id!);
  const rootNodes: T[] = [];
  for (const node of nodes) {
    if (nodesByParentId[node.id!] == null) rootNodes.push(node);
  }
  return rootNodes;
}

/** Replaces references in the Node or Struct with their mapped values. */
function replaceNodeReferences(obj: AnyNodeData | AnyStructData | any, map: Record<string, NodeReferenceData>): any {
  if (isNodeRef(obj) && map[obj.id!]) {
    // replace node reference
    return map[obj.id!];
  }

  // walk object
  if (Array.isArray(obj)) {
    for (let i = 0; i < obj.length; i++) {
      obj[i] = replaceNodeReferences(obj[i], map);
    }
    return obj;
  } else if (typeof obj === "object") {
    for (const key in obj) {
      obj[key] = replaceNodeReferences(obj[key], map);
    }
    return obj;
  } else {
    return obj;
  }
}

/**
 * Creates a clone of this node and its node descendants with the same content (and different identity)
 * The new node will be appended after the current node in its parent.
 **/
export function cloneNode<T extends AnyNodeData>(
  tx: Transaction,
  graph: ReadNodeGraph,
  oldNode: T,
  options: {
    after?: AnyNodeData;
    before?: AnyNodeData;
    includeChildren?: boolean;
    now?: Timestamp;
    set?: Partial<T>;
    keepProperties?: boolean;
    _isNested?: boolean;
    _isDefinitionCounterpart?: boolean;
    _keepOrder?: boolean;
    _oldNodeByOldId?: Record<string, AnyNodeData>;
    _newNodeByOldId?: Record<string, AnyNodeData>;
  } = {
    includeChildren: true,
  },
): T {
  // ensure clone is bundled into a change
  if (tx.change?.key == null) tx = tx.with({ change: { key: newChangeId(), title: "Clone" } });

  // clone this node
  const now = options?.now ?? Timestamp.now();
  const newNode = _cloneNode(oldNode, now);
  if (options?.set) Object.assign(newNode, options.set);

  // update derived properties
  if (!options?.keepProperties) {
    // name
    if ("name" in newNode && !options?._isNested) {
      // update name
      if (isGeneratedNodeName(oldNode.metatype as unknown as NodeType, (newNode as any).name)) {
        // bump generated node name
        const siblings = graph.getChildren(oldNode.parentPtr!, oldNode.metatype as unknown as NodeType);
        newNode.name = generateNodeName(oldNode, siblings);
      } else {
        // bump digit at end (or add 2) if already exists
        const seq = (newNode as any).name.match(/\d+$/);
        if (seq != null) {
          const num = parseInt(seq[0]);
          newNode.name = newNode.name!.replace(/\d+$/, (num + 1).toString());
        } else {
          newNode.name += "2";
        }
      }
    }
    // order
    if ("orderKey" in newNode) {
      const after = options?.after ?? oldNode;
      const before = options?.before ?? getNextSibling(graph, oldNode);
      const orderKey = generateOrderKey(
        (after as any).orderKey,
        (after as any)?.orderKey == (before as any)?.orderKey ? null : (before as any)?.orderKey,
      );
      newNode.orderKey = orderKey;
    }
    // position
    if (isNode(newNode, NodeType.ACTION)) {
      newNode.position = addVector2(newNode.position, { x: 0, y: FLOW_GRID_STEP * 6 });
    }
  }

  // remember new identity
  const newNodeByOldId = options?._newNodeByOldId != null ? options._newNodeByOldId : {};
  const oldNodeByOldId = options?._oldNodeByOldId != null ? options._oldNodeByOldId : {};
  newNodeByOldId[oldNode.id!] = newNode;
  oldNodeByOldId[oldNode.id!] = oldNode;

  // clone blocks/definitions together
  if (!options?._isDefinitionCounterpart) {
    if (isNode(oldNode, NodeType.BLOCK)) {
      // for definition blocks, also clone the source node
      const source = unwrapBlockDefinition(oldNode);
      if (source?.definitionPtr?.id == oldNode.id) {
        cloneNode(tx, graph, source, {
          includeChildren: true,
          now,
          set: { parentPtr: newNode.parentPtr },
          _isNested: true,
          _isDefinitionCounterpart: true,
          _keepOrder: true,
          _newNodeByOldId: newNodeByOldId,
          _oldNodeByOldId: oldNodeByOldId,
        });
      }
    } else if (isInlineNode(oldNode) && oldNode.definitionPtr != null) {
      // for inline source nodes, also clone the block definition
      const block = supergraph.getOrError(oldNode.definitionPtr) as BlockData;
      cloneNode(tx, graph, block, {
        includeChildren: true,
        now,
        set: { nodePtr: toNodeRef(newNode) },
        _isNested: true,
        _isDefinitionCounterpart: true,
        _keepOrder: true,
        _newNodeByOldId: newNodeByOldId,
        _oldNodeByOldId: oldNodeByOldId,
      });
    }
  }

  // clone all children (recursively)
  if (options?.includeChildren) {
    const clonePtr = toNodeRef(newNode);
    const children = graph.getChildren(oldNode);
    for (const child of children) {
      cloneNode(tx, graph, child, {
        includeChildren: true,
        now,
        set: { parentPtr: clonePtr },
        _isNested: true,
        _keepOrder: true,
        _newNodeByOldId: newNodeByOldId,
        _oldNodeByOldId: oldNodeByOldId,
      });
    }
  }

  // perform clone in one go
  if (!options?._isNested) {
    _createClones(tx, graph, oldNodeByOldId, newNodeByOldId);
  }

  return newNode;
}

/** Updates the given map with the new node references. */
export function _createClones(
  tx: Transaction,
  graph: ReadNodeGraph,
  oldNodeByOldId: Record<string, AnyNodeData>,
  newNodeByOldId: Record<string, AnyNodeData>,
) {
  // map old to new refs
  const identityRefMap: Record<string, NodeReferenceData> = {};
  for (const nodeId in newNodeByOldId) {
    const newNode = newNodeByOldId[nodeId];
    identityRefMap[nodeId] = toNodeRef(newNode);
  }

  // actually create nodes
  for (const oldNode of Object.values(oldNodeByOldId)) {
    let newNode = newNodeByOldId[oldNode.id!];
    newNode = replaceNodeReferences(newNode, identityRefMap);
    tx.create(newNode);
  }
}

/** Clones the given nodes (preserving their order) and returns the new nodes. */
export function cloneNodes<T extends AnyNodeData>(tx: Transaction, graph: ReadNodeGraph, nodes: T[]) {
  if (tx.change?.key == null) tx = tx.with({ change: { key: newChangeId(), title: "Duplicate" } });

  // get the root nodes
  nodes = getRootNodes(nodes);
  const nodesByParentId: Record<string, AnyNodeData[]> = groupByList(nodes, (node) => node.parentPtr?.id!);

  // clone them (maintaing relative position)
  const clonedNodes: AnyNodeData[] = [];
  const clonedNodesByParentId: Record<string, AnyNodeData[]> = {};
  const oldNodeByOldId: Record<string, AnyNodeData> = {};
  const newNodeByOldId: Record<string, AnyNodeData> = {};
  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i];
    const parentId = node.parentPtr?.id!;
    // if we already have the parent cloned, go after its last child, otherwise go after the parent
    let after;
    if (clonedNodesByParentId[parentId] != null) {
      after = clonedNodesByParentId[parentId]?.at(-1);
    } else {
      after = nodesByParentId[parentId]?.at(-1);
    }
    const lastChild = nodesByParentId[parentId]?.at(-1);
    const before = lastChild != null ? getNextSibling(graph, lastChild) : null;
    const clone = cloneNode(tx, graph, node, {
      after,
      before: before ?? undefined,
      includeChildren: true,
      _isNested: true,
      _oldNodeByOldId: oldNodeByOldId,
      _newNodeByOldId: newNodeByOldId,
    });
    clonedNodes.push(clone);
    if (clonedNodesByParentId[parentId] == null) clonedNodesByParentId[parentId] = [];
    clonedNodesByParentId[parentId]?.push(clone);
  }

  // actually clone
  _createClones(tx, graph, oldNodeByOldId, newNodeByOldId);

  return clonedNodes;
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
    anchor: "start" | "center" | "end" | "before" | "after";
    target?: AnyNodeData | NodeReferenceData;
    debounce?: DebounceLevel;
  },
) {
  const { anchor } = options;
  const node = resolveNode(graph, nodeOrRef);
  let target = options.target != null ? resolveNode(graph, options.target) : undefined;
  if (node?.id == target?.id) {
    return; // no-op
  } else if (target != null && isDescendantOf(graph, target, node)) {
    throw new Error(`move ${describeNode(node)} to ${anchor} ${describeNode(target)} would be circular`);
  }
  if (tx.change?.key == null) {
    tx = tx.with({ change: { key: newChangeId(), title: "Move" } });
  }

  let parentPtr: NodeReferenceData | undefined;
  if (anchor == "start" || anchor == "end" || anchor == "before" || anchor == "after") {
    // move before target (in its parent's children = target siblings)
    if (target == null) throw new Error(`no target given to move node ${anchor} ${describeNode(node)}`);
    if (target.metatype != node.metatype)
      throw new Error(`target ${describeNode(target)} is not of same type as node ${describeNode(node)}`);
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
    parentPtr = target.parentPtr!;
  } else if (anchor == "center") {
    // move to end of target's children of that type
    if (target == null) throw new Error(`no target given to move node ${anchor} ${describeNode(node)}`);
    if ("orderKey" in node) {
      updateOrder({
        tx,
        node: node as AnyNodeData & { orderKey: string },
        position: "after",
        reference: null,
        getNodes: () => graph.getChildren(target!, node.metatype as unknown as NodeType) as any,
      });
    }
    parentPtr = toNodeRef(target);
  } else {
    assertNever(anchor);
  }
  tx.move(node, { parentPtr }, { debounce: options.debounce ?? "tick" });

  // move block and defined source node together
  if (isNode(node, NodeType.BLOCK)) {
    // for definition blocks, also move the source node
    const source = unwrapBlockDefinition(node);
    if (source?.definitionPtr?.id == node.id) {
      tx.move(source, { parentPtr }, { debounce: options.debounce ?? "tick" });
    }
  } else if (isInlineNode(node)) {
    if (node.definitionPtr != null) {
      // for inline source nodes, also move the block definition
      let block = supergraph.getOrError(node.definitionPtr) as BlockData;
      if (isInlineNode(target) && target.definitionPtr != null) {
        target = supergraph.getOrError(target.definitionPtr) as BlockData;
      }
      if (!PARENT_NODE_TYPES[NodeType.BLOCK].includes(parentPtr.nodeType)) {
        // block no longer needed
        block = { ...block, nodePtr: undefined }; // clear nodePtr to avoid deleting that too
        tx.delete(block);
        tx.update(node, { definitionPtr: undefined });
      } else {
        moveNode(tx, graph, block, { anchor, target });
      }
    } else if (PARENT_NODE_TYPES[NodeType.BLOCK].includes(parentPtr.nodeType)) {
      // create a new block to 'define' the inline source node
      const parent = supergraph.getOrError(parentPtr) as PageData | BlockData;
      const block = createBlock(tx, graph, {
        block: {
          metatype: NodeType.BLOCK,
          packagePtr: node.packagePtr,
          type: node.metatype as any,
          nodePtr: toNodeRef(node),
        },
        anchor: "inside",
        target: parent,
      });
      tx.update(node, { definitionPtr: toNodeRef(block) });
    }
  }
}

/**
 * Moves the given nodes around. Like moveNode but such that multiple nodes are order preserved relatively.
 */
export function moveNodes(
  tx: Transaction,
  graph: ReadNodeGraph,
  nodes: AnyNodeData[],
  options: {
    anchor: "start" | "center" | "end" | "before" | "after";
    target?: AnyNodeData | NodeReferenceData;
  },
) {
  if (nodes.length == 0) return;
  if (tx.change?.key == null) tx = tx.with({ change: { key: newChangeId(), title: "Move" } });
  // move first to target
  moveNode(tx, graph, nodes[0], { anchor: options.anchor, target: options.target });
  // move the rest relative to the first
  for (let i = 1; i < nodes.length; i++) {
    moveNode(tx, graph, nodes[i], { anchor: "after", target: graph.getOrError(nodes[i - 1]) });
  }
}

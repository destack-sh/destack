/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import type { AnyStructData, BlockData, EnumTypeMapping, PropertyInfo } from "@/proto/wire";
import {
  BlockProperty,
  BlockType,
  ENUM_BY_TYPE,
  EnumType,
  IconData,
  NodeReferenceData,
  NodeType,
  NotificationData,
  ObjectType,
  PROPERTY_INFOS_BY_TYPE,
  RecordData,
  RunData,
  SignalData,
  StructType,
  ViewProperty,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { isNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeNodeName, type ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE } from "@/system/icon";
import type { Transaction } from "@/system/transaction";
import { generateOrderKeys, generateOrderKey, isValidOrderKey, INTEGER_ZERO } from "@/utils/fractional";
import { Casing, toCasing } from "@/utils/string";
import { AVAILABLE_VIEW_TYPES, type ViewComponent } from "@/views";

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

export const ROOT_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH];
export const BASED_NODE_TYPES = [NodeType.RECORD, NodeType.RUN, NodeType.SIGNAL, NodeType.NOTIFICATION];
export const RUNTIME_NODE_TYPES = [
  NodeType.SESSION,
  NodeType.RUN,
  NodeType.PAUSE,
  NodeType.SIGNAL,
  NodeType.LOG,
  NodeType.NOTIFICATION,
];
export const LOCAL_NODE_TYPES = [NodeType.RECORD, ...RUNTIME_NODE_TYPES];
export const DEFAULT_LOADED_SOURCE_NODE_TYPES = [
  NodeType.PACKAGE,
  NodeType.DEPENDENCY,
  NodeType.UPGRADE,
  NodeType.SPACE,
  NodeType.LINK,
  NodeType.NOTICE,
  NodeType.BLOCK,
  NodeType.TRIGGER,
  NodeType.FIELD,
  NodeType.QUERY,
  NodeType.STEP,
  NodeType.VIEW,
];

export const ROOT_VIEW_TYPES = new Set<ViewType>([ViewType.WINDOW, ViewType.TAB, ViewType.SPLIT]);
export const NODE_VIEW_TYPES = new Set<ViewType>([
  ViewType.PAGE,
  ViewType.BLOCK,
  ViewType.SCREEN,
  ViewType.DATABASE,
  ViewType.FLOW,
  ViewType.FIELD,
  ViewType.STEP,
]);
// views that have a white background
export const FULL_VIEW_TYPES = new Set<ViewType>([
  ...NODE_VIEW_TYPES,
  ViewType.EXPLORE,
  ViewType.OUTLINE,
  ViewType.CREATE,
  ViewType.INSPECT,
]);

export const ENABLED_BLOCK_TYPES = [
  BlockType.PAGE,
  BlockType.TEXT,
  BlockType.CLASS,
  BlockType.CHOICE,
  BlockType.CODE,
  BlockType.VARIABLE,
];

// views that aren't about a specific node but should just keep the current root view node
export const RIDEALONG_VIEW_TYPES = new Set([ViewType.EXPLORE, ViewType.OUTLINE, ViewType.CREATE, ViewType.INSPECT]);

/**
 * Gets the 'base' node defining a certain node. See HasBase.
 */
export function getBaseFromNode(node: AnyNodeData): NodeReferenceData | null {
  if (node.metatype == ObjectType.RECORD) {
    return (node as RecordData).parentPtr ?? null;
  } else if (node.metatype == ObjectType.RUN) {
    return (node as RunData).blockPtr ?? null;
  } else if (node.metatype == ObjectType.SIGNAL || node.metatype == ObjectType.NOTIFICATION) {
    return (node as SignalData | NotificationData).senderPtr ?? null;
  } else {
    return null;
  }
}

/** Sorts the given nodes using explicit order keys if available, createdAt otherwise, then id. */
export function defaultSortNode<T extends AnyNodeData>(nodes: T[]): void {
  nodes.sort((a, b) => {
    if ((a as any).orderKey != null && (b as any).orderKey != null && (a as any).orderKey != (b as any).orderKey) {
      return (a as any).orderKey > (b as any).orderKey ? 1 : -1;
    } else if (a.createdAt != null && b.createdAt != null && a.createdAt.seconds != b.createdAt.seconds) {
      return Number(b.createdAt.seconds - a.createdAt.seconds);
    } else {
      return a.id > b.id ? 1 : -1;
    }
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
  order.tx.update({ ...order.node, orderKey }, ["orderKey"]);
}

/**
 * Gets the order key relative to the reference. Nodes must be in order.
 */
export function getOrderKey<T extends { id: string; orderKey: string }>(order: {
  position: "before" | "after";
  reference: T | null;
  nodes: T[];
}) {
  let orderKey;
  if (order.position == "before") {
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
      tx.update({ ...node, orderKey }, ["orderKey"]);
    } else if (node.orderKey == prevOrderKey) {
      // find all duplicates with same key from here and fix them in one go
      const numDuplicates = nodes.slice(i).filter((n) => n.orderKey == node.orderKey).length;
      const duplicates = nodes.slice(i, i + numDuplicates);
      const orderKeys = generateOrderKeys(prevOrderKey, nodes[i + numDuplicates]?.orderKey ?? null, numDuplicates);
      for (let j = 0; j < numDuplicates; j++) {
        // @ts-ignore: orderKey must exist
        tx.update({ ...duplicates[j], orderKey: orderKeys[j] }, ["orderKey"]);
      }
      i += numDuplicates;
    } else {
      i++;
    }
  }
}

export function toCamelName<T extends object>(cls: T, key: any) {
  return toCasing(cls[key as keyof T] as string, Casing.CAMEL);
}

export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.BLOCK_TYPE]: ENABLED_BLOCK_TYPES,
  [EnumType.VIEW_TYPE]: AVAILABLE_VIEW_TYPES,
};

export type EnumOption<T extends EnumType = EnumType> = {
  id: string;
  icon?: IconData;
  title: string;
  value: EnumTypeMapping[T];
  isHidden?: boolean;
};

export function getEnumOptions<T extends EnumType>(enumType: T): EnumOption<T>[] {
  const protoEnum = ENUM_BY_TYPE[enumType];
  const icons = ENUM_ICONS_BY_TYPE[enumType];
  const availableEnums =
    FILTERED_ENUMS[enumType] ?? Object.values(protoEnum).filter((v) => typeof v == "number" && v > 0);
  const options: EnumOption<T>[] = availableEnums.map((value) => {
    const icon = icons?.[value];
    const name = protoEnum[value] as string;
    const title = toCasing(name, Casing.CAMEL, true);
    const option: EnumOption<T> = { id: value.toString(), icon, title, value: value as EnumTypeMapping[T] };
    return option;
  });
  return options;
}

/** Gets a random value from an enum, ignoring the number keys (which are for protobuf). */
export function getRandomEnumOption<T extends EnumType>(enumType: T): EnumTypeMapping[T] {
  const options = getEnumOptions(enumType);
  return options[Math.floor(Math.random() * options.length)].value;
}

/** Create a Block relative to another. */
export function createBlock(
  tx: Transaction,
  graph: ReadNodeGraph,
  blockIn: { type: BlockType; isPage?: boolean; isProtocol?: boolean },
  anchor: "before" | "after",
  targetPtr: BlockData | TypedNodeReferenceData<NodeType.BLOCK>,
) {
  const target = isNode(targetPtr) ? targetPtr : graph.getOrError(targetPtr);
  const siblings = graph.getChildren(target.parentPtr!, NodeType.BLOCK);
  const block = tx.create({
    metatype: NodeType.BLOCK,
    parentPtr: target.parentPtr,
    packagePtr: target.packagePtr,
    type: blockIn.type,
    isPage: blockIn.isPage || blockIn.type == BlockType.PAGE,
    isProtocol: blockIn.isProtocol || blockIn.type == BlockType.PROTOCOL,
    orderKey: getOrderKey({ position: anchor, reference: target, nodes: siblings }),
    name: makeNodeName(graph, { metatype: ObjectType.BLOCK, type: blockIn.type, parentPtr: target.parentPtr }),
  });
  return block;
}

type InspectionCategory = {
  category: string;
  properties: ({ from?: number; to?: number; excluding?: number[] } | number)[];
};
const INSPECTION_INFO_BY_TYPE: Partial<Record<ObjectType, InspectionCategory[]>> = {
  [ObjectType.FIELD]: [
    { category: "common", properties: [{ to: 40 }, { from: 60 }] },
    { category: "constraint", properties: [{ from: 40, to: 60 }] },
  ],
  [ObjectType.BLOCK]: [
    { category: "common", properties: [{ to: 40, excluding: [BlockProperty.policies] }] },
    { category: "content", properties: [{ from: 40, to: 60 }] },
    { category: "flags", properties: [{ from: 60, to: 70 }] },
    { category: "policy", properties: [BlockProperty.policies] },
  ],
  [ObjectType.VIEW]: [
    { category: "common", properties: [{ to: 40 }, ViewProperty.isInput] },
    { category: "content", properties: [{ from: 40, to: 50 }] },
    { category: "style", properties: [{ from: 50, to: 60 }] },
    { category: "layout", properties: [{ from: 60, to: 70 }] },
    { category: "behavior", properties: [{ from: 70, to: 80 }] },
  ],
};

type InspectedProperty = {
  title: string;
  category: string;
  property: PropertyInfo;
  component?: ViewComponent;
  props?: Record<string, any>;
  isFullWidth?: boolean;
};
type InspectionLayout = {
  properties: InspectedProperty[];
};

export function getInspectionLayout(node: AnyNodeData): InspectionLayout {
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[node.metatype];
  const seenProperties: Record<number, PropertyInfo> = {};
  const inspectedProperties: InspectedProperty[] = [];

  const categories = INSPECTION_INFO_BY_TYPE[node.metatype] ?? [
    { category: "common", properties: [{ from: undefined, to: undefined }] },
  ];

  for (const category of categories) {
    // assemble all properties in category
    const categoryPropertyInfos: PropertyInfo[] = [];
    for (const range of category.properties) {
      let propertiesInRange;
      if (typeof range == "object") {
        propertiesInRange = Object.values(propertyInfos).filter((property) => {
          if ((range.from != null && property.id < range.from) || (range.to != null && property.id >= range.to))
            return false;
          if (range.excluding != null && range.excluding.includes(property.id)) return false;
          return true;
        });
      } else {
        propertiesInRange = Object.values(propertyInfos).filter((property) => property.id == range);
      }
      for (const property of propertiesInRange) {
        if (seenProperties[property.id]) continue;
        seenProperties[property.id] = property;
        categoryPropertyInfos.push(property);
      }
    }

    // map properties to components
    for (const property of categoryPropertyInfos) {
      if (property.id < 30 || property.isInternal || property.isAutoset || property.isComputed || property.isSystem)
        continue;
      let cleanName = property.name;
      if (cleanName.endsWith("_ptr")) cleanName = cleanName.slice(0, -4);
      const title = toCasing(cleanName, Casing.CAMEL, true);
      inspectedProperties.push({
        title,
        category: category.category,
        property,
      });
    }
  }
  return { properties: inspectedProperties };
}

/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  BenchType,
  type AnyStructData,
  type BlockData,
  type EnumTypeMapping,
  type FieldData,
  type PropertyInfo,
} from "@/proto/wire";
import {
  BenchProperty,
  BlockProperty,
  BlockType,
  ENUM_BY_TYPE,
  EnumType,
  FieldProperty,
  FileProperty,
  IconData,
  NodeReferenceData,
  NodeType,
  NotificationData,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  PrimitiveType,
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
import { getViewComponentForValueType, makeTypeInfo, type TypeIdentity } from "@/system/value";
import { generateOrderKey, generateOrderKeys, isValidOrderKey } from "@/utils/fractional";
import { Casing, toCasing } from "@/utils/string";
import type { ViewProps } from "@/views";

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

export const TYPE_BLOCK_TYPES = [BlockType.CLASS, BlockType.CHOICE, BlockType.SIGNAL, BlockType.DATABASE];

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
  return toCasing(cls[key as keyof T] as string, Casing.CAMEL, true);
}

// NOTE: we soft-limit the subset of available enum options in bench-web
//  (in code and backend the entire ranges are available)
export const EXPOSED_BLOCK_TYPES = [
  BlockType.MODULE,
  BlockType.PAGE,
  BlockType.TEXT,
  BlockType.CLASS,
  BlockType.CHOICE,
  BlockType.CODE,
  BlockType.VARIABLE,
];
export const EXPOSED_STRUCT_TYPES = [
  // core
  StructType.PATH,
  StructType.TYPE_INFO,
  StructType.CONTEXT,
  StructType.SCHEDULE,
  StructType.PROJECTION,
  // files
  StructType.FILE,
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
  StructType.REQUEST,
  // flow
  StructType.STEP_CONNECTION,
  // text
  StructType.TEXT,
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
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
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

/** Create a Field relative to another. */
export function createField(
  tx: Transaction,
  graph: ReadNodeGraph,
  anchor: "before" | "above" | "after" | "below",
  targetPtr: FieldData | TypedNodeReferenceData<NodeType.FIELD>,
) {
  const target = isNode(targetPtr) ? targetPtr : graph.getOrError(targetPtr);
  const siblings = graph.getChildren(target.parentPtr!, NodeType.FIELD);
  const field = tx.create({
    metatype: NodeType.FIELD,
    parentPtr: target.parentPtr,
    packagePtr: target.packagePtr,
    orderKey: getOrderKey({ position: anchor, reference: target, nodes: siblings }),
    name: makeNodeName(graph, { metatype: ObjectType.FIELD, parentPtr: target.parentPtr }),
    kind: target.kind,
  });
  return field;
}

//
// Inspection
//

type InspectionCategory = {
  category: string;
  properties: (
    | { from?: number; to?: number; excluding?: number[] }
    | {
        from: number;
        to: number;
        replace: (properties: PropertyInfo[]) => InspectedPropertyPartial;
      }
    | number
  )[];
};
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
type InspectedPropertyPartial = Pick<
  InspectedProperty,
  "title" | "viewType" | "props" | "isFullWidth" | "read" | "write"
>;
type InspectionLayout = {
  properties: InspectedProperty[];
};

function typeProperty(): InspectedPropertyPartial {
  return {
    title: "Type",
    viewType: ViewType.PICKER,
    props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
    read: (node) => node,
    write: (tx, node, value: TypeIdentity) =>
      tx.updateDebounced(node, {
        primitiveType: value.primitiveType,
        benchType: value.benchType,
        baseTypePtr: value.baseTypePtr,
      }),
  };
}

const INSPECTION_INFO_BY_TYPE: Partial<Record<ObjectType, InspectionCategory[]>> = {
  [ObjectType.FIELD]: [
    {
      category: "Common",
      properties: [
        { from: 40, to: 43, replace: typeProperty },
        { to: 43, excluding: [FieldProperty.kind] },
        { from: 60 },
        FieldProperty.visibility,
      ],
    },
    { category: "Constraint", properties: [FieldProperty.formatHint] },
  ],
  [ObjectType.BLOCK]: [
    {
      category: "Common",
      properties: [
        {
          to: 40,
          excluding: [BlockProperty.text, BlockProperty.basesPtr, BlockProperty.builtinBase, BlockProperty.policies],
        },
      ],
    },
    { category: "Flags", properties: [{ from: 60, to: 70, excluding: [BlockProperty.pausedAt] }] },
  ],
  [ObjectType.VIEW]: [
    { category: "Common", properties: [{ to: 40 }, ViewProperty.isInput] },
    { category: "Content", properties: [{ from: 40, to: 50 }] },
    { category: "Style", properties: [{ from: 50, to: 60 }] },
    { category: "Layout", properties: [{ from: 60, to: 70 }] },
    { category: "Behavior", properties: [{ from: 70, to: 80 }] },
  ],
};
const FULL_WIDTH_VIEW_TYPES = [ViewType.TEXT, ViewType.CODE];
const ALWAYS_EXCLUDED_PROPERTIES: string[] = ["order_key"];

export function getInspectionLayout(metatype: ObjectType, options?: { exclude?: string[] }): InspectionLayout {
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype];
  const seenProperties: Record<number, PropertyInfo> = {};
  const inspectedProperties: InspectedProperty[] = [];
  const excluded = ALWAYS_EXCLUDED_PROPERTIES.concat(options?.exclude ?? []);

  const allProperties = PROPERTY_ENUM_BY_TYPE[metatype] ?? [];
  const categories = INSPECTION_INFO_BY_TYPE[metatype] ?? [
    { category: "common", properties: [{ from: undefined, to: undefined }] },
  ];

  for (const category of categories) {
    // assemble all properties in category
    for (const range of category.properties) {
      let propertiesInRange;
      if (typeof range == "object") {
        propertiesInRange = Object.values(propertyInfos).filter((property) => {
          if ((range.from != null && property.id < range.from) || (range.to != null && property.id >= range.to))
            return false;
          if ("excluding" in range && range.excluding != null && range.excluding.includes(property.id)) return false;
          return true;
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
        const inspectedProperty: InspectedProperty = {
          ...replaced,
          property: propertiesInRange[0],
          category: category.category,
        };
        inspectedProperties.push(inspectedProperty);
        continue; // already handled
      }
      for (const property of propertiesInRange) {
        if (seenProperties[property.id]) continue;
        if (property.id < 30 || property.isAutoset || property.isComputed || property.isSystem) continue;
        if (excluded.includes(property.name)) continue;
        seenProperties[property.id] = property;

        // map properties to components
        let pythonName = property.name;
        if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
        if (pythonName.startsWith("is_")) pythonName = pythonName.slice(3);
        const title = toCasing(pythonName, Casing.CAMEL, true);
        const inspectedProperty: InspectedProperty = {
          title,
          protoName: allProperties[property.id],
          category: category.category,
          property,
        };
        try {
          const { viewType, props } = getViewComponentForValueType({
            primitiveType: property.primitiveType,
            benchType: (property.enumType ?? property.referenceNodes?.[0] ?? property.referenceStruct) as unknown as
              | BenchType
              | undefined,
          });
          inspectedProperty.viewType = viewType;
          inspectedProperty.props = { ...props, isInput: true };
          inspectedProperty.isFullWidth = FULL_WIDTH_VIEW_TYPES.includes(viewType);
        } catch {
          // will show missing component
        }
        inspectedProperties.push(inspectedProperty);
      }
    }
  }
  return { properties: inspectedProperties };
}

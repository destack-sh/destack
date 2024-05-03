/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  BenchType,
  BlockProperty,
  BlockType,
  ColorType,
  ENUM_BY_TYPE,
  EnumType,
  FieldZone,
  FieldProperty,
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
  StepType,
  StructType,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  type AnyStructData,
  type BlockData,
  type EnumTypeMapping,
  type FieldData,
  type PropertyInfo,
  TypeKind,
} from "@/proto/wire";
import { describeNode, isNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import { type ReadNodeGraph } from "@/system/graph";
import { ENUM_ICONS_BY_TYPE, getNodeIcon, makeIcon } from "@/system/icon";
import type { Transaction } from "@/system/transaction";
import { getViewForValueType, makeTypeInfo, type TypeIdentity } from "@/system/value";
import { generateOrderKey, generateOrderKeys, isValidOrderKey } from "@/utils/fractional";
import { Casing, toCasing } from "@/utils/string";
import { getRandomColorType } from "@/utils/style";
import type { ViewProps } from "@/views/common";

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
export const RUNNABLE_BLOCK_TYPES = [BlockType.TEXT, BlockType.CODE, BlockType.FLOW];

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
    return (node as SignalData | NotificationData).originPtr ?? null;
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

export function getTkB64FromPtr(ptr: NodeReferenceData) {
  const ck = ptr.ck ?? ptr.id;
  if (ck == null) throw new Error(`invalid ptr: ${ptr}`);
  const hex = ck.replace(/-/g, "");
  const bytes = Buffer.from(hex, "hex");
  return bytes.slice(0, TK_LENGTH_BYTES).toString("base64");
}

export function padCkFromTkB64(tkB64: string) {
  const bytes = Buffer.from(tkB64, "base64");
  const padded = Buffer.alloc(16);
  bytes.copy(padded);
  const hex = padded.toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
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

/** Extracts the last (potentially multi-digit) characters as an integer */
export function extractNameId(name: string): number | null {
  const match = name.match(/\d+$/);
  return match ? parseInt(match[0]) : null;
}

const NODE_NAME_DISCRIMINATORS: Partial<Record<NodeType, string>> = {
  [NodeType.FIELD]: "zone", // only used for option/input/output
  [NodeType.BLOCK]: "type",
  [NodeType.VIEW]: "type",
  [NodeType.STEP]: "type",
};

/** Generates a node name for our :AutoNaming. */
export function generateNodeName<T extends NodeType>(metatype: T, siblings: AnyNodeData[], value?: number): string {
  let key: string | undefined;
  if (metatype != NodeType.FIELD || (value != FieldZone.VARIABLE && value != FieldZone.MEMBER))
    key = NODE_NAME_DISCRIMINATORS[metatype];
  else key = undefined;
  if (key != null) {
    if (value == null) throw new Error(`value is required for discriminator ${key}`);
    const properties = PROPERTY_ENUM_BY_TYPE[metatype as unknown as ObjectType];
    const propertyInfos = PROPERTY_INFOS_BY_TYPE[metatype as unknown as ObjectType];
    const enumType = ENUM_BY_TYPE[propertyInfos[properties![key as any]]?.enumType!];
    let typeName = enumType[value];
    if (typeof typeName != "string") throw new Error(`unknown type ${value} for ${NodeType[metatype]}.${key}`);
    typeName = toCasing(typeName, Casing.CAMEL);
    const maxId = Math.max(
      ...siblings.filter((n) => (n as any)[key!] == value).map((n) => extractNameId((n as any).name) ?? 0),
      0,
    );
    return `${typeName}${maxId + 1}`;
  } else {
    const metatypeName = toCamelName(NodeType, metatype);
    const maxId = Math.max(...siblings.map((n) => extractNameId((n as any).name) ?? 0), 0);
    return `${metatypeName}${maxId + 1}`;
  }
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
    return enumType[typeName as any] != null;
  } else {
    return NodeType[typeName as any] != null;
  }
}

/** Generates the name for a node in the given graph */
export function makeNodeName(
  graph: ReadNodeGraph,
  node: { metatype: ObjectType; parentPtr?: NodeReferenceData; zone?: any; type?: any },
): string {
  if (node.parentPtr == null) throw new Error("parentPtr is required");
  const siblings = graph.getChildren(node.parentPtr, node.metatype as unknown as NodeType);
  return generateNodeName(node.metatype as unknown as NodeType, siblings, node.zone ?? node.type);
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
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
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

/** Create a Field relative to a Field or a Block. */
export function createField(
  tx: Transaction,
  graph: ReadNodeGraph,
  anchor: "before" | "above" | "after" | "below" | "inside" | "center",
  targetPtr: FieldData | TypedNodeReferenceData<NodeType.FIELD> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>,
  fieldIn?: Partial<FieldData>,
) {
  const target = isNode(targetPtr) ? targetPtr : graph.getOrError(targetPtr);

  // get position within parent
  let parentPtr: NodeReferenceData;
  let orderKey: string;
  let zone: FieldZone;
  let siblings: FieldData[];
  if (isNode(target, NodeType.BLOCK)) {
    if (anchor != "inside" && anchor != "center") throw new Error(`unexpected anchor for block: ${anchor}`);
    siblings = graph.getChildren(target, NodeType.FIELD);
    parentPtr = toNodeReference(target);
    orderKey = getOrderKey({ position: "after", reference: siblings[siblings.length - 1], nodes: siblings });
    // figure out field kind based on block type
    if (target.type == BlockType.CHOICE) zone = FieldZone.OPTION;
    else if (TYPE_BLOCK_TYPES.includes(target.type)) zone = FieldZone.MEMBER;
    else if (RUNNABLE_BLOCK_TYPES.includes(target.type)) zone = FieldZone.INPUT;
    else zone = FieldZone.VARIABLE;
  } else if (isNode(target, NodeType.FIELD)) {
    if (anchor == "inside" || anchor == "center") throw new Error(`unexpected anchor for field: ${anchor}`);
    siblings = graph.getChildren(target.parentPtr!, NodeType.FIELD);
    parentPtr = target.parentPtr!;
    orderKey = getOrderKey({ position: anchor, reference: target, nodes: siblings });
    zone = target.zone;
  } else {
    throw new Error(`unexpected target node type: ${describeNode(target)}`);
  }

  // default to Text if no type given
  if (zone != FieldZone.OPTION && fieldIn?.kind == null) {
    fieldIn = { ...fieldIn, kind: TypeKind.STRUCT, benchType: BenchType.TEXT };
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

//
// Inspection
//

/** Gets the discriminating subtype for a node, if any */
export function getNodeSubtype(node: AnyNodeData): FieldZone | BlockType | ViewType | StepType | any {
  if (isNode(node, NodeType.FIELD)) return node.zone;
  else return (node as any).type;
}

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
};

// NOTE: we (try to) only use metatype/type to avoid recomputing inspection layouts on every change (might have to revisit)
function getInspectionInfo(metatype: ObjectType, type: any): Record<string, InspectionCategory> | null {
  if (metatype == ObjectType.FIELD) {
    if (type == FieldZone.OPTION) {
      return { Common: [FieldProperty.zone, FieldProperty.text, FieldProperty.visibility] };
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
            props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
            read: (node: FieldData) => node,
            write: (tx: Transaction, node: FieldData, value: TypeIdentity) => {
              tx.updateDebounced(node, {
                kind: value.kind,
                primitiveType: value.primitiveType,
                benchType: value.benchType,
                baseTypePtr: value.baseTypePtr,
              });
            },
          }),
        },
        { from: 30, to: 43, excluding: [FieldProperty.valuePacked] },
        FieldProperty.visibility,
      ],
      Constraint: [FieldProperty.formatHint, { from: 60 }],
    };
    return properties;
  } else if (metatype == ObjectType.BLOCK) {
    const properties: Record<string, InspectionCategory> = {
      Common: [BlockProperty.type],
      Flags: [{ from: 60, to: 70, excluding: [BlockProperty.pausedAt] }],
    };
    if (type == BlockType.ALIAS || type == BlockType.VARIABLE) {
      // base type
      properties.Common.push({
        from: BlockProperty.builtinBase,
        to: BlockProperty.builtinBase + 1,
        replace: () => ({
          title: "Base",
          viewType: ViewType.PICKER,
          props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
          read: (node: AnyNodeData) => (node as BlockData).builtinBase,
          write: (tx: Transaction, node: AnyNodeData, value: TypeIdentity) => {
            tx.updateDebounced(node as BlockData, {
              builtinBase: makeTypeInfo({
                kind: value.kind,
                primitiveType: value.primitiveType,
                benchType: value.benchType,
                baseTypePtr: value.baseTypePtr,
              }),
            });
          },
        }),
      });
    }
    properties.Common.push(BlockProperty.visibility);
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

const FULL_WIDTH_VIEW_TYPES = [ViewType.TEXT, ViewType.CODE];
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
  const categories = getInspectionInfo(metatype, type) ?? [
    { category: "common", properties: [{ from: undefined, to: undefined }] },
  ];

  for (const category of Object.keys(categories)) {
    const categoryProperties = categories[category as keyof typeof categories] as InspectionCategory;
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
        let pythonName = property.name;
        if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
        if (pythonName.startsWith("is_")) pythonName = pythonName.slice(3);
        const title = toCasing(pythonName, Casing.CAMEL, true);
        const protoName = allProperties[property.id];
        const inspectedProperty: InspectedProperty = { title, protoName, category, property };
        try {
          const valueView = getViewForValueType({
            primitiveType: property.primitiveType,
            benchType: (property.enumType ?? property.referenceNodes?.[0] ?? property.referenceStruct) as unknown as
              | BenchType
              | undefined,
            isList: property.isList ?? false,
            isSecret: property.isEncrypted ?? false,
          });
          if (valueView == null) throw new Error(`no view for property ${property.id}`);
          const { viewType, props } = valueView;
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

import {
  ActionType,
  Anchor,
  BlockType,
  EditType,
  EmptyProperty,
  EnumType,
  InlineNodeData,
  ModelType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PropertyInfo,
  PropertyReferenceData,
  Region,
  ResourceNodeData,
  RunStatus,
  SourceNodeData,
  StructType,
  TaskStatus,
  TypeBaseNodeData,
  ViewDataInfo,
  ViewProperty,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, propertyInfo } from "@/proto/wiring";
import { Casing, toCasing } from "@/utils/string";

export const FLOAT_EPSILON = 1e-6;

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

export function isSourceNodeType(nodeType: NodeType): boolean {
  return nodeType >= 5000 && nodeType < 5500;
}

export function isInlineNodeType(nodeType: NodeType): boolean {
  return INLINE_NODE_TYPES.includes(nodeType);
}

export function isTypeBaseNodeType(nodeType: NodeType): boolean {
  return TYPE_BASE_NODE_TYPES.includes(nodeType);
}

export function isSourceNode(node: any): node is SourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isSourceNodeType(node.metatype as unknown as NodeType);
}

export function isInlineNode(node: any): node is InlineNodeData {
  if (node == null || typeof node != "object") return false;
  else return isInlineNodeType(node.metatype as unknown as NodeType);
}

export function isTypeBaseNode(node: any): node is TypeBaseNodeData {
  if (node == null || typeof node != "object") return false;
  else return isTypeBaseNodeType(node.metatype as unknown as NodeType);
}

export function isStateNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 5500 && nodeType < 6000;
}

export function isRuntimeNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 6000 && nodeType < 6500;
}

export function isResourceNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 2000 && nodeType < 3000;
}

export function isResourceNode(node: any): node is ResourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isResourceNodeType(node.metatype as unknown as NodeType);
}

export function isStaticResourceNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 2000 && nodeType < 2100;
}

export function isDynamicResourceNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 2100 && nodeType < 3000;
}

export function isLocalNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 8000;
}

export function isGlobalNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType < 2000;
}

export function isRegionalNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 2000 && nodeType < 8000;
}

export function isBenchNodeType(nodeType: any): boolean {
  return (
    typeof nodeType == "number" &&
    ((nodeType >= 2000 && nodeType < 10000) ||
      nodeType == NodeType.BENCH ||
      nodeType == NodeType.PACKAGE ||
      nodeType == NodeType.HANDLE ||
      nodeType == NodeType.MEMBERSHIP ||
      nodeType == NodeType.INVITE ||
      nodeType == NodeType.CLIENT)
  );
}

/** Whether the node type isn't loaded by default */
export function isUnloadedNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && !isSourceNodeType(nodeType) && !isStaticResourceNodeType(nodeType);
}

// node types :NodeTypes
export const ROOT_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH];
export const BASED_NODE_TYPES = [
  // :HasBase
  NodeType.RECORD,
  NodeType.MESSAGE,
  NodeType.RUN,
];
export const COSMOS_NODE_TYPES = NODE_TYPES.filter((nodeType) => nodeType < 100);
export const SOURCE_NODE_TYPES = NODE_TYPES.filter(isSourceNodeType);
export const STATE_NODE_TYPES = NODE_TYPES.filter(isStateNodeType);
export const RUNTIME_NODE_TYPES = NODE_TYPES.filter(isRuntimeNodeType);
export const TIMED_NODE_TYPES = [NodeType.MESSAGE, NodeType.SESSION, NodeType.RUN, NodeType.INTERRUPTION, NodeType.LOG];
export const RESOURCE_NODE_TYPES = NODE_TYPES.filter(isResourceNodeType);
export const GLOBAL_NODE_TYPES = NODE_TYPES.filter(isGlobalNodeType);
export const REGIONAL_NODE_TYPES = NODE_TYPES.filter(isRegionalNodeType);
export const LOCAL_NODE_TYPES = NODE_TYPES.filter(isLocalNodeType);
export const PUBLIC_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH];
export const USER_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.CLIENT, NodeType.HANDLE];
export const INLINE_NODE_TYPES = [
  ...RESOURCE_NODE_TYPES,
  NodeType.CHANNEL,
  NodeType.PAGE,
  NodeType.CLASS,
  NodeType.CHOICE,
  NodeType.DATABASE,
  NodeType.FLOW,
  NodeType.KIT,
  NodeType.VIEW,
  NodeType.TAG,
  NodeType.TASK,
  NodeType.PLAN,
  NodeType.ROLE,
  NodeType.IDENTITY,
  NodeType.TEAM,
];
INLINE_NODE_TYPES.sort();
export const UNLOADED_RESOURCE_NODE_TYPES = [NodeType.FILE, NodeType.STREAM];
export const LOADED_PACKAGE_NODE_TYPES = [
  ...SOURCE_NODE_TYPES,
  ...RESOURCE_NODE_TYPES.filter((t) => !UNLOADED_RESOURCE_NODE_TYPES.includes(t)),
  NodeType.CHANNEL,
  NodeType.CLAIM,
  NodeType.PLAN,
  NodeType.TASK,
  NodeType.ROLE,
  NodeType.IDENTITY,
  NodeType.TEAM,
];

export const RUNNABLE_NODE_TYPES = [NodeType.FLOW, NodeType.ACTION, NodeType.LINK];
export const TYPE_NODE_TYPES = [NodeType.CLASS, NodeType.CHOICE, NodeType.DATABASE];
export const TYPE_BASE_NODE_TYPES = [...RUNNABLE_NODE_TYPES, ...TYPE_NODE_TYPES];

// block types
export const BLOCK_TYPES = Object.values(BlockType).filter((v) => typeof v == "number" && v > 0) as BlockType[];
export const CANVAS_BLOCK_TYPES = [BlockType.PAGE, BlockType.KIT, BlockType.FLOW, BlockType.DATABASE];
export const TEXT_BLOCK_TYPES = BLOCK_TYPES.filter((bt) => bt >= BlockType.PARAGRAPH);
export const NODE_BLOCK_TYPES = BLOCK_TYPES.filter((bt) => bt < BlockType.PARAGRAPH);

// action
export const ACTION_TYPES = Object.values(ActionType).filter((v) => typeof v == "number" && v > 0) as ActionType[];
export const FLOW_ACTION_TYPES = ACTION_TYPES.filter((st) => st < 100);
export const SOURCE_ACTION_TYPES = [ActionType.START];
export const SINK_ACTION_TYPES = [ActionType.END];

// run
export const TERMINAL_RUN_STATUSES = [RunStatus.CANCELLED, RunStatus.ABORTED, RunStatus.FAILED, RunStatus.COMPLETED];
export const INTERRUPTED_RUN_STATUSES = [RunStatus.PAUSED, RunStatus.YIELDED, RunStatus.WAITING];
export const ACTIVE_RUN_STATUSES = [RunStatus.RUNNING, RunStatus.PAUSED, ...INTERRUPTED_RUN_STATUSES];
export const BAD_RUN_STATUSES = [RunStatus.CANCELLED, RunStatus.ABORTED, RunStatus.FAILED];

// task
export const ACTIVE_TASK_STATUSES = [TaskStatus.RUNNING, TaskStatus.WAITING, TaskStatus.REVIEWING];
export const TERMINAL_TASK_STATUSES = [TaskStatus.COMPLETED, TaskStatus.FAILED, TaskStatus.CANCELLED];

// view
export const VIEW_TYPES = Object.values(ViewType).filter((v) => typeof v == "number" && v > 0) as ViewType[];
export const ROOT_VIEW_TYPES = new Set([ViewType.WINDOW, ViewType.TAB, ViewType.HISTORY, ViewType.SPLIT]);
export const HELPER_VIEW_TYPES = new Set(VIEW_TYPES.filter((vt) => vt >= 20000 && vt < 30000));

export const NAME_CONSTRAINT = ViewDataInfo[ViewProperty.name].constraint!;

/** Default base type for based Nodes */
export const BASE_TYPE_BY_NODE_TYPE: Partial<Record<NodeType, NodeType>> = {
  [NodeType.RECORD]: NodeType.DATABASE,
  [NodeType.MESSAGE]: NodeType.CLASS,
  [NodeType.RUN]: NodeType.FLOW,
};

/**
 * Gets the 'base' node defining a certain node. See :HasBase.
 */
export function getBaseFromNode(node: Partial<AnyNodeData>): NodeReferenceData | null {
  if (isNode(node, NodeType.RECORD)) {
    return node.databasePtr ?? null;
  } else if (isNode(node, NodeType.MESSAGE)) {
    return node.clazzPtr ?? null;
  } else if (isNode(node, NodeType.RUN)) {
    return node.linkPtr ?? node.actionPtr ?? node.flowPtr ?? null;
  } else {
    return null;
  }
}

/**
 * Gets the base reference from a node reference.
 */
export function getBaseFromNodeReference(nodeRef: NodeReferenceData): NodeReferenceData | null {
  if (nodeRef.baseId != null) {
    const baseType = BASE_TYPE_BY_NODE_TYPE[nodeRef.nodeType];
    if (baseType == null) throw new Error(`no base type found for ${describeNode(nodeRef)}`);
    return {
      metatype: ObjectType.NODE_REFERENCE,
      nodeType: baseType,
      id: nodeRef.baseId,
      benchId: nodeRef.benchId,
    };
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

export function toCamelName<T extends object>(cls: T, key: any) {
  const name = cls[key as keyof T] as string;
  if (name == null) throw new Error(`invalid key into ${cls}: ${key}`);
  return toCasing(name, Casing.CAMEL, true);
}

export const NODE_SUBTYPE_PACKED_ID = EmptyProperty.subnodePacked;
export const NODE_SUBTYPE_PACKED_KEY = NODE_SUBTYPE_PACKED_ID.toString(); // it's the same property id for all nodes

//
// Enums
//

export const EDIT_TYPE_PRESENT_VERB: Partial<Record<EditType, string>> = {
  [EditType.CREATE]: "creates",
  [EditType.UPSERT]: "upserts",
  [EditType.UPDATE]: "updates",
  [EditType.MOVE]: "moves",
  [EditType.DELETE]: "deletes",
  [EditType.RESTORE]: "restores",
  [EditType.ERASE]: "erases",
};
export const EDIT_TYPE_PAST_VERB: Partial<Record<EditType, string>> = {
  [EditType.CREATE]: "created",
  [EditType.UPSERT]: "upserted",
  [EditType.UPDATE]: "updated",
  [EditType.MOVE]: "moved",
  [EditType.DELETE]: "deleted",
  [EditType.RESTORE]: "restored",
  [EditType.ERASE]: "erased",
};

export const EXPOSED_NODE_TYPES = NODE_TYPES.filter((t) => t != NodeType.SKIP);
export const EXPOSED_BLOCK_TYPES = [
  BlockType.DATABASE,
  BlockType.FLOW,
  BlockType.PAGE,
  BlockType.CHOICE,
  BlockType.TASK,
  BlockType.KIT,
  ...TEXT_BLOCK_TYPES,
];
export const EXPOSED_STRUCT_TYPES = [
  // core
  StructType.PATH,
  StructType.TYPE,
  StructType.SCHEDULE,
  // files
  StructType.ICON,
  // code
  StructType.CODE,
  // expressions
  StructType.EXPRESSION,
  StructType.SELECTION,
  StructType.VALUE,
  // views
  StructType.COLOR,
  StructType.FONT,
  StructType.OFFSET,
  StructType.RECTANGLE,
  // access
  StructType.POLICY,
  StructType.POLICY_RULE,
  // text
  StructType.TEXT,
  // run
  StructType.RUN_OPTIONS,
  StructType.RUN_FRAME,
  StructType.RUN_TRACE,
  StructType.BREAKPOINT,
];
export const EXPOSED_PRIMITIVE_TYPES = [
  PrimitiveType.BOOLEAN,
  PrimitiveType.INT64,
  PrimitiveType.FLOAT64,
  PrimitiveType.STRING,
  PrimitiveType.JSON,
  PrimitiveType.BYTES,
  PrimitiveType.UUID,
  PrimitiveType.DATE,
  PrimitiveType.DATETIME,
  PrimitiveType.TIME,
  PrimitiveType.DURATION,
];
export const EXPOSED_ANCHORS = [
  // the rest are exposed too but as additional flags (start/end)
  Anchor.LEFT,
  Anchor.TOP,
  Anchor.RIGHT,
  Anchor.BOTTOM,
];
export const EXPOSED_MODEL_TYPES = [
  // :ModelType
  ModelType.OPENAI_GPT4_0,
  ModelType.OPENAI_GPT4_O_MINI,
  ModelType.OPENAI_O1_MINI,
  ModelType.OPENAI_O1,
  ModelType.ANTHROPIC_CLAUDE_3_5_SONNET,
];
export const EXPOSED_REGIONS = [Region.FRANKFURT];
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.NODE_TYPE]: EXPOSED_NODE_TYPES,
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
  [EnumType.ANCHOR]: EXPOSED_ANCHORS,
  [EnumType.MODEL_TYPE]: EXPOSED_MODEL_TYPES,
  [EnumType.REGION]: EXPOSED_REGIONS,
};

export function getPropertyTitle(property: PropertyInfo | PropertyReferenceData): string {
  if (isStruct(property, StructType.PROPERTY_REFERENCE)) property = propertyInfo(property.objectType!, property.id);
  let pythonName = property.name;
  if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
  if (pythonName.endsWith("_packed")) pythonName = pythonName.slice(0, -7);
  const title = toCasing(pythonName, Casing.CAMEL, true);
  return title;
}

export function getPropertyName(property: PropertyInfo): string {
  const properties = PROPERTY_ENUM_BY_TYPE[property.component];
  if (!properties) throw new Error(`invalid property component: ${property.component}`);
  return properties[property.id];
}

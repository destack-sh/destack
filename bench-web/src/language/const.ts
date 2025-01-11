import {
  ActionType,
  Anchor,
  BlockDataInfo,
  BlockProperty,
  BlockType,
  DynamicResourceNodeData,
  EditType,
  EmptyProperty,
  EnumType,
  FieldData,
  MessageData,
  ModelFamily,
  ModelProvider,
  ModelType,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PropertyInfo,
  PropertyReferenceData,
  RecordData,
  Region,
  ResourceNodeData,
  RunData,
  RunStatus,
  RuntimeNodeData,
  SourceNodeData,
  StateNodeData,
  StaticResourceNodeData,
  StructType,
  TypeFormat,
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
  return nodeType >= 3000 && nodeType < 3100;
}

export function isSourceNode(node: any): node is SourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isSourceNodeType(node.metatype as unknown as NodeType);
}

export function isStateNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 3500 && nodeType < 3600;
}

export function isStateNode(node: any): node is StateNodeData {
  if (node == null || typeof node != "object") return false;
  else return isStateNodeType(node.metatype as unknown as NodeType);
}

export function isRuntimeNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 3600 && nodeType < 3700;
}

export function isRuntimeNode(node: any): node is RuntimeNodeData {
  if (node == null || typeof node != "object") return false;
  else return isRuntimeNodeType(node.metatype as unknown as NodeType);
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

export function isStaticResourceNode(node: any): node is StaticResourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isStaticResourceNodeType(node.metatype as unknown as NodeType);
}

export function isDynamicResourceNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 2100 && nodeType < 2200;
}

export function isDynamicResourceNode(node: any): node is DynamicResourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isDynamicResourceNodeType(node.metatype as unknown as NodeType);
}

export function isLocalNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && nodeType >= 3000;
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
  NodeType.FIELD,
  NodeType.RECORD,
  NodeType.MESSAGE,
  NodeType.RUN,
];
export const SOURCE_NODE_TYPES = NODE_TYPES.filter(isSourceNodeType);
export const STATE_NODE_TYPES = NODE_TYPES.filter(isStateNodeType);
export const RUNTIME_NODE_TYPES = NODE_TYPES.filter(isRuntimeNodeType);
export const TIMED_NODE_TYPES = [NodeType.SESSION, NodeType.RUN, NodeType.INTERRUPTION, NodeType.LOG, NodeType.MESSAGE];
export const RESOURCE_NODE_TYPES = NODE_TYPES.filter(isResourceNodeType);
export const STATIC_RESOURCE_NODE_TYPES = RESOURCE_NODE_TYPES.filter(isStaticResourceNodeType);
export const DYNAMIC_RESOURCE_NODE_TYPES = RESOURCE_NODE_TYPES.filter(isDynamicResourceNodeType);
export const LOCAL_NODE_TYPES = NODE_TYPES.filter(isLocalNodeType);

// block types
export const BLOCK_TYPES = Object.values(BlockType).filter((v) => typeof v == "number" && v > 0) as BlockType[];
export const CANVAS_BLOCK_TYPES = [BlockType.FLOW, BlockType.DATABASE, BlockType.VIEW];
export const TYPE_BLOCK_TYPES = [BlockType.CHOICE, BlockType.MESSAGE, BlockType.DATABASE];
export const RUNNABLE_BLOCK_TYPES = [BlockType.FLOW];
export const CLASSY_BLOCK_TYPES = [BlockType.MESSAGE, ...RUNNABLE_BLOCK_TYPES, BlockType.VARIABLE, BlockType.DATABASE];

// action
export const ACTION_TYPES = Object.values(ActionType).filter((v) => typeof v == "number" && v > 0) as ActionType[];
export const BOUNDARY_ACTION_TYPES = ACTION_TYPES.filter((st) => st < 50);
export const SOURCE_ACTION_TYPES = [ActionType.START];
export const SINK_ACTION_TYPES = [ActionType.COMPLETE, ActionType.FAIL];
export const INVISIBLE_ACTION_TYPES = [ActionType.TEXT];
export const RUN_ACTION_TYPES = ACTION_TYPES.filter((st) => st >= 50 && st < 100);
export const CONTROL_ACTION_TYPES = ACTION_TYPES.filter((st) => st >= 100 && st < 150);
export const CONTAINER_ACTION_TYPES = ACTION_TYPES.filter((st) => st >= 500 && st < 560);

// run
export const TERMINAL_RUN_STATUSES = [RunStatus.CANCELLED, RunStatus.ABORTED, RunStatus.FAILED, RunStatus.COMPLETED];
export const INTERRUPTED_RUN_STATUSES = [RunStatus.PAUSED, RunStatus.YIELDED, RunStatus.WAITING];
export const ACTIVE_RUN_STATUSES = [
  RunStatus.PREPARING,
  RunStatus.RUNNING,
  RunStatus.PAUSED,
  ...INTERRUPTED_RUN_STATUSES,
];

// view
export const VIEW_TYPES = Object.values(ViewType).filter((v) => typeof v == "number" && v > 0) as ViewType[];
export const ROOT_VIEW_TYPES = new Set([ViewType.WINDOW, ViewType.TAB, ViewType.HISTORY, ViewType.SPLIT]);
export const HELPER_VIEW_TYPES = new Set(VIEW_TYPES.filter((vt) => vt >= 30000 && vt < 40000));

export const NAME_CONSTRAINT = BlockDataInfo[BlockProperty.name].constraint!;
export const TITLE_CONSTRAINT = ViewDataInfo[ViewProperty.title].constraint!;

/** Default base type for based Nodes */
export const BASE_TYPE_BY_NODE_TYPE: Partial<Record<NodeType, NodeType>> = {
  [NodeType.FIELD]: NodeType.BLOCK,
  [NodeType.RECORD]: NodeType.BLOCK,
  [NodeType.RUN]: NodeType.BLOCK,
  [NodeType.MESSAGE]: NodeType.BLOCK,
};

/**
 * Gets the 'base' node defining a certain node. See :HasBase.
 */
export function getBaseFromNode(node: Partial<AnyNodeData>): NodeReferenceData | null {
  if (isNode(node, NodeType.RECORD)) {
    return node.blockPtr ?? null;
  } else if (isNode(node, NodeType.FIELD)) {
    return node.parentPtr ?? null;
  } else if (isNode(node, NodeType.RUN) || isNode(node, NodeType.INTERRUPTION)) {
    return node.pipePtr ?? node.actionPtr ?? node.blockPtr ?? null;
  } else if (isNode(node, NodeType.MESSAGE)) {
    return node.blockPtr ?? null;
  } else {
    return null;
  }
}

/**
 * Gets the base reference from a node reference.
 */
export function getBaseFromNodeReference(nodeRef: NodeReferenceData): NodeReferenceData | null {
  if (nodeRef.baseCk != null) {
    // NOTE :Broken :Architecture: technically there could be multiple different base types for the references
    //  (but right now we only use the node type to get the appropriate supergraph, and since all bases are source nodes,
    //   it doesn't matter which base type we use - for now)
    const baseType = BASE_TYPE_BY_NODE_TYPE[nodeRef.nodeType];
    if (baseType == null) throw new Error(`no base type found for ${describeNode(nodeRef)}`);
    return {
      metatype: ObjectType.NODE_REFERENCE,
      nodeType: baseType,
      ck: nodeRef.baseCk,
      benchId: nodeRef.baseBenchId ?? nodeRef.benchId,
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

// NOTE: we soft-limit the subset of available enum options in bench-web
//  (in code and backend the entire ranges are available)
export const EXPOSED_NODE_TYPES = NODE_TYPES.filter((t) => t != NodeType.SKIP);
export const EXPOSED_BLOCK_TYPES = [
  BlockType.TEXT,
  BlockType.DATABASE,
  BlockType.FLOW,
  BlockType.PAGE,
  BlockType.CHOICE,
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
  StructType.RUN_ATTEMPT,
  StructType.RUN_ERROR,
  StructType.RUN_FRAME,
  StructType.RUN_TRACE,
  StructType.BREAKPOINT,
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
export const EXPOSED_REGIONS = [Region.FRANKFURT, Region.OHIO];
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

export const ENUM_TITLE_BY_TYPE: Partial<Record<EnumType, Record<any, string>>> = {
  [EnumType.PRIMITIVE_TYPE]: {
    [PrimitiveType.INT64]: "Integer",
    [PrimitiveType.FLOAT64]: "Number",
    [PrimitiveType.UUID]: "UUID",
    [PrimitiveType.JSON]: "JSON",
    [PrimitiveType.DATETIME]: "Date and Time",
  },
  [EnumType.TYPE_FORMAT]: {
    [TypeFormat.URL]: "URL",
  },
  [EnumType.MODEL_PROVIDER]: {
    [ModelProvider.OPENAI]: "OpenAI",
  },
  [EnumType.MODEL_FAMILY]: {
    [ModelFamily.OPENAI_GPT]: "OpenAI GPT",
    [ModelFamily.OPENAI_O]: "OpenAI O",
    [ModelFamily.ANTHROPIC_CLAUDE]: "Anthropic Claude",
    [ModelFamily.GOOGLE_GEMINI]: "Google Gemini",
    [ModelFamily.META_LLAMA]: "Meta Llama",
  },
  [EnumType.MODEL_TYPE]: {
    // :ModelType
    [ModelType.OPENAI_GPT4_0]: "OpenAI GPT-4o",
    [ModelType.OPENAI_GPT4_O_MINI]: "OpenAI GPT-4o Mini",
    [ModelType.OPENAI_O1_MINI]: "OpenAI O1 Mini",
    [ModelType.OPENAI_O1]: "OpenAI O1",
    [ModelType.ANTHROPIC_CLAUDE_3_5_SONNET]: "Anthropic Claude 3.5 Sonnet",
    [ModelType.GOOGLE_GEMINI_1_5_PRO]: "Google Gemini 1.5",
    [ModelType.META_LLAMA_3_1_80B]: "Meta Llama 3.1",
    [ModelType.META_LLAMA_3_1_400B]: "Meta Llama 3.1 (400B)",
  },
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

import {
  Anchor,
  BlockDataInfo,
  BlockProperty,
  BlockType,
  EditType,
  ENUM_BY_TYPE,
  EnumType,
  EnumTypeMapping,
  FieldData,
  IconData,
  ModelProvider,
  ModelType,
  NodeReferenceData,
  NodeType,
  NotificationData,
  ObjectType,
  PipeFilter,
  PipeModulation,
  PrimitiveType,
  PropertyInfo,
  RecordData,
  RunData,
  RunStatus,
  SignalData,
  StepType,
  StructType,
  TypeFormat,
  ViewDataInfo,
  ViewProperty,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { describeNode } from "@/proto/wiring";
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

// node types :NodeTypes
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
export const SOURCE_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 1000 && nt < 1100);
export const STATE_NODE_TYPES = SOURCE_NODE_TYPES.filter((nt) => nt >= 1100 && nt < 1200);
export const RUNTIME_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 1200 && nt < 1300);
export const TIMED_NODE_TYPES = [
  NodeType.SESSION,
  NodeType.RUN,
  NodeType.SIGNAL,
  NodeType.LOG,
  NodeType.NOTIFICATION,
  NodeType.MESSAGE,
];
export const RESOURCE_NODE_TYPES = NODE_TYPES.filter((nt) => nt >= 500 && nt < 600);

// block types
export const BLOCK_TYPES = Object.values(BlockType).filter((v) => typeof v == "number" && v > 0) as BlockType[];
export const PAGE_BLOCK_TYPES = [BlockType.PAGE, BlockType.FLOW, BlockType.DATABASE, BlockType.VIEW];
export const TYPE_BLOCK_TYPES = [BlockType.CLASS, BlockType.CHOICE, BlockType.SIGNAL, BlockType.DATABASE];
export const RUNNABLE_BLOCK_TYPES = [BlockType.TEXT, BlockType.CODE, BlockType.FLOW];
export const CLASSY_BLOCK_TYPES = [
  BlockType.CLASS,
  BlockType.SIGNAL,
  ...RUNNABLE_BLOCK_TYPES,
  BlockType.VALUE,
  BlockType.DATABASE,
];

// step
export const STEP_TYPES = Object.values(StepType).filter((v) => typeof v == "number" && v > 0) as StepType[];
export const BOUNDARY_STEP_TYPES = STEP_TYPES.filter((st) => st < 50);
export const INCOMING_STEP_TYPES = [StepType.START, StepType.VALUE, StepType.TRIGGER];
export const OUTGOING_STEP_TYPES = [StepType.COMPLETE, StepType.FAIL];
export const RUN_STEP_TYPES = STEP_TYPES.filter((st) => st >= 50 && st < 100);
export const CONTROL_STEP_TYPES = STEP_TYPES.filter((st) => st >= 100 && st < 150);
export const CONTAINER_STEP_TYPES = STEP_TYPES.filter((st) => st >= 500 && st < 550);

// run
export const TERMINAL_RUN_STATUSES = [RunStatus.CANCELLED, RunStatus.ABORTED, RunStatus.FAILED, RunStatus.COMPLETED];
export const ACTIVE_RUN_STATUSES = [RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.SUSPENDED];
export const HALTED_RUN_STATUSES = [RunStatus.PAUSED, RunStatus.SUSPENDED];

// view
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
    return (node as RunData).stepPtr ?? (node as RunData).blockPtr ?? null;
  } else if (node.metatype == ObjectType.SIGNAL || node.metatype == ObjectType.NOTIFICATION) {
    return (node as SignalData | NotificationData).blockPtr ?? null;
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

// :NodeSubtype
export const NODE_SUBTYPE_BY_TYPE: Partial<Record<NodeType, string>> = {
  [NodeType.FIELD]: "zone",
  [NodeType.BLOCK]: "type",
  [NodeType.VIEW]: "type",
  [NodeType.STEP]: "type",
  [NodeType.FILE]: "coarseType",
};

//
// Enums
//

export const EDIT_TYPE_PRESENT_VERB: Record<EditType, string> = {
  [EditType.UNSPECIFIED]: "???",
  [EditType.CREATE]: "creates",
  [EditType.UPSERT]: "upserts",
  [EditType.UPDATE]: "updates",
  [EditType.MOVE]: "moves",
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
  [EditType.DELETE]: "deleted",
  [EditType.RESTORE]: "restored",
  [EditType.ERASE]: "erased",
};

// NOTE: we soft-limit the subset of available enum options in bench-web
//  (in code and backend the entire ranges are available)
export const EXPOSED_NODE_TYPES = NODE_TYPES.filter((t) => t != NodeType.SKIP);
export const EXPOSED_BLOCK_TYPES = [
  BlockType.PAGE,
  BlockType.CLASS,
  BlockType.CHOICE,
  BlockType.TEXT,
  BlockType.CODE,
  BlockType.FLOW,
  BlockType.VIEW,
  BlockType.VALUE,
  BlockType.DATABASE,
];
export const EXPOSED_STEP_TYPES = [StepType.START, StepType.COMPLETE, StepType.CODE, StepType.TEXT, StepType.BLOCK];
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
  StructType.VALUE,
  // views
  StructType.COLOR,
  StructType.FONT,
  StructType.OFFSET,
  StructType.BOX,
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
  PrimitiveType.DATETIME,
];
export const EXPOSED_ANCHORS = [
  // the rest are exposed too but as additional flags (start/end)
  Anchor.LEFT,
  Anchor.TOP,
  Anchor.RIGHT,
  Anchor.BOTTOM,
];
export const EXPOSED_PIPE_FILTERS = [PipeFilter.IS_TRUTHY, PipeFilter.IS_FALSY, PipeFilter.HAS_ERROR];
export const EXPOSED_PIPE_MODULATORS = [PipeModulation.FLATTEN, PipeModulation.ACCUMULATE];
export const EXPOSED_MODEL_TYPES = [
  // :ModelType
  ModelType.OPENAI_GPT4_0,
  ModelType.OPENAI_GPT4_O_MINI,
  ModelType.OPENAI_O1_MINI,
  ModelType.OPENAI_O1_PREVIEW,
  ModelType.ANTHROPIC_CLAUDE_3_5_SONNET,
];
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.NODE_TYPE]: EXPOSED_NODE_TYPES,
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STEP_TYPE]: EXPOSED_STEP_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
  [EnumType.ANCHOR]: EXPOSED_ANCHORS,
  [EnumType.PIPE_FILTER]: EXPOSED_PIPE_FILTERS,
  [EnumType.PIPE_MODULATION]: EXPOSED_PIPE_MODULATORS,
  [EnumType.MODEL_TYPE]: EXPOSED_MODEL_TYPES,
};

export const ENUM_TITLE_BY_TYPE: Partial<Record<EnumType, Record<any, string>>> = {
  [EnumType.PRIMITIVE_TYPE]: {
    [PrimitiveType.INT64]: "Integer",
    [PrimitiveType.FLOAT64]: "Number",
    [PrimitiveType.UUID]: "UUID",
    [PrimitiveType.JSON]: "JSON",
    [PrimitiveType.DATETIME]: "Date",
  },
  [EnumType.TYPE_FORMAT]: {
    [TypeFormat.URL]: "URL",
  },
  [EnumType.MODEL_PROVIDER]: {
    [ModelProvider.OPENAI]: "OpenAI",
  },
  [EnumType.MODEL_TYPE]: {
    // :ModelType
    [ModelType.OPENAI_GPT4_0]: "OpenAI GPT-4o",
    [ModelType.OPENAI_GPT4_O_MINI]: "OpenAI GPT-4o Mini",
    [ModelType.OPENAI_O1_MINI]: "OpenAI O1 Mini",
    [ModelType.OPENAI_O1_PREVIEW]: "OpenAI O1 Preview",
    [ModelType.ANTHROPIC_CLAUDE_3_5_SONNET]: "Anthropic Claude 3.5 Sonnet",
    [ModelType.GOOGLE_GEMINI_1_5_PRO]: "Google Gemini 1.5",
    [ModelType.META_LLAMA_3_1_80B]: "Meta Llama 3.1",
    [ModelType.META_LLAMA_3_1_400B]: "Meta Llama 3.1 (400B)",
  },
};

export function getPropertyTitle(property: PropertyInfo): string {
  let pythonName = property.name;
  if (pythonName.endsWith("_ptr")) pythonName = pythonName.slice(0, -4);
  if (pythonName.endsWith("_packed")) pythonName = pythonName.slice(0, -7);
  const title = toCasing(pythonName, Casing.CAMEL, true);
  return title;
}

import {
  BlockDataInfo,
  BlockProperty,
  BlockType,
  EnumType,
  FieldData,
  NodeReferenceData,
  NodeType,
  NotificationData,
  ObjectType,
  RecordData,
  RunData,
  RunStatus,
  SignalData,
  StructType,
  ViewDataInfo,
  ViewProperty,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { describeNode } from "@/proto/wiring";
import { Casing, toCasing } from "@/utils/string";

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

export function toCamelName<T extends object>(cls: T, key: any) {
  const name = cls[key as keyof T] as string;
  if (name == null) throw new Error(`invalid key into ${cls}: ${key}`);
  return toCasing(name, Casing.CAMEL, true);
}

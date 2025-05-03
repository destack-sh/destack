import {
  ActionType,
  Anchor,
  BlockType,
  EmptyProperty,
  EnumType,
  PAGE_NODE_TYPES,
  PageNodeData,
  INSTANTIABLE_NODE_TYPES,
  InstantiableNodeData,
  JOINABLE_NODE_TYPES,
  JoinableNodeData,
  NodeMode,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PrimitiveType,
  PROCESSABLE_NODE_TYPES,
  ProcessableNodeData,
  ProcessStatus,
  PROPERTY_ENUM_BY_TYPE,
  PropertyInfo,
  PropertyReferenceData,
  Region,
  RESOURCE_NODE_TYPES,
  ResourceNodeData,
  RUNNABLE_NODE_TYPES,
  RunnableNodeData,
  SOURCE_NODE_TYPES,
  SourceNodeData,
  StructType,
  SUBJECT_NODE_TYPES,
  SubjectNodeData,
  TEMPLATABLE_NODE_TYPES,
  TemplatableNodeData,
  ViewDataInfo,
  ViewProperty,
  ViewType,
  type AnyNodeData,
  PROVISIONABLE_NODE_TYPES,
  ProvisionableNodeData,
  Continent,
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

export function isPageNodeType(nodeType: NodeType): boolean {
  return PAGE_NODE_TYPES.includes(nodeType);
}

export function isSubjectNodeType(nodeType: NodeType): boolean {
  return SUBJECT_NODE_TYPES.includes(nodeType);
}

export function isJoinableNodeType(nodeType: NodeType): boolean {
  return JOINABLE_NODE_TYPES.includes(nodeType);
}

export function isProcessableNodeType(nodeType: NodeType): boolean {
  return PROCESSABLE_NODE_TYPES.includes(nodeType);
}

export function isTemplatableNodeType(nodeType: NodeType): boolean {
  return TEMPLATABLE_NODE_TYPES.includes(nodeType);
}

export function isInstantiableNodeType(nodeType: NodeType): boolean {
  return INSTANTIABLE_NODE_TYPES.includes(nodeType);
}

export function isRunnableNodeType(nodeType: NodeType): boolean {
  return RUNNABLE_NODE_TYPES.includes(nodeType);
}

export function isSourceNode(node: any): node is SourceNodeData {
  if (node == null || typeof node != "object") return false;
  else return isSourceNodeType(node.metatype as unknown as NodeType);
}

export function isSubjectNode(node: any): node is SubjectNodeData {
  if (node == null || typeof node != "object") return false;
  else return isSubjectNodeType(node.metatype as unknown as NodeType);
}

export function isJoinableNode(node: any): node is JoinableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isJoinableNodeType(node.metatype as unknown as NodeType);
}

export function isPageNode(node: any): node is PageNodeData {
  if (node == null || typeof node != "object") return false;
  else return isPageNodeType(node.metatype as unknown as NodeType);
}

export function isProcessableNode(node: any): node is ProcessableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isProcessableNodeType(node.metatype as unknown as NodeType);
}

export function isTemplatableNode(node: any): node is TemplatableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isTemplatableNodeType(node.metatype as unknown as NodeType);
}

export function isInstantiableNode(node: any): node is InstantiableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isInstantiableNodeType(node.metatype as unknown as NodeType);
}

export function isRunnableNode(node: any): node is RunnableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isRunnableNodeType(node.metatype as unknown as NodeType);
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

export function isProvisionableResourceNodeType(nodeType: any): boolean {
  return typeof nodeType == "number" && PROVISIONABLE_NODE_TYPES.includes(nodeType);
}

export function isProvisionableResourceNode(node: any): node is ProvisionableNodeData {
  if (node == null || typeof node != "object") return false;
  else return isProvisionableResourceNodeType(node.metatype as unknown as NodeType);
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

/** Whether the Node is 'active' (not a template/archived/...) */
export function isNodeActive(node: AnyNodeData): boolean {
  if ("mode" in node) {
    return node.mode < NodeMode.TEMPLATE && node.deletedAt == null && node.archivedAt == null;
  } else {
    return node.deletedAt == null && node.archivedAt == null;
  }
}

/** Whether the node is an instance of a template */
export function isNodeInstance(node: AnyNodeData): boolean {
  return "ck" in node && node.templatePtr != null && node.id != node.ck;
}

// node types
export const UNLOADED_RESOURCE_NODE_TYPES = [NodeType.FILE];
export const LOADED_PACKAGE_NODE_TYPES = [
  ...SOURCE_NODE_TYPES,
  ...RESOURCE_NODE_TYPES.filter((t) => !UNLOADED_RESOURCE_NODE_TYPES.includes(t)),
  NodeType.CHANNEL,
  NodeType.CLAIM,
  NodeType.PLAN,
  NodeType.TASK,
  NodeType.ROLE,
  NodeType.AGENT,
  NodeType.TEAM,
  NodeType.SPACE,
  NodeType.VIEW,
];

export const TYPE_NODE_TYPES = [NodeType.AGENT, NodeType.CLASS, NodeType.CHOICE, NodeType.DATABASE];

// block types
export const BLOCK_TYPES = Object.values(BlockType).filter((v) => typeof v == "number" && v > 0) as BlockType[];
export const CANVAS_BLOCK_TYPES = [BlockType.PAGE, BlockType.KIT, BlockType.DATABASE];
export const TEXT_BLOCK_TYPES = BLOCK_TYPES.filter((bt) => bt >= BlockType.PARAGRAPH);

// action
export const ACTION_TYPES = Object.values(ActionType).filter((v) => typeof v == "number" && v > 0) as ActionType[];
export const FLOW_ACTION_TYPES = ACTION_TYPES.filter((st) => st < 100);
export const SOURCE_ACTION_TYPES = [ActionType.START];
export const SINK_ACTION_TYPES = [ActionType.END];

// run
export const TERMINAL_PROCESS_STATUSES = [
  ProcessStatus.CANCELLED,
  ProcessStatus.ABORTED,
  ProcessStatus.FAILED,
  ProcessStatus.COMPLETED,
  ProcessStatus.SKIPPED,
];
export const INTERRUPTED_PROCESS_STATUSES = [ProcessStatus.PAUSED, ProcessStatus.YIELDED, ProcessStatus.WAITING];
export const PRE_PROCESS_STATUSES = [
  ProcessStatus.CREATED,
  ProcessStatus.ASSIGNED,
  ProcessStatus.SCHEDULED,
  ProcessStatus.QUEUED,
];
export const ACTIVE_PROCESS_STATUSES = [ProcessStatus.RUNNING, ProcessStatus.FAILING];
export const INACTIVE_PROCESS_STATUSES = [ProcessStatus.IDLE];
export const BAD_PROCESS_STATUSES = [
  ProcessStatus.CANCELLED,
  ProcessStatus.ABORTED,
  ProcessStatus.FAILED,
  ProcessStatus.DIED,
];

// view
export const VIEW_TYPES = Object.values(ViewType).filter((v) => typeof v == "number" && v > 0) as ViewType[];
export const ROOT_VIEW_TYPES = new Set([ViewType.WINDOW, ViewType.TAB, ViewType.HISTORY, ViewType.SPLIT]);
export const HELPER_VIEW_TYPES = new Set(VIEW_TYPES.filter((vt) => vt >= 20000 && vt < 30000));

export const NAME_CONSTRAINT = ViewDataInfo[ViewProperty.name].constraint!;

/** Default base type for based Nodes */
export const BASE_TYPE_BY_NODE_TYPE: Partial<Record<NodeType, NodeType>> = {
  [NodeType.RECORD]: NodeType.DATABASE,
  [NodeType.MESSAGE]: NodeType.THREAD,
  [NodeType.RUN]: NodeType.FLOW,
};

/**
 * Gets the 'base' node defining a certain node. See :HasBase.
 */
export function getBaseFromNode(node: Partial<AnyNodeData>): NodeReferenceData | null {
  if (isNode(node, NodeType.RECORD)) {
    return node.databasePtr ?? null;
  } else if (isNode(node, NodeType.MESSAGE)) {
    return node.threadPtr ?? null;
  } else if (isNode(node, NodeType.RUN)) {
    return node.transitionPtr ?? node.actionPtr ?? node.flowPtr ?? null;
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

function hexToBase64(hex: string) {
  const bytes = [];
  for (let i = 0; i < hex.length; i += 2) {
    bytes.push(parseInt(hex.substr(i, 2), 16));
  }
  const str = String.fromCharCode(...bytes);
  return btoa(str);
}

/** Gets the 8-byte template key in base64 from a node ck */
export function getTkB64FromCk(ck: string) {
  const hex = ck.replace(/-/g, "");
  return hexToBase64(hex);
}

export function toCamelName<T extends object>(cls: T, key: any) {
  const name = cls[key as keyof T] as string;
  if (name == null) throw new Error(`invalid key into ${cls}: ${key}`);
  return toCasing(name, Casing.CAMEL, true);
}

//
// Enums
//

export const EXPOSED_NODE_TYPES = NODE_TYPES.filter((t) => t != NodeType.SKIP);
export const EXPOSED_BLOCK_TYPES = [
  BlockType.DATABASE,
  BlockType.PAGE,
  BlockType.TASK,
  BlockType.AGENT,
  ...TEXT_BLOCK_TYPES,
];
export const EXPOSED_STRUCT_TYPES = [
  // core
  StructType.TYPE,
  StructType.SCHEDULE,
  // files
  StructType.ICON,
  // code
  StructType.CODE,
  // views
  StructType.COLOR,
  StructType.FONT,
  StructType.OFFSET,
  StructType.RECTANGLE,
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
  PrimitiveType.DATE,
  PrimitiveType.DATETIME,
  PrimitiveType.TIME,
  PrimitiveType.DURATION,
];
export const EXPOSED_REGIONS = [Region.FRANKFURT];
export const EXPOSED_CONTINENTS = [Continent.EUROPE];
export const FILTERED_ENUMS: Partial<Record<EnumType, number[]>> = {
  [EnumType.NODE_TYPE]: EXPOSED_NODE_TYPES,
  [EnumType.BLOCK_TYPE]: EXPOSED_BLOCK_TYPES,
  [EnumType.STRUCT_TYPE]: EXPOSED_STRUCT_TYPES,
  [EnumType.OBJECT_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES],
  [EnumType.BENCH_TYPE]: [...NODE_TYPES, ...EXPOSED_STRUCT_TYPES, ...ENUM_TYPES],
  [EnumType.PRIMITIVE_TYPE]: EXPOSED_PRIMITIVE_TYPES,
  [EnumType.REGION]: EXPOSED_REGIONS,
  [EnumType.CONTINENT]: EXPOSED_CONTINENTS,
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

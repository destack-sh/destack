/**
 * Many constants are generated into proto/wire, here some additional ones.
 */

import {
  BenchType,
  NodeType,
  type AnyNodeData,
  NodeReferenceData,
  RecordData,
  RunData,
  SignalData,
  NotificationData,
  type NodeTypeMapping,
  NODE_PROPERTY_ENUM_BY_TYPE,
} from "@/proto/wire";

export const ROOT_NODE_TYPES = [NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH];
export const BASED_NODE_TYPES = [NodeType.RECORD, NodeType.RUN, NodeType.SIGNAL, NodeType.NOTIFICATION];

export function getBaseFromNode(node: AnyNodeData): NodeReferenceData | null {
  if (node.metatype == BenchType.RECORD) {
    return (node as RecordData).parentPtr ?? null;
  } else if (node.metatype == BenchType.RUN) {
    return (node as RunData).blockPtr ?? null;
  } else if (node.metatype == BenchType.SIGNAL || node.metatype == BenchType.NOTIFICATION) {
    return (node as SignalData | NotificationData).senderPtr ?? null;
  } else {
    return null;
  }
}

export function defaultSort<T extends NodeType>(metatype: T, nodes: NodeTypeMapping[T][]) {
  const properties = NODE_PROPERTY_ENUM_BY_TYPE[metatype as unknown as BenchType]!;
  if ("orderKey" in properties) nodes.sort((a, b) => ((a as any).orderKey ?? "").localeCompareTo((b as any).orderKey));
  else nodes.sort((a, b) => (b.createdAt?.nanos ?? 0) - (a.createdAt?.nanos ?? 0));
}

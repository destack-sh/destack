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
  ViewType,
} from "@/proto/wire";
import type { Transaction } from "@/system/transaction";
import { generateKeyBetween } from "@/utils/fractional";

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
export const LOADED_SOURCE_NODE_TYPES = [
  NodeType.PACKAGE,
  NodeType.DEPENDENCY,
  NodeType.UPGRADE,
  NodeType.SPACE,
  NodeType.LINK,
  NodeType.SKIP,
  NodeType.NOTICE,
  NodeType.BLOCK,
  NodeType.TRIGGER,
  NodeType.FIELD,
  NodeType.RECORD, // nocheckin: should error because crosses store boundaries (then remove Record from loaded)
  NodeType.QUERY,
  NodeType.VIEW,
];

/**
 * Gets the 'base' node defining a certain node. See HasBase.
 */
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

/**
 * Sorts the given nodes using explicit order keys if available.
 */
export function defaultSort<T extends NodeType>(metatype: T, nodes: NodeTypeMapping[T][]) {
  const properties = NODE_PROPERTY_ENUM_BY_TYPE[metatype as unknown as BenchType]!;
  if ("orderKey" in properties)
    nodes.sort((a, b) => ((a as any).orderKey ?? "").localeCompare((b as any).orderKey ?? ""));
  else nodes.sort((a, b) => (b.createdAt?.nanos ?? 0) - (a.createdAt?.nanos ?? 0));
}

/**
 * Sets the node order keys so that the target is position relative to the reference. Nodes must be in order.
 * If any positions need to be 'fixed' because of previous duplicates, these updates are also included.
 */
export function updateOrderKey<T extends AnyNodeData & { orderKey: string }>(order: {
  tx: Transaction;
  target: T;
  position: "before" | "after";
  reference: T | null;
  nodes: T[];
}) {
  const orderKey = getOrderKey<T>(order);
  // @ts-ignore: orderKey must be present at this point
  order.tx.update({ ...order.target, orderKey });

  // TODO :Robustness: fix order keys if there are duplicates
  //  (may happen if two nodes are created in the same place simultaneously)
}

/**
 * Gets the order key relative to the reference. Nodes must be in order.
 */
export function getOrderKey<T extends AnyNodeData & { orderKey: string }>(order: {
  position: "before" | "after";
  reference: T | null;
  nodes: T[];
}) {
  let orderKey;
  if (order.position == "before") {
    const a =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) - 1];
    orderKey = generateKeyBetween(a?.orderKey ?? null, order.reference?.orderKey ?? null);
  } else {
    const b =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) + 1];
    orderKey = generateKeyBetween(order.reference?.orderKey ?? null, b?.orderKey ?? null);
  }
  return orderKey;
}

export const ROOT_VIEW_TYPES = [ViewType.WINDOWED, ViewType.WINDOW, ViewType.TABBED, ViewType.SPLIT];

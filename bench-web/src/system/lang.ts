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
  IconData,
  BlockType,
} from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import type { Transaction } from "@/system/transaction";
import { generateKeyBetween, generateNKeysBetween } from "@/utils/fractional";
import { Casing, toCasing } from "@/utils/string";

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
  referenceId: string | null;
  nodes: () => T[];
}) {
  let orderKey;
  let nodes = order.nodes();
  try {
    orderKey = getOrderKey<T>({
      nodes,
      position: order.position,
      reference: order.referenceId == null ? null : nodes.find((n) => n.id == order.referenceId) ?? null,
    });
  } catch {
    // 'fix' order keys if we couldn't generate one
    //  (usually because of duplicates, we don't enforce uniqueness per order key for simplicity)
    fixOrderKeys(order.tx, nodes);
    nodes = order.nodes(); // 'refresh' to apply tx changes
    orderKey = getOrderKey<T>({
      nodes,
      position: order.position,
      reference: order.referenceId == null ? null : nodes.find((n) => n.id == order.referenceId) ?? null,
    });
  }
  // @ts-ignore: orderKey must exist
  order.tx.update({ ...order.target, orderKey });
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
    orderKey = generateKeyBetween(a?.orderKey ?? null, order.reference?.orderKey ?? null);
  } else {
    const b =
      order.reference?.id == null ? null : order.nodes[order.nodes.findIndex((n) => n.id == order.reference!.id) + 1];
    orderKey = generateKeyBetween(order.reference?.orderKey ?? null, b?.orderKey ?? null);
  }
  return orderKey;
}

/**
 * Patches any broken order keys to put the nodes in the given order.
 */
export function fixOrderKeys<T extends AnyNodeData & { orderKey: string }>(tx: Transaction, nodes: T[]) {
  // ensure nodes are in current order
  nodes.sort((a, b) => a.orderKey.localeCompare(b.orderKey));

  // scan for successive duplicates (they must be successive now)
  let i = 0;
  while (i < nodes.length) {
    const prevOrderKey = i == 0 ? null : nodes[i - 1].orderKey;
    const node = nodes[i];
    if (node.orderKey == prevOrderKey) {
      // find all duplicates with same key from here and fix them in one go
      const numDuplicates = nodes.slice(i).filter((n) => n.orderKey == node.orderKey).length;
      const duplicates = nodes.slice(i, i + numDuplicates);
      const orderKeys = generateNKeysBetween(prevOrderKey, nodes[i + numDuplicates]?.orderKey ?? null, numDuplicates);
      for (let j = 0; j < numDuplicates; j++) {
        // @ts-ignore: orderKey must exist
        tx.update({ ...duplicates[j], orderKey: orderKeys[j] });
      }
      i += numDuplicates;
    } else {
      i++;
    }
  }
}

export const DEFAULT_MISSING_ICON = makeIcon({ name: "fas fa-question" });

export const ROOT_VIEW_TYPES = [ViewType.WINDOWED, ViewType.WINDOW, ViewType.TABBED, ViewType.SPLIT];
export const ROOT_VIEW_COMPONENT_NAMES = ROOT_VIEW_TYPES.map((t) => toCasing(ViewType[t], Casing.CAMEL));

function _makeIcons<K extends string | number>(icons: Partial<Record<K, string | IconData>>): Record<K, IconData> {
  return Object.fromEntries(
    Object.entries(icons).map(([key, value]) => {
      return [key as K, typeof value == "string" ? makeIcon({ name: value as string }) : value];
    }),
  ) as Record<K, IconData>;
}

export const ICON_BY_NODE_TYPE: Partial<Record<NodeType, IconData>> = _makeIcons<NodeType>({
  // root
  [NodeType.BENCH]: "fas ca-castle",
  [NodeType.ENVIRONMENT]: "fas fa-globe",
  [NodeType.BRANCH]: "fas fa-code-branch",

  // source
  [NodeType.PACKAGE]: "fas fa-box-open",
  [NodeType.DEPENDENCY]: "fas fa-turn-down-right",
  [NodeType.UPGRADE]: "fas fa-circle-up",
  [NodeType.SPACE]: "fas fa-browser",
  [NodeType.LINK]: "fas fa-link",
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.NOTICE]: "fas fa-square-exclamation",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.TRIGGER]: "fas fa-bolt",
  // [NodeType.FIELD]: "fas fa-font",
  [NodeType.RECORD]: "fas fa-database",
  [NodeType.QUERY]: "fas fa-magnifying-glass",
  [NodeType.VIEW]: "fas fa-window",

  // auth
  [NodeType.BADGE]: "fas fa-id-badge",
  [NodeType.ROLE]: "fas fa-user-tag",
  [NodeType.IDENTITY]: "fas fa-image-user",
  [NodeType.MEMBERSHIP]: "fas fa-users",
  [NodeType.INVITE]: "fas fa-envelope",

  // runtime
  [NodeType.SESSION]: "fas fa-circle-play",
  [NodeType.RUN]: "fas fa-play",
  [NodeType.PAUSE]: "fas fa-pause",
  [NodeType.SIGNAL]: "fas fa-signal-stream",
  [NodeType.LOG]: "fas fa-file-alt",
  [NodeType.NOTIFICATION]: "fas fa-bell",

  // resources
  [NodeType.SERVER]: "fas fa-server",
  [NodeType.STORE]: "fas fa-database",
  [NodeType.DRIVE]: "fas fa-hdd",
  [NodeType.CACHE]: "fas fa-memory",
  [NodeType.FILE_CONTENT]: "fas fa-file",

  // user
  [NodeType.HANDLE]: "fas fa-at",
  [NodeType.USER]: "fas fa-user",
  [NodeType.ORGANIZATION]: "fas fa-building",
  [NodeType.CLIENT]: "fas fa-desktop",
});

export const ICON_BY_BLOCK_TYPE: Partial<Record<BlockType, IconData>> = _makeIcons<BlockType>({
  [BlockType.PAGE]: "fas fa-page",
  [BlockType.BLANK]: "fas fa-square",
  [BlockType.TEXT]: "fas fa-font",
  [BlockType.ALIAS]: "fas fa-link",

  [BlockType.CLASS]: "fas fa-objects-column",
  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.SIGNAL]: "fas fa-signal-stream",
  [BlockType.PROTOCOL]: "fas fa-list-check",

  [BlockType.SINGLE_VARIABLE]: "fas fa-columns-3",
  [BlockType.MULTI_VARIABLE]: "fas fa-columns-3",

  [BlockType.MODEL_ROUTINE]: "fas fa-function",
  [BlockType.CODE_ROUTINE]: "fas fa-code",
  [BlockType.SCRIPT]: "fas fa-file-code",
  [BlockType.FLOW]: "fas fa-diagram-project",

  [BlockType.QUERY]: "fas fa-magnifying-glass",
  [BlockType.DATABASE]: "fas fa-database",

  [BlockType.SCREEN]: "fas fa-window",

  [BlockType.ROLE]: "fas fa-user-tag",
  [BlockType.IDENTITY]: "fas fa-image-user",
});

export function getNodeTypeIcon(nodeType: NodeType) {
  return ICON_BY_NODE_TYPE[nodeType] ?? DEFAULT_MISSING_ICON;
}

export function getBlockTypeIcon(blockType: BlockType) {
  return ICON_BY_BLOCK_TYPE[blockType] ?? DEFAULT_MISSING_ICON;
}

export function getNodeIcon(node: { metatype: BenchType; type?: BlockType | ViewType }) {
  // TODO :Incomplete: view type icons
  if (node.metatype == BenchType.BLOCK) return getBlockTypeIcon(node.type! as BlockType);
  else return getNodeTypeIcon(node.metatype as unknown as NodeType);
}

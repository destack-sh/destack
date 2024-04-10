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
  BlockData,
  ViewData,
} from "@/proto/wire";
import { makeIcon } from "@/system/icon";
import type { Transaction } from "@/system/transaction";
import { generateKeyBetween, generateNKeysBetween } from "@/utils/fractional";
import { Casing, toCasing } from "@/utils/string";
import type { AnyNode } from "postcss";

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
  NodeType.NOTICE,
  NodeType.BLOCK,
  NodeType.TRIGGER,
  NodeType.FIELD,
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
 * Sorts the given nodes using explicit order keys if available, createdAt otherwise, then id.
 */
export function defaultSort(nodes: AnyNodeData[]): void {
  nodes.sort((a, b) => {
    if ((a as any).orderKey != null && (b as any).orderKey != null && (a as any).orderKey != (b as any).orderKey) {
      return (a as any).orderKey.localeCompare((b as any).orderKey);
    } else if (a.createdAt != null && b.createdAt != null && a.createdAt.seconds != b.createdAt.seconds) {
      return Number(b.createdAt.seconds - a.createdAt.seconds);
    } else {
      return a.id.localeCompare(b.id);
    }
  });
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
  order.tx.update({ ...order.target, orderKey }, ["orderKey"]);
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
        tx.update({ ...duplicates[j], orderKey: orderKeys[j] }, ["orderKey"]);
      }
      i += numDuplicates;
    } else {
      i++;
    }
  }
}

export const DEFAULT_MISSING_ICON = makeIcon({ name: "fas fa-question" });
export const DEFAULT_VIEW_ICON = makeIcon({ name: "fas fa-browser" });
export const DEFAULT_USER_ICON = makeIcon({ name: "fas fa-user-tie" });
export const DEFAULT_BENCH_ICON = makeIcon({ name: "fas fa-fort" });

export const ROOT_VIEW_TYPES = new Set([ViewType.WINDOW, ViewType.TAB, ViewType.SPLIT]);
export const ROOT_VIEW_COMPONENT_NAMES = new Set(
  Array.from(ROOT_VIEW_TYPES.keys()).map((t) => toCasing(ViewType[t], Casing.CAMEL)),
);
// views that have a white background
export const FULL_VIEW_TYPES = new Set([
  ViewType.PAGE,
  ViewType.BLOCK,
  ViewType.DATABASE,
  ViewType.EXPLORER,
  ViewType.OUTLINE,
  ViewType.LIBRARY,
  ViewType.INSPECTOR,
]);
// views that aren't about a specific node but should just keep the current root view node
export const RIDEALONG_VIEW_TYPES = new Set([
  ViewType.EXPLORER,
  ViewType.OUTLINE,
  ViewType.LIBRARY,
  ViewType.INSPECTOR,
]);

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
  [NodeType.SPACE]: "fas fa-galaxy",
  [NodeType.LINK]: "fas fa-link",
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.NOTICE]: "fas fa-square-exclamation",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.TRIGGER]: "fas fa-bolt",
  [NodeType.FIELD]: "fas fa-font",
  [NodeType.RECORD]: "fas fa-database",
  [NodeType.QUERY]: "fas fa-magnifying-glass",
  [NodeType.VIEW]: "fas fa-browser",

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
  [BlockType.PAGE]: "fas fa-folder",
  [BlockType.BLANK]: "fas fa-cube",
  [BlockType.TEXT]: "fas fa-font",
  [BlockType.ALIAS]: "fas fa-link",

  [BlockType.CLASS]: "fas fa-objects-column",
  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.SIGNAL]: "fas fa-signal-stream",
  [BlockType.PROTOCOL]: "fas fa-list-check",

  [BlockType.SINGLE_VARIABLE]: "fas fa-columns-3",
  [BlockType.MULTI_VARIABLE]: "fas fa-columns-3",

  [BlockType.NATURAL_ROUTINE]: "fas fa-text",
  [BlockType.CODE_ROUTINE]: "fas fa-code",
  [BlockType.SCRIPT]: "fas fa-file-code",
  [BlockType.FLOW]: "fas fa-diagram-project",

  [BlockType.QUERY]: "fas fa-magnifying-glass",
  [BlockType.DATABASE]: "fas fa-database",

  [BlockType.SCREEN]: "fas fa-window",

  [BlockType.ROLE]: "fas fa-user-tag",
  [BlockType.IDENTITY]: "fas fa-image-user",
});

export const ICON_BY_VIEW_TYPE: Partial<Record<ViewType, IconData>> = _makeIcons<ViewType>({
  //
  // Intrinsics
  //

  // kernel
  [ViewType.USER_WIZARD]: "fas fa-user",
  [ViewType.BENCH_WIZARD]: "fas fa-fort",
  [ViewType.CHALLENGE_WIZARD]: "fas fa-trophy",
  [ViewType.KEYMAP]: "fas fa-keyboard",
  [ViewType.MOCK]: "fas fa-bug",

  // system
  [ViewType.PAGE]: "fas fa-folder",
  [ViewType.BLOCK]: "fas fa-cube",
  [ViewType.FIELD]: "fas fa-font",
  [ViewType.DATABASE]: "fas fa-database",
  [ViewType.EXPLORER]: "fas fa-compass",
  [ViewType.OUTLINE]: "fas fa-list-tree",
  [ViewType.INSPECTOR]: "fas fa-eye",
  [ViewType.HISTORY]: "fas fa-history",
  [ViewType.RESOURCE]: "fas fa-box",
  [ViewType.LIBRARY]: "fas fa-books",
  [ViewType.LOG]: "fas fa-file-alt",

  //
  // General
  //

  // containers (root)
  [ViewType.WINDOW]: "fas fa-window",
  [ViewType.TAB]: "fas fa-sidebar",
  [ViewType.SPLIT]: "fas fa-reflect-horizontal",
  [ViewType.SPLIT_COLLAPSIBLE]: "fas fa-reflect-horizontal",
  // containers (layout)
  [ViewType.WIZARD]: "fas fa-hat-wizard",
  [ViewType.STACK]: "fas fa-layer-group",
  [ViewType.COLLAPSIBLE]: "fas fa-chevron-circle-down",
  [ViewType.GRID]: "fas fa-table-cells-large",
  [ViewType.ROW]: "fas fa-table-rows",
  [ViewType.COLUMN]: "fas fa-table-columns",
  // containers (data)
  [ViewType.LIST]: "fas fa-list",
  [ViewType.TABLE]: "fas fa-table",
  [ViewType.FEED]: "fas fa-list-timeline",
  // containers (group)
  [ViewType.GROUP]: "fas fa-object-group",
  [ViewType.SECTION]: "fas fa-xmark-lines",

  // presentation
  [ViewType.SPACER]: "fas fa-square-dashed",
  [ViewType.DIVIDER]: "fas fa-horizontal-rule",
  [ViewType.SHAPE]: "fas fa-shapes",
  [ViewType.PROGRESS]: "fas fa-spinner",
  [ViewType.AVATAR]: "fas fa-user-circle",
  [ViewType.BADGE]: "fas fa-badge",
  [ViewType.CHART]: "fas fa-chart-pie",

  // controls
  [ViewType.BUTTON]: "fas fa-hand-pointer",
  [ViewType.MULTI_BUTTON]: "fas fa-hand-pointer",
  [ViewType.LINK]: "fas fa-link",

  // content
  [ViewType.VALUE]: "fas fa-box",
  // numeric
  [ViewType.NUMBER]: "fas fa-hashtag",
  [ViewType.SLIDER]: "fas fa-slider",
  // stringy
  [ViewType.PLAIN_TEXT]: "fas fa-text",
  [ViewType.TEXT]: "fas fa-font",
  [ViewType.CODE]: "fas fa-code",
  [ViewType.JSON]: "fas fa-brackets-curly",
  // selection
  [ViewType.TOGGLE]: "fas fa-toggle-large-on",
  [ViewType.PICKER]: "fas fa-caret-circle-down",
  [ViewType.DATE]: "fas fa-calendar-days",
  [ViewType.TIME]: "fas fa-clock",
  [ViewType.CALENDAR]: "fas fa-calendar",
  [ViewType.COLOR]: "fas fa-palette",
  // file
  [ViewType.FILE]: "fas fa-file",
  [ViewType.ICON]: "fas fa-icons",
  [ViewType.IMAGE]: "fas fa-image",
  [ViewType.VIDEO]: "fas fa-video",
  [ViewType.AUDIO]: "fas fa-volume",
});

export function getNodeTypeIcon(nodeType: NodeType) {
  return ICON_BY_NODE_TYPE[nodeType] ?? DEFAULT_MISSING_ICON;
}

export function getBlockTypeIcon(blockType: BlockType) {
  return ICON_BY_BLOCK_TYPE[blockType] ?? DEFAULT_MISSING_ICON;
}

export function getViewTypeIcon(viewType: ViewType) {
  return ICON_BY_VIEW_TYPE[viewType] ?? DEFAULT_VIEW_ICON;
}

export function getNodeIcon(node: AnyNodeData | { metatype: BenchType; type?: BlockType | ViewType }) {
  if (node.metatype == BenchType.BLOCK) {
    const icon = getBlockTypeIcon((node as BlockData).type! as BlockType);
    if (icon != null) return icon;
  } else if (node.metatype == BenchType.VIEW) {
    const icon = getViewTypeIcon((node as ViewData).type! as ViewType);
    if (icon != null) return icon;
  }
  return getNodeTypeIcon(node.metatype as unknown as NodeType);
}

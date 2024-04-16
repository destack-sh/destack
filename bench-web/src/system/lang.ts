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
import { generateOrderKey, generateNKeysBetween, isValidOrderKey } from "@/utils/fractional";
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
  NodeType.STEP,
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
      return (a as any).orderKey > (b as any).orderKey ? 1 : -1;
    } else if (a.createdAt != null && b.createdAt != null && a.createdAt.seconds != b.createdAt.seconds) {
      return Number(b.createdAt.seconds - a.createdAt.seconds);
    } else {
      return a.id > b.id ? 1 : -1;
    }
  });
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
  position: "before" | "after";
  reference: T | null;
  nodes: T[];
}) {
  let orderKey;
  if (order.position == "before") {
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
  defaultSort(nodes);

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

export const DEFAULT_MISSING_ICON = makeIcon({ faName: "fas fa-question" });
export const DEFAULT_VIEW_ICON = makeIcon({ faName: "fas fa-browser" });
export const DEFAULT_USER_ICON = makeIcon({ faName: "fas fa-user-tie" });
export const DEFAULT_BENCH_ICON = makeIcon({ faName: "fas fa-fort" });

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
// views that have a white background
export const FULL_VIEW_TYPES = new Set<ViewType>([
  ...NODE_VIEW_TYPES,
  ViewType.EXPLORE,
  ViewType.OUTLINE,
  ViewType.CREATE,
  ViewType.INSPECT,
]);
// views that aren't about a specific node but should just keep the current root view node
export const RIDEALONG_VIEW_TYPES = new Set([ViewType.EXPLORE, ViewType.OUTLINE, ViewType.CREATE, ViewType.INSPECT]);

function _makeIcons<K extends string | number>(icons: Partial<Record<K, string | IconData>>): Record<K, IconData> {
  return Object.fromEntries(
    Object.entries(icons).map(([key, value]) => {
      return [key as K, typeof value == "string" ? makeIcon({ faName: value as string }) : value];
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
  [BlockType.PAGE]: "fas fa-memo",
  [BlockType.BLANK]: "fas fa-empty-set",
  [BlockType.ALIAS]: "fas fa-link",

  [BlockType.CLASS]: "fas fa-objects-column",
  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.SIGNAL]: "fas fa-signal-stream",
  [BlockType.PROTOCOL]: "fas fa-list-check",

  [BlockType.VARIABLE]: "fas fa-sliders",
  [BlockType.MULTI_VARIABLE]: "fas fa-sliders",

  [BlockType.TEXT]: "fas fa-text",
  [BlockType.CODE]: "fas fa-code",
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
  [ViewType.PAGE]: "fas fa-memo-pad",
  [ViewType.BLOCK]: "fas fa-cube",
  [ViewType.FIELD]: "fas fa-font",
  [ViewType.DATABASE]: "fas fa-database",
  [ViewType.EXPLORE]: "fas fa-compass",
  [ViewType.OUTLINE]: "fas fa-list-tree",
  [ViewType.INSPECT]: "fas fa-eye",
  [ViewType.CREATE]: "fas fa-hexagon-plus",

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
  [ViewType.JSON]: "fas fa-binary",
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

export function getNodeTypeIcon(nodeType: NodeType): IconData | null {
  return ICON_BY_NODE_TYPE[nodeType] ?? null;
}

export function getBlockTypeIcon(blockType: BlockType): IconData | null {
  return ICON_BY_BLOCK_TYPE[blockType] ?? null;
}

export function getViewTypeIcon(viewType: ViewType): IconData | null {
  return ICON_BY_VIEW_TYPE[viewType] ?? null;
}

export function getNodeIcon(node: AnyNodeData | { metatype: BenchType; type?: BlockType | ViewType }) {
  if ((node as any).icon != null) {
    return (node as any).icon;
  } else if (node.metatype == BenchType.BLOCK) {
    const icon = getBlockTypeIcon((node as BlockData).type! as BlockType);
    if (icon != null) return icon;
  } else if (node.metatype == BenchType.VIEW) {
    const icon = getViewTypeIcon((node as ViewData).type! as ViewType);
    if (icon != null) return icon;
  }
  return getNodeTypeIcon(node.metatype as unknown as NodeType);
}

export function toCamelName<T extends object>(cls: T, key: any) {
  return toCasing(cls[key as keyof T] as string, Casing.CAMEL);
}

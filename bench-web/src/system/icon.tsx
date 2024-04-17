import {
  IconKind,
  type IconData,
  ObjectType,
  ViewType,
  NodeType,
  BlockType,
  type AnyNodeData,
  BlockData,
  ViewData,
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";

export const IconInline: FunctionalComponent<Pick<IconData, "emoji" | "file" | "faName">> = (props) => {
  if (props.faName) {
    // font awesome
    return <i class={props.faName + " w-[18px] text-center"} />;
  } else if (props.emoji) {
    return <span>{props.emoji}</span>;
  } else {
    throw new Error(`unexpected icon ${props}`);
  }
};

type IconIn = string | Pick<IconData, "emoji" | "file" | "faName">;
export function makeIcon(icon: IconIn): IconData {
  let kind: IconKind;
  if (typeof icon == "string") {
    return { metatype: ObjectType.ICON, kind: IconKind.FONT_AWESOME, faName: icon, setProperties: [] };
  } else if (icon.emoji) {
    kind = IconKind.EMOJI;
  } else if (icon.file) {
    kind = IconKind.FILE;
  } else if (icon.faName) {
    kind = IconKind.FONT_AWESOME;
  } else {
    throw new Error(`unexpected icon ${icon}`);
  }
  return {
    metatype: ObjectType.ICON,
    kind,
    ...icon,
    setProperties: [],
  };
}

export function toIconMaybe(icon?: IconIn | null): IconData | undefined {
  if (icon == null) return undefined;
  return makeIcon(icon);
}

function _makeIcons<K extends string | number>(icons: Partial<Record<K, string | IconData>>): Record<K, IconData> {
  return Object.fromEntries(
    Object.entries(icons).map(([key, value]) => {
      return [key as K, typeof value == "string" ? makeIcon({ faName: value as string }) : value];
    }),
  ) as Record<K, IconData>;
}

export const DEFAULT_MISSING_ICON = makeIcon({ faName: "fas fa-question" });
export const DEFAULT_VIEW_ICON = makeIcon({ faName: "fas fa-browser" });
export const DEFAULT_USER_ICON = makeIcon({ faName: "fas fa-user-tie" });
export const DEFAULT_BENCH_ICON = makeIcon({ faName: "fas fa-fort" });

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

export function getNodeIcon(node: AnyNodeData | { metatype: ObjectType; type?: BlockType | ViewType }) {
  if ((node as any).icon != null) {
    return (node as any).icon;
  } else if (node.metatype == ObjectType.BLOCK) {
    const icon = getBlockTypeIcon((node as BlockData).type! as BlockType);
    if (icon != null) return icon;
  } else if (node.metatype == ObjectType.VIEW) {
    const icon = getViewTypeIcon((node as ViewData).type! as ViewType);
    if (icon != null) return icon;
  }
  return getNodeTypeIcon(node.metatype as unknown as NodeType);
}

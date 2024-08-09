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
  EnumType,
  PrimitiveType,
  StructType,
  BenchType,
  FieldZone,
  FieldData,
  ColorType,
  ColorShade,
  ColorData,
  LogLevel,
  Alignment,
  StepType,
  Anchor,
  EditType,
  RunStatus,
  FileType,
  RegionArea,
  Region,
  ResourceStatus,
  FileFormat,
  TypeKind,
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";
// fa-icons is generated with:
// curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
//  | jq 'to_entries | map(select(.value.free | index("solid") or index("brands")) | {"id": .key, label: .value.label, unicode: .value.unicode, alias: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
//  > fa-icons.json
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import { IS_DEV, isDeveloperMode } from "@/utils/globals";
import { getColorHex, makeColor } from "@/ui/style";
import { isNode, isNodeRef, isStruct, type SomeNodeReferenceData } from "@/proto/wiring";

export type IconMetadata = {
  id: string;
  title: string;
  unicode: string;
  alias: string[];
  family: "fas" | "fab";
  faName: string;
};
export function metadataToIcon(metadata: IconMetadata, color: ColorData | undefined): IconData {
  return {
    metatype: ObjectType.ICON,
    kind: IconKind.FONT_AWESOME,
    faName: `${metadata.family} fa-${metadata.id}`,
    color,
  };
}

export const AVAILABLE_FA_ICONS: IconMetadata[] = _AVAILABLE_FA_ICONS.map((i) => ({
  ...i,
  faName: `${i.family} fa-${i.id}`,
})) as IconMetadata[];
export const AVAILABLE_ICONS_BY_ID: Record<string, IconMetadata> = Object.fromEntries(
  AVAILABLE_FA_ICONS.map((i) => [i.id, i]),
);

type IconInlineProps = Pick<IconData, "emoji" | "faName"> & {
  color?: ColorType | ColorData;
  shade?: ColorShade;
  forceColor?: "inherit" | ColorType;
  fallbackColor?: ColorType;
};
export const IconInline: FunctionalComponent<IconInlineProps> = (props) => {
  let colorHex;
  if (props.forceColor == "inherit") colorHex = undefined;
  else if (props.color != null) colorHex = getColorHex(props.color, props.shade);
  else if (props.forceColor != null) colorHex = getColorHex(props.forceColor, props.shade);
  else if (props.fallbackColor != null) colorHex = getColorHex(props.fallbackColor, props.shade);
  else colorHex = undefined;
  if (props.faName) {
    // font awesome
    return <i class={`${props.faName} text-center`} style={{ color: colorHex }} />;
  } else if (props.emoji) {
    return <span style={{ color: colorHex }}>{props.emoji}</span>;
  } else {
    if (IS_DEV || isDeveloperMode.value) return <span class="text-danger-500">?icon: {JSON.stringify(props)}</span>;
    else return <span style={{ color: colorHex }}>???</span>;
  }
};
IconInline.props = ["emoji", "file", "faName", "color", "fallbackColor", "shade", "ignoreColor"];

export function getIconMetadata(icon: IconData): IconMetadata | undefined {
  if (icon.kind == IconKind.FONT_AWESOME) {
    const id = icon.faName!.split(" ")[1].slice(3);
    return AVAILABLE_ICONS_BY_ID[id];
  } else {
    return undefined;
  }
}

export function newIconId(): number {
  /** Exactly like newStructId for now (but want to avoid importing it due to circularity) */
  return Math.floor(Math.random() * 0x7fffffff);
}

type ColorIn = ColorData | ColorType;
type IconIn = string | (Pick<IconData, "emoji" | "faName"> & { color?: ColorIn });
export function makeIcon(icon: IconIn): IconData {
  let kind: IconKind;
  if (typeof icon == "string") {
    if (icon.startsWith("fa")) {
      return { metatype: ObjectType.ICON, kind: IconKind.FONT_AWESOME, faName: icon };
    } else {
      return { metatype: ObjectType.ICON, kind: IconKind.EMOJI, emoji: icon };
    }
  } else if (icon.emoji) {
    kind = IconKind.EMOJI;
  } else if (icon.faName) {
    kind = IconKind.FONT_AWESOME;
  } else {
    throw new Error(`unexpected icon ${icon}`);
  }
  const color = icon.color != null && typeof icon.color != "object" ? makeColor(icon.color) : icon.color;
  return { metatype: ObjectType.ICON, kind, ...icon, color };
}

export function toIconMaybe(icon?: IconIn | null): IconData | undefined {
  if (icon == null) return undefined;
  return makeIcon(icon);
}

function _makeIcons<K extends string | number>(icons: Partial<Record<K, IconIn>>): Record<K, IconData> {
  return Object.fromEntries(
    Object.entries(icons).map(([key, value]) => {
      return [key as K, makeIcon(value as IconIn)];
    }),
  ) as Record<K, IconData>;
}

export const DEFAULT_MISSING_ICON = makeIcon({ faName: "fas fa-question" });
export const DEFAULT_VIEW_ICON = makeIcon({ faName: "fas fa-browser" });
export const DEFAULT_USER_ICON = makeIcon({ faName: "fas fa-user-tie" });
export const DEFAULT_BENCH_ICON = makeIcon({ faName: "fas fa-circle-dot" });
export const DEFAULT_ENUM_ICON = makeIcon({ faName: "fas fa-caret-circle-down" });

export const ICON_BY_NODE_TYPE: Partial<Record<NodeType, IconData>> = _makeIcons<NodeType>({
  // universe
  [NodeType.BENCH]: "fas fa-circle-dot",
  [NodeType.HANDLE]: "fas fa-at",
  [NodeType.USER]: "fas fa-user",
  [NodeType.ORGANIZATION]: "fas fa-building",
  [NodeType.CLIENT]: "fas fa-desktop",

  // resources
  [NodeType.SERVER]: "fas fa-server",
  [NodeType.MACHINE]: "fas fa-desktop",
  [NodeType.STORE]: "fas fa-database",
  [NodeType.DRIVE]: "fas fa-hdd",
  [NodeType.FILE]: "fas fa-file",

  // source
  [NodeType.BRANCH]: "fas fa-code-branch",
  [NodeType.PACKAGE]: "fas fa-box-open",
  [NodeType.DEPENDENCY]: "fas fa-turn-down-right",
  [NodeType.SPACE]: "fas fa-galaxy",
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.TRIGGER]: "fas fa-bolt",
  [NodeType.FIELD]: "fas fa-triangle",
  [NodeType.QUERY]: "fas fa-magnifying-glass",
  [NodeType.VIEW]: "fas fa-browser",
  [NodeType.STEP]: "fas fa-step-forward",
  [NodeType.BADGE]: "fas fa-id-badge",
  [NodeType.MEMBERSHIP]: "fas fa-book-user",
  [NodeType.INVITE]: "fas fa-circle-nodes",

  // runtime
  [NodeType.SESSION]: "fas fa-circle-play",
  [NodeType.RUN]: "fas fa-play",
  [NodeType.SIGNAL]: "fas fa-signal-stream",
  [NodeType.LOG]: "fas fa-file-alt",
  [NodeType.NOTIFICATION]: "fas fa-bell",
  [NodeType.MESSAGE]: "fas fa-message",
  [NodeType.RECORD]: "fas fa-database",
});

export const ICON_BY_STRUCT_TYPE: Partial<Record<StructType, IconData>> = _makeIcons<StructType>({
  // core
  [StructType.PATH]: "fas fa-path",
  [StructType.TYPE_INFO]: "fas fa-tilde",
  [StructType.SCHEDULE]: "fas fa-calendar",
  [StructType.TRIGGER_INFO]: "fas fa-bolt",
  // files
  [StructType.ICON]: "fas fa-icons",
  // code
  [StructType.CODE]: "fas fa-code",
  // expression
  [StructType.EXPRESSION]: "fas fa-sigma",
  [StructType.SELECTION]: "fas fa-check",
  // views
  [StructType.COLOR]: "fas fa-palette",
  [StructType.FONT]: "fas fa-font",
  [StructType.OFFSET]: "fas fa-arrows-alt",
  [StructType.BOX]: "fas fa-box",
  // access
  [StructType.POLICY]: "fas fa-shield-check",
  [StructType.POLICY_RULE]: "fas fa-shield-check",
  // flow
  [StructType.PIPE]: "fas fa-arrow-right",
  // text
  [StructType.TEXT]: "fas fa-text",
  [StructType.TEXT_LINE]: "fas fa-grip-lines",
  // run
  [StructType.RUN_OPTIONS]: "fas fa-play",
  [StructType.RUN_ATTEMPT]: "fas fa-play",
  [StructType.RUN_ERROR]: "fas fa-play",
  [StructType.RUN_FRAME]: "fas fa-play",
  [StructType.RUN_TRACE]: "fas fa-play",
  [StructType.BREAKPOINT]: "fas fa-pause",
  [StructType.MODEL_OPTIONS]: "fas fa-play",
  [StructType.LOG_INFO]: "fas fa-file-lines",
});

export const ICON_BY_OBJECT_TYPE: Partial<Record<ObjectType, IconData>> = {
  ...ICON_BY_NODE_TYPE,
  ...ICON_BY_STRUCT_TYPE,
};

export const ICON_BY_BENCH_TYPE: Partial<Record<BenchType, IconData>> = {
  ...ICON_BY_OBJECT_TYPE,
};

export const ICON_BY_BLOCK_TYPE: Partial<Record<BlockType, IconData>> = _makeIcons<BlockType>({
  [BlockType.PAGE]: "fas fa-memo",

  [BlockType.CLASS]: "fas fa-objects-column",
  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.SIGNAL]: "fas fa-signal-stream",

  [BlockType.TEXT]: "fas fa-text",
  [BlockType.CODE]: "fas fa-code",
  [BlockType.FLOW]: "fas fa-diagram-project",

  [BlockType.VARIABLE]: "fas fa-sliders",
  [BlockType.QUERY]: "fas fa-magnifying-glass",
  [BlockType.DATABASE]: "fas fa-database",

  [BlockType.VIEW]: "fas fa-window",

  [BlockType.ROLE]: "fas fa-user-tag",
  [BlockType.IDENTITY]: "fas fa-image-user",
});

export const ICON_BY_VIEW_TYPE: Partial<Record<ViewType, IconData>> = _makeIcons<ViewType>({
  //
  // Intrinsics
  //

  // kernel
  [ViewType.USER_WIZARD]: "fas fa-user",
  [ViewType.BENCH_WIZARD]: "fas fa-circle-dot",
  [ViewType.EMPTY]: "fas fa-bug",

  // system
  // nodes
  [ViewType.PAGE]: "fas fa-memo-pad",
  [ViewType.BLOCK]: "fas fa-cube",
  [ViewType.FIELD]: "fas fa-font",
  [ViewType.DATABASE]: "fas fa-database",
  [ViewType.TYPE]: "fas fa-objects-column",
  [ViewType.VARIABLE]: "fas fa-sliders",
  // helpers
  [ViewType.TREE]: "fas fa-list-tree",
  [ViewType.INSPECT]: "fas fa-eye",
  [ViewType.CREATE]: "fas fa-plus",
  [ViewType.CHAT]: "fas fa-message",
  [ViewType.START]: "fas fa-play",
  [ViewType.HISTORY]: "fas fa-clock-rotate-left",
  [ViewType.TIMELINE]: "fas fa-timeline",

  //
  // General
  //

  // containers (root)
  [ViewType.WINDOW]: "fas fa-window",
  [ViewType.TAB]: "fas fa-sidebar",
  [ViewType.SPLIT]: "fas fa-reflect-horizontal",
  [ViewType.SPLIT_DRAWER]: "fas fa-reflect-horizontal",
  // containers (layout)
  [ViewType.STACK]: "fas fa-layer-group",
  [ViewType.DRAWER]: "fas fa-square-minus",
  [ViewType.GRID]: "fas fa-table-cells-large",
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
  [ViewType.VALUE]: "fas fa-box-taped",
  // numeric
  [ViewType.NUMBER]: "fas fa-hashtag",
  [ViewType.SLIDER]: "fas fa-slider",
  // stringy
  [ViewType.STRING]: "fas fa-font-case",
  [ViewType.TEXT]: "fas fa-text",
  [ViewType.CODE]: "fas fa-code",
  [ViewType.JSON]: "fas fa-brackets-curly",
  // selection
  [ViewType.TOGGLE]: "fas fa-square-check",
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

export const ICON_BY_ALIGNMENT: Partial<Record<Alignment, IconData>> = _makeIcons({
  [Alignment.START]: "fas fa-align-left",
  [Alignment.MIDDLE]: "fas fa-objects-align-center-horizontal",
  [Alignment.END]: "fas fa-align-right",
  [Alignment.SPACE_BETWEEN]: "fas fa-distribute-spacing-horizontal",
});
export const ICON_BY_ANCHOR: Partial<Record<Anchor, IconData>> = _makeIcons({
  [Anchor.LEFT]: "fas fa-align-left",
  [Anchor.TOP]: "fas fa-align-top",
  [Anchor.RIGHT]: "fas fa-align-right",
  [Anchor.BOTTOM]: "fas fa-align-bottom",
});

export const ICON_BY_PRIMITIVE_TYPE: Partial<Record<PrimitiveType, IconData>> = _makeIcons({
  [PrimitiveType.BOOLEAN]: "fas fa-toggle-large-on",
  [PrimitiveType.INT16]: "fas fa-tally-4",
  [PrimitiveType.INT32]: "fas fa-tally-4",
  [PrimitiveType.INT64]: "fas fa-tally-4",
  [PrimitiveType.FLOAT32]: "fas fa-hashtag",
  [PrimitiveType.FLOAT64]: "fas fa-hashtag",
  [PrimitiveType.STRING]: "fas fa-font-case",
  [PrimitiveType.JSON]: "fas fa-brackets-curly",
  [PrimitiveType.BYTES]: "fas fa-binary",
  [PrimitiveType.UUID]: "fas fa-fingerprint",
  [PrimitiveType.DATETIME]: "fas fa-calendar",
  [PrimitiveType.INTERVAL]: "fas fa-calendar",
});

export const ICON_BY_FILE_TYPE: Partial<Record<FileType, IconData>> = _makeIcons({
  [FileType.TEXT]: "fas fa-file-lines",
  [FileType.IMAGE]: "fas fa-image",
  [FileType.AUDIO]: "fas fa-volume",
  [FileType.VIDEO]: "fas fa-video",
  [FileType.DOCUMENT]: "fas fa-file-invoice",
  [FileType.DATA]: "fas fa-database",
  [FileType.ARCHIVE]: "fas fa-file-zipper",
  [FileType.EXECUTABLE]: "fas fa-file-binary",
  [FileType.GENERIC]: "fas fa-file",
});

// NOTE :UX!: use vscode-icons for file format icons
export const ICON_BY_FILE_FORMAT: Partial<Record<FileFormat, IconData>> = _makeIcons({
  // text
  [FileFormat.TXT]: "fas fa-file-lines",
  [FileFormat.MARKDOWN]: "fas fa-pen-fancy",
  [FileFormat.RTF]: "fas fa-file-word",
  [FileFormat.LOG]: "fas fa-clipboard-list",
  // image
  [FileFormat.PDF]: "fas fa-file-pdf",
  [FileFormat.DOCX]: "fas fa-file-word",
  [FileFormat.PPTX]: "fas fa-file-powerpoint",
  [FileFormat.XLSX]: "fas fa-file-excel",
  [FileFormat.ODS]: "fas fa-table",
  [FileFormat.EPUB]: "fas fa-book",
  [FileFormat.CHM]: "fas fa-book",
  [FileFormat.DOC]: "fas fa-file-word",
  [FileFormat.XLS]: "fas fa-file-excel",
  [FileFormat.PPT]: "fas fa-file-powerpoint",
});

export const ICON_BY_FIELD_ZONE: Partial<Record<FieldZone, IconData>> = _makeIcons({
  [FieldZone.VARIABLE]: "fas fa-sliders",
  [FieldZone.MEMBER]: "fas fa-objects-column",
  [FieldZone.INPUT]: "fas fa-arrow-down-right",
  [FieldZone.OUTPUT]: "fas fa-arrow-up-right",
  [FieldZone.OPTION]: "fas fa-circle-small",
  [FieldZone.RUNTIME]: "fas fa-play",
});

export const ICON_BY_LOG_LEVEL: Record<LogLevel, IconData> = {
  [LogLevel.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.TRACE]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.DEBUG]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.INFO]: makeIcon({ faName: "fas fa-circle-check" }),
  [LogLevel.WARNING]: makeIcon({ faName: "fas fa-exclamation-triangle" }),
  [LogLevel.ERROR]: makeIcon({ faName: "fas fa-exclamation-circle" }),
  [LogLevel.CRITICAL]: makeIcon({ faName: "fas fa-skull" }),
};

export const ICON_BY_RUN_STATUS: Record<RunStatus, IconData> = {
  [RunStatus.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [RunStatus.SCHEDULED]: makeIcon({ faName: "fas fa-clock" }),
  [RunStatus.QUEUED]: makeIcon({ faName: "fas fa-hourglass" }),
  [RunStatus.RUNNING]: makeIcon({ faName: "fas fa-play" }),
  [RunStatus.PAUSED]: makeIcon({ faName: "fas fa-pause" }),
  [RunStatus.SUSPENDED]: makeIcon({ faName: "fas fa-circle-pause" }),
  [RunStatus.COMPLETED]: makeIcon({ faName: "fas fa-circle-check" }),
  [RunStatus.CANCELLED]: makeIcon({ faName: "fas fa-circle-xmark" }),
  [RunStatus.ABORTED]: makeIcon({ faName: "fas fa-skull" }),
  [RunStatus.FAILED]: makeIcon({ faName: "fas fa-exclamation-triangle" }),
};

export const ICON_BY_EDIT_TYPE: Partial<Record<EditType, IconData>> = _makeIcons<EditType>({
  [EditType.CREATE]: "fas fa-plus",
  [EditType.UPSERT]: "fas fa-plus",
  [EditType.UPDATE]: "fas fa-pen",
  [EditType.MOVE]: "fas fa-arrows-turn-right",
  [EditType.ARCHIVE]: "fas fa-box-archive",
  [EditType.UNARCHIVE]: "fas fa-box-archive",
  [EditType.DELETE]: "fas fa-trash",
  [EditType.RESTORE]: "fas fa-trash-undo",
});

export const ICON_BY_REGION_AREA: Partial<Record<RegionArea, IconData>> = _makeIcons<RegionArea>({
  [RegionArea.EUROPE]: "🇪🇺",
  [RegionArea.NORTH_AMERICA]: "🇺🇸",
  [RegionArea.SOUTH_AMERICA]: "fas fas fa-globe-americas",
  [RegionArea.MIDDLE_EAST]: "fas fas fa-globe-asia",
  [RegionArea.AFRICA]: "fas fas fa-globe-africa",
  [RegionArea.ASIA]: "fas fas fa-globe-asia",
  [RegionArea.AUSTRALIA]: "🇦🇺",
  [RegionArea.GLOBAL]: "fas fas fa-globe",
});

export const ICON_BY_REGION: Partial<Record<Region, IconData>> = _makeIcons<Region>({
  [Region.ZURICH]: "🇨🇭",
  [Region.FRANKFURT]: "🇩🇪",
  [Region.VIRGINIA]: "🇺🇸",
  [Region.OHIO]: "🇺🇸",
  [Region.OREGON]: "🇺🇸",
  [Region.SAO_PAULO]: "🇵🇸",
  [Region.CAPE_TOWN]: "🇿🇦",
  [Region.MUMBAI]: "🇮🇳",
  [Region.SINGAPORE]: "🇸🇬",
  [Region.TOKYO]: "🇯🇵",
  [Region.SYDNEY]: "🇦🇺",
});

export const ICON_BY_RESOURCE_STATUS: Partial<Record<ResourceStatus, IconData>> = _makeIcons<ResourceStatus>({
  [ResourceStatus.DECLARED]: "fas fa-circle-dot",
  [ResourceStatus.UP]: "fas fa-circle-check",
  [ResourceStatus.DOWN]: "fas fa-circle-xmark",
  [ResourceStatus.DEGRADED]: "fas fa-circle-exclamation",
  [ResourceStatus.SLEEPING]: "fas fa-zzz",
  [ResourceStatus.GONE]: "fas fa-empty-set",
});

// big registry of ICON_BY_* by enum type
export const ENUM_ICONS_BY_TYPE: Partial<Record<EnumType, Record<any, IconData>>> = {
  [EnumType.NODE_TYPE]: ICON_BY_NODE_TYPE,
  [EnumType.STRUCT_TYPE]: ICON_BY_STRUCT_TYPE,
  [EnumType.OBJECT_TYPE]: ICON_BY_OBJECT_TYPE,
  [EnumType.BENCH_TYPE]: ICON_BY_BENCH_TYPE,
  [EnumType.BLOCK_TYPE]: ICON_BY_BLOCK_TYPE,
  [EnumType.VIEW_TYPE]: ICON_BY_VIEW_TYPE,
  [EnumType.ALIGNMENT]: ICON_BY_ALIGNMENT,
  [EnumType.ANCHOR]: ICON_BY_ANCHOR,
  [EnumType.PRIMITIVE_TYPE]: ICON_BY_PRIMITIVE_TYPE,
  [EnumType.FILE_TYPE]: ICON_BY_FILE_TYPE,
  [EnumType.FILE_FORMAT]: ICON_BY_FILE_FORMAT,
  [EnumType.FIELD_ZONE]: ICON_BY_FIELD_ZONE,
  [EnumType.LOG_LEVEL]: ICON_BY_LOG_LEVEL,
  [EnumType.EDIT_TYPE]: ICON_BY_EDIT_TYPE,
  [EnumType.REGION_AREA]: ICON_BY_REGION_AREA,
  [EnumType.REGION]: ICON_BY_REGION,
  [EnumType.RESOURCE_STATUS]: ICON_BY_RESOURCE_STATUS,
};

export function getNodeIcon(
  node:
    | AnyNodeData
    | SomeNodeReferenceData
    | {
        metatype: ObjectType;
        type?: BlockType | ViewType | StepType;
        primitiveType?: PrimitiveType;
        benchType?: BenchType;
      },
) {
  if ((node as any).icon != null) {
    return (node as any).icon;
  } else if (isNode(node, NodeType.BLOCK)) {
    const icon = ICON_BY_BLOCK_TYPE[node.type];
    if (icon != null) return icon;
  } else if (isNode(node, NodeType.VIEW)) {
    const icon = ICON_BY_VIEW_TYPE[node.type];
    if (icon != null) return icon;
  } else if (isNode(node, NodeType.FIELD)) {
    if (node.zone == FieldZone.OPTION) {
      return ICON_BY_FIELD_ZONE[FieldZone.OPTION];
    } else if (node.primitiveType != null) {
      const icon = ICON_BY_PRIMITIVE_TYPE[node.primitiveType];
      if (icon != null) return icon;
    } else if (node.kind == TypeKind.BASED_NODE && node.benchType == BenchType.FIELD) {
      return ICON_BY_BLOCK_TYPE[BlockType.CHOICE];
    } else if (node.kind == TypeKind.OBJECT) {
      return ICON_BY_BLOCK_TYPE[BlockType.CLASS];
    } else if (node.benchType != null) {
      const icon = ICON_BY_BENCH_TYPE[node.benchType];
      if (icon != null) return icon;
    }
  } else if (isNode(node, NodeType.FILE) || isStruct(node, StructType.FILE_REFERENCE)) {
    if (ICON_BY_FILE_TYPE[node.coarseType] != null) return ICON_BY_FILE_TYPE[node.coarseType];
  }

  if (isNodeRef(node)) {
    return ICON_BY_NODE_TYPE[node.type] ?? DEFAULT_MISSING_ICON;
  } else {
    return ICON_BY_NODE_TYPE[node.metatype! as unknown as NodeType] ?? DEFAULT_MISSING_ICON;
  }
}

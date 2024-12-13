import {
  Alignment,
  Anchor,
  BenchType,
  BlockType,
  CacheMode,
  ChangeVignetteData,
  ColorData,
  ColorShade,
  ColorType,
  EditType,
  EnumType,
  ExpressionType,
  FieldData,
  FieldType,
  FileFormat,
  FileType,
  HubAspect,
  IconKind,
  InterruptType,
  LogLevel,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PipeType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  Region,
  RegionArea,
  ResourceStatus,
  RunStatus,
  NodeMode,
  StepType,
  StructType,
  TypeFormat,
  TypeKind,
  ViewType,
  type AnyNodeData,
  type IconData,
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";
// fa-icons is generated with:
// curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
//  | jq 'to_entries | map(select(.value.free | index("s@olid") or index("brands")) | {"id": .key, label: .value.label, unicode: .value.unicode, aliases: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
//  > fa-icons.json
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import type { TypeIdentity } from "@/language/field";
import { isNode } from "@/proto/wiring";
import { getColorHex, makeColor } from "@/ui/style";
import { IS_DEV, IS_DEVELOPER_MODE } from "@/utils/globals";

export type IconMetadata = {
  id: string;
  title: string;
  unicode: string;
  aliases?: string[];
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
    if (IS_DEV || IS_DEVELOPER_MODE.value) return <span class="text-danger-500">?icon: {JSON.stringify(props)}</span>;
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
export const DEFAULT_ENUM_ICON = makeIcon({ faName: "fas fa-caret-circle-down" });
export const DEFAULT_SYSTEM_ICON = makeIcon({ faName: "fas fa-gear" });

export const ICON_BY_NODE_TYPE: Partial<Record<NodeType, IconData>> = _makeIcons<NodeType>({
  // universe
  [NodeType.BENCH]: "fas fa-circle-dot",
  [NodeType.HANDLE]: "fas fa-at",
  [NodeType.USER]: "fas fa-user",
  [NodeType.ORGANIZATION]: "fas fa-building",
  [NodeType.CLIENT]: "fas fa-desktop",

  // auth
  [NodeType.MEMBERSHIP]: "fas fa-book-user",
  [NodeType.INVITE]: "fas fa-circle-nodes",

  // resource
  [NodeType.SERVER]: "fas fa-server",
  [NodeType.MACHINE]: "fas fa-computer-classic",
  [NodeType.STORE]: "fas fa-database",
  [NodeType.DRIVE]: "fas fa-hdd",
  [NodeType.VAULT]: "fas fa-treasure-chest",
  [NodeType.CACHE]: "fas fa-bolt-lightning",
  [NodeType.FILE]: "fas fa-file",
  [NodeType.SECRET]: "fas fa-key",
  [NodeType.BROWSER]: "fas fa-browser",

  // source
  [NodeType.BRANCH]: "fas fa-code-branch",
  [NodeType.PACKAGE]: "fas fa-box-open",
  [NodeType.DEPENDENCY]: "fas fa-turn-down-right",
  [NodeType.SPACE]: "fas fa-galaxy",
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.TRIGGER]: "fas fa-bolt-lightning",
  [NodeType.FIELD]: "fas fa-triangle",
  [NodeType.QUERY]: "fas fa-magnifying-glass",
  [NodeType.VIEW]: "fas fa-window-frame",
  [NodeType.STEP]: "fas fa-step-forward",
  [NodeType.PIPE]: "fas fa-pipe-section",
  [NodeType.BADGE]: "fas fa-id-badge",

  // state
  [NodeType.MESSAGE]: "fas fa-message",
  [NodeType.RECORD]: "fas fa-database",

  // runtime
  [NodeType.SESSION]: "fas fa-circle-play",
  [NodeType.RUN]: "fas fa-play",
  [NodeType.LOG]: "fas fa-file-alt",
});

export const ICON_BY_STRUCT_TYPE: Partial<Record<StructType, IconData>> = _makeIcons<StructType>({
  // core
  [StructType.PATH]: "fas fa-path",
  [StructType.TYPE_INFO]: "fas fa-tilde",
  [StructType.SCHEDULE]: "fas fa-calendar",
  // files
  [StructType.ICON]: "fas fa-icons",
  // code
  [StructType.CODE]: "fas fa-code",
  // expression
  [StructType.EXPRESSION]: "fas fa-sigma",
  [StructType.SELECTION]: "fas fa-list-check",
  [StructType.VALUE]: "fas fa-hexagon",
  // views
  [StructType.COLOR]: "fas fa-palette",
  [StructType.FONT]: "fas fa-font",
  [StructType.OFFSET]: "fas fa-arrows-alt",
  [StructType.RECTANGLE]: "fas fa-box",
  [StructType.VECTOR2]: "fas fa-vector-square",
  [StructType.VECTOR3]: "fas fa-vector-square",
  [StructType.VECTOR4]: "fas fa-vector-square",
  [StructType.LINE]: "fas fa-bezier-curve",
  // auth
  [StructType.POLICY]: "fas fa-shield-check",
  [StructType.POLICY_RULE]: "fas fa-shield-check",
  // text
  [StructType.TEXT]: "fas fa-align-left",
  [StructType.TEXT_LINE]: "fas fa-grip-lines",
  // run
  [StructType.RUN_OPTIONS]: "fas fa-play",
  [StructType.RUN_ATTEMPT]: "fas fa-play",
  [StructType.RUN_ERROR]: "fas fa-play",
  [StructType.RUN_FRAME]: "fas fa-play",
  [StructType.RUN_TRACE]: "fas fa-play",
  [StructType.BREAKPOINT]: "fas fa-pause",
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
  [BlockType.TEXT]: "fas fa-align-left",
  // class
  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.MESSAGE]: "fas fa-envelope",
  // runnable
  [BlockType.ACTION]: "fas fa-circle-play",
  [BlockType.FLOW]: "fas fa-diagram-project",
  // data
  [BlockType.VALUE]: "fas fa-hexagon",
  [BlockType.QUERY]: "fas fa-magnifying-glass",
  [BlockType.DATABASE]: "fas fa-database",
  // view
  [BlockType.VIEW]: "fas fa-window-frame",
  // auth
  [BlockType.ROLE]: "fas fa-user-tag",
  [BlockType.IDENTITY]: "fas fa-image-user",
});

export const ICON_BY_STEP_TYPE: Partial<Record<StepType, IconData>> = _makeIcons<StepType>({
  // boundary
  [StepType.START]: "fas fa-circle-play",
  [StepType.COMPLETE]: "fas fa-flag-checkered",
  [StepType.FAIL]: "fas fa-circle-exclamation",
  [StepType.TRIGGER]: "fas fa-bolt-lightning",
  // read
  [StepType.GET]: "fas fa-file-arrow-down",
  [StepType.SEARCH]: "fas fa-magnifying-glass",
  // write
  [StepType.CREATE]: "fas fa-plus",
  [StepType.CLONE]: "fas fa-clone",
  [StepType.UPDATE]: "fas fa-pencil",
  [StepType.DELETE]: "fas fa-trash",
  // run
  [StepType.ACTION]: "fas fa-code",
  [StepType.YIELD]: "fas fa-hand",
  // session
  [StepType.PAUSE]: "fas fa-pause",
  [StepType.RESUME]: "fas fa-play",
  [StepType.STOP]: "fas fa-stop",
  // control
  // ...
  // containers
  [StepType.LOOP]: "fas fa-repeat",
  // misc
  [StepType.TEXT]: "fas fa-align-left",
});

export const ICON_BY_PIPE_TYPE: Partial<Record<PipeType, IconData>> = _makeIcons<PipeType>({
  [PipeType.PASS]: "fas fa-arrow-right-long",
  [PipeType.SELECT]: "fas fa-question",
  [PipeType.OPTION]: "fas fa-circle-chevron-down",
  [PipeType.STREAM]: "fas fa-signal-stream",
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
  [ViewType.PAGE]: "fas fa-memo",
  [ViewType.BLOCK]: "fas fa-cube",
  [ViewType.FIELD]: "fas fa-font",
  [ViewType.DATABASE]: "fas fa-database",
  [ViewType.VIEW]: "fas fa-window-maximize",
  [ViewType.FLOW]: "fas fa-project-diagram",
  [ViewType.STEP]: "fas fa-shoe-prints",
  [ViewType.TYPE]: "fas fa-objects-column",
  [ViewType.OBJECT]: "fas fa-cubes",
  [ViewType.PIPE]: "fas fa-tachometer-alt",

  // helpers
  [ViewType.TREE]: "fas fa-list-tree",
  [ViewType.DETAIL]: "fas fa-eye",
  [ViewType.CREATE]: "fas fa-plus",
  [ViewType.CHAT]: "fas fa-message",
  [ViewType.RUN]: "fas fa-play",
  [ViewType.FEED]: "fas fa-list-timeline",
  [ViewType.TIMELINE]: "fas fa-timeline",

  //
  // Organization
  //

  // layout
  [ViewType.WINDOW]: "fas fa-window",
  [ViewType.TAB]: "fas fa-sidebar",
  [ViewType.SPLIT]: "fas fa-reflect-horizontal",
  [ViewType.SPLIT_DRAWER]: "fas fa-reflect-horizontal",
  [ViewType.STACK]: "fas fa-layer-group",
  [ViewType.DRAWER]: "fas fa-square-minus",
  [ViewType.SCROLL]: "fas fa-arrows-alt-v",
  [ViewType.GRID]: "fas fa-table-cells-large",

  // groups
  [ViewType.GROUP]: "fas fa-object-group",
  [ViewType.SECTION]: "fas fa-xmark-lines",
  [ViewType.FORM]: "fas fa-clipboard-list",

  // presentation
  [ViewType.SPACER]: "fas fa-square-dashed",
  [ViewType.DIVIDER]: "fas fa-horizontal-rule",

  // collections
  [ViewType.LIST]: "fas fa-list",
  [ViewType.TABLE]: "fas fa-table",
  [ViewType.GALLERY]: "fas fa-th-large",
  [ViewType.BOARD]: "fas fa-columns",

  //
  // Style
  //

  // navigation
  [ViewType.BREADCRUMB]: "fas fa-ellipsis-h",
  [ViewType.PROGRESS]: "fas fa-spinner",
  [ViewType.AVATAR]: "fas fa-user-circle",
  [ViewType.BADGE]: "fas fa-badge",

  // illustration
  [ViewType.SHAPE]: "fas fa-shapes",

  // graphing
  [ViewType.CHART]: "fas fa-chart-pie",

  //
  // Action
  //

  // controls
  [ViewType.BUTTON]: "fas fa-hand-pointer",
  [ViewType.MULTI_BUTTON]: "fas fa-hand-pointer",
  [ViewType.LINK]: "fas fa-link",

  //
  // Content
  //

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

  // rich
  [ViewType.COLOR]: "fas fa-palette",
  [ViewType.ICON]: "fas fa-icons",

  // file
  [ViewType.FILE]: "fas fa-file",
  [ViewType.IMAGE]: "fas fa-image",
  [ViewType.AUDIO]: "fas fa-volume",
  [ViewType.VIDEO]: "fas fa-video",
  [ViewType.DOCUMENT]: "fas fa-file-alt",
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
export const ICON_BY_HUB_ASPECT: Partial<Record<HubAspect, IconData>> = _makeIcons({
  [HubAspect.BENCH]: "fas fa-code",
  [HubAspect.ACTIVITY]: "fas fa-wave-pulse",
  [HubAspect.CATALOG]: "fas fa-globe",
  [HubAspect.LIBRARY]: "fas fa-shapes",
});

export const ICON_BY_TYPE_KIND: Partial<Record<TypeKind, IconData>> = _makeIcons({
  [TypeKind.PRIMITIVE]: "fas fa-circle-dot",
  [TypeKind.STRUCT]: "fas fa-circle-dot",
  [TypeKind.NODE]: "fas fa-circle-nodes",
  [TypeKind.BASED_NODE]: "fas fa-circle-nodes",
  [TypeKind.ENUM]: "fas fa-list-ul",
  [TypeKind.CUSTOM_OBJECT]: "fas fa-shapes",
  [TypeKind.LITERAL]: "fas fa-circle-dot",
  [TypeKind.UNION]: "fas fa-circle-dot",
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
  [PrimitiveType.DATE]: "fas fa-calendar",
  [PrimitiveType.TIME]: "fas fa-clock",
  [PrimitiveType.INTERVAL]: "fas fa-stopwatch",
});

export const ICON_BY_TYPE_FORMAT: Partial<Record<TypeFormat, IconData>> = _makeIcons({
  [TypeFormat.URL]: "fas fa-link",
  [TypeFormat.EMAIL]: "fas fa-at",
  [TypeFormat.EMOJI]: "fas fa-smile",
  [TypeFormat.PHONE_NUMBER]: "fas fa-phone",
  [TypeFormat.SLUG]: "fas fa-hashtag",
});

export const ICON_BY_FILE_TYPE: Partial<Record<FileType, IconData>> = _makeIcons({
  [FileType.TEXT]: "fas fa-file-lines",
  [FileType.CODE]: "fas fa-file-code",
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

export const ICON_BY_FIELD_TYPE: Partial<Record<FieldType, IconData>> = _makeIcons({
  [FieldType.VARIABLE]: "fas fa-sliders",
  [FieldType.MEMBER]: "fas fa-objects-column",
  [FieldType.INPUT]: "fas fa-arrow-down-right",
  [FieldType.OUTPUT]: "fas fa-arrow-up-right",
  [FieldType.OPTION]: "fas fa-circle-small",
});

export const ICON_BY_LOG_LEVEL: Record<LogLevel, IconData> = {
  [LogLevel.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.TRACE]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.DEBUG]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.INFO]: makeIcon({ faName: "fas fa-circle-check" }),
  [LogLevel.WARNING]: makeIcon({ faName: "fas fa-circle-exclamation" }),
  [LogLevel.ERROR]: makeIcon({ faName: "fas fa-circle-exclamation" }),
  [LogLevel.CRITICAL]: makeIcon({ faName: "fas fa-skull" }),
};

export const ICON_BY_RUN_STATUS: Record<RunStatus, IconData> = {
  [RunStatus.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [RunStatus.SCHEDULED]: makeIcon({ faName: "fas fa-clock" }),
  [RunStatus.QUEUED]: makeIcon({ faName: "fas fa-hourglass" }),
  [RunStatus.RUNNING]: makeIcon({ faName: "fas fa-circle-notch" }),
  [RunStatus.PAUSED]: makeIcon({ faName: "fas fa-pause" }),
  [RunStatus.YIELDED]: makeIcon({ faName: "fas fa-hand" }),
  [RunStatus.WAITING]: makeIcon({ faName: "fas fa-bolt-lightning" }),
  [RunStatus.COMPLETED]: makeIcon({ faName: "fas fa-check" }),
  [RunStatus.CANCELLED]: makeIcon({ faName: "fas fa-circle-xmark" }),
  [RunStatus.ABORTED]: makeIcon({ faName: "fas fa-skull" }),
  [RunStatus.FAILED]: makeIcon({ faName: "fas fa-circle-exclamation" }),
};

export const ICON_BY_INTERRUPT_TYPE: Record<InterruptType, IconData> = {
  [InterruptType.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [InterruptType.YIELD]: makeIcon({ faName: "fas fa-hand" }),
  [InterruptType.PAUSE]: makeIcon({ faName: "fas fa-pause" }),
  [InterruptType.WAIT]: makeIcon({ faName: "fas fa-bolt-lightning" }),
};

export const ICON_BY_EDIT_TYPE: Partial<Record<EditType, IconData>> = _makeIcons<EditType>({
  [EditType.CREATE]: "fas fa-plus",
  [EditType.UPSERT]: "fas fa-plus",
  [EditType.UPDATE]: "fas fa-pen",
  [EditType.MOVE]: "fas fa-arrows-turn-right",
  [EditType.DELETE]: "fas fa-trash",
  [EditType.RESTORE]: "fas fa-trash-undo",
});

export const ICON_BY_EXPRESSION_OP: Partial<Record<ExpressionType, IconData>> = _makeIcons<ExpressionType>({
  [ExpressionType.NOT]: "fas fa-exclamation",
  [ExpressionType.AND]: "fas fa-ampersand",
  [ExpressionType.OR]: "fas fa-pipe",
  [ExpressionType.EQUALS]: "fas fa-equals",
  [ExpressionType.NOT_EQUALS]: "fas fa-not-equals",
  [ExpressionType.GREATER_THAN]: "fas fa-greater-than",
  [ExpressionType.GREATER_THAN_OR_EQUALS]: "fas fa-greater-than-equal",
  [ExpressionType.LESS_THAN]: "fas fa-less-than",
  [ExpressionType.LESS_THAN_OR_EQUALS]: "fas fa-less-than-equal",
  [ExpressionType.MATCHES_REGEX]: "fas fa-regex",
  [ExpressionType.STARTS_WITH]: "fas fa-arrow-right-from-line",
  [ExpressionType.ENDS_WITH]: "fas fa-arrow-left-from-line",
  [ExpressionType.ASCENDING]: "fas fa-arrow-up",
  [ExpressionType.DESCENDING]: "fas fa-arrow-down",
});

export const ICON_BY_NODE_MODE: Partial<Record<NodeMode, IconData>> = _makeIcons<NodeMode>({
  [NodeMode.PRODUCTION]: "fas fa-globe",
  [NodeMode.DEVELOPMENT]: "fas fa-bug",
  [NodeMode.TEST]: "fas fa-flask",
  [NodeMode.PREVIEW]: "fas fa-eye",
  [NodeMode.BUILTIN]: "fas fa-cog",
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
  [ResourceStatus.DECOMMISSIONED]: "fas fa-circle-slash",
});

export const ICON_BY_CACHE_MODE: Partial<Record<CacheMode, IconData>> = _makeIcons<CacheMode>({
  [CacheMode.NEVER]: "fas fa-bolt-slash",
  [CacheMode.ALWAYS]: "fas fa-bolt-lightning",
});

// big registry of ICON_BY_* by enum type
export const ICONS_BY_ENUM_TYPE: Partial<Record<EnumType, Record<any, IconData>>> = {
  [EnumType.NODE_TYPE]: ICON_BY_NODE_TYPE,
  [EnumType.STRUCT_TYPE]: ICON_BY_STRUCT_TYPE,
  [EnumType.OBJECT_TYPE]: ICON_BY_OBJECT_TYPE,
  [EnumType.BENCH_TYPE]: ICON_BY_BENCH_TYPE,
  [EnumType.BLOCK_TYPE]: ICON_BY_BLOCK_TYPE,
  [EnumType.STEP_TYPE]: ICON_BY_STEP_TYPE,
  [EnumType.PIPE_TYPE]: ICON_BY_PIPE_TYPE,
  [EnumType.VIEW_TYPE]: ICON_BY_VIEW_TYPE,
  [EnumType.ALIGNMENT]: ICON_BY_ALIGNMENT,
  [EnumType.ANCHOR]: ICON_BY_ANCHOR,
  [EnumType.PRIMITIVE_TYPE]: ICON_BY_PRIMITIVE_TYPE,
  [EnumType.TYPE_KIND]: ICON_BY_TYPE_KIND,
  [EnumType.TYPE_FORMAT]: ICON_BY_TYPE_FORMAT,
  [EnumType.FILE_TYPE]: ICON_BY_FILE_TYPE,
  [EnumType.FILE_FORMAT]: ICON_BY_FILE_FORMAT,
  [EnumType.FIELD_ZONE]: ICON_BY_FIELD_TYPE,
  [EnumType.LOG_LEVEL]: ICON_BY_LOG_LEVEL,
  [EnumType.EDIT_TYPE]: ICON_BY_EDIT_TYPE,
  [EnumType.NODE_MODE]: ICON_BY_NODE_MODE,
  [EnumType.REGION_AREA]: ICON_BY_REGION_AREA,
  [EnumType.REGION]: ICON_BY_REGION,
  [EnumType.RESOURCE_STATUS]: ICON_BY_RESOURCE_STATUS,
  [EnumType.CACHE_MODE]: ICON_BY_CACHE_MODE,
};

/** Resolves the icon for a type :FieldIcon */
export function getTypeIcon(node: Partial<FieldData> | TypeIdentity): IconData | undefined {
  if ((node as FieldData).type == FieldType.OPTION) {
    return ICON_BY_FIELD_TYPE[FieldType.OPTION];
  } else if (node.primitiveType != null) {
    if (node.format != null) {
      const icon = ICON_BY_TYPE_FORMAT[node.format];
      if (icon != null) return icon;
    }
    const icon = ICON_BY_PRIMITIVE_TYPE[node.primitiveType];
    if (icon != null) return icon;
  } else if (node.kind == TypeKind.BASED_NODE && node.benchType == BenchType.FIELD) {
    return ICON_BY_BLOCK_TYPE[BlockType.CHOICE];
  } else if (node.benchType != null) {
    const icon = ICON_BY_BENCH_TYPE[node.benchType];
    if (icon != null) return icon;
  } else if (node.kind == TypeKind.NODE) {
    return ICON_BY_TYPE_KIND[node.kind];
  }
  return undefined;
}

/** Gets the icon for a node 'subtype' (enum property) with the given name/value.  */
function getNodeSubtypeIcon(nodeType: NodeType, subtype: any): IconData | undefined {
  const allProperties = PROPERTY_ENUM_BY_TYPE[nodeType]!;
  const prop = PROPERTY_INFOS_BY_TYPE[nodeType][allProperties["type" as any]];
  if (prop?.enumType != null) {
    const enumIcons = ICONS_BY_ENUM_TYPE[prop.enumType];
    if (enumIcons?.[subtype] != null) {
      return enumIcons[subtype];
    }
  }
  return undefined;
}

/** Gets the icon for a node or node reference. */
export function getNodeIcon(
  node: ({ metatype: NodeType | ObjectType } & Partial<AnyNodeData | NodeReferenceData>) | ChangeVignetteData,
  options?: { nodeType?: NodeType; defaultToUndefined?: boolean },
): IconData | undefined {
  if ((node as any).icon != null) {
    // already has specific icon
    return (node as any).icon;
  } else if (isNode(node, NodeType.FIELD)) {
    // more specific icons for fields
    const icon = getTypeIcon(node);
    if (icon != null) return icon;
  }

  // get icon for subtype
  const nodeType =
    options?.nodeType ?? (isNode(node) ? (node.metatype as unknown as NodeType) : (node as NodeReferenceData).nodeType);
  const nodeSubtype = (node as ChangeVignetteData).subtype ?? (node as any).type;
  if (nodeSubtype != null) {
    const nodeSubtypeIcon = getNodeSubtypeIcon(nodeType, nodeSubtype);
    if (nodeSubtypeIcon != null) {
      return nodeSubtypeIcon;
    }
  }

  // generic icon for node type
  if (ICON_BY_NODE_TYPE[nodeType] != null) {
    return ICON_BY_NODE_TYPE[nodeType];
  }

  // missing icon
  if (options?.defaultToUndefined) {
    return undefined;
  } else {
    return DEFAULT_MISSING_ICON;
  }
}

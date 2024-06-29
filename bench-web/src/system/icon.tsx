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
  Visibility,
  FormatHint,
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
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";
// file is generated with:
// curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
//  | jq 'to_entries | map(select(.value.free | index("solid") or index("brands")) | {"id": .key, label: .value.label, unicode: .value.unicode, alias: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
//  > fa-icons.json
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import { IS_DEV, isDeveloperMode } from "@/utils/globals";
import { getColorHex, makeColor } from "@/utils/style";

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

type IconInlineProps = Pick<IconData, "emoji" | "file" | "faName"> & {
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
    if (IS_DEV || isDeveloperMode.value) return <span class="text-danger-500">?invalid: {JSON.stringify(props)}</span>;
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
type IconIn = string | (Pick<IconData, "emoji" | "file" | "faName"> & { color?: ColorIn });
export function makeIcon(icon: IconIn): IconData {
  let kind: IconKind;
  if (typeof icon == "string") {
    return { metatype: ObjectType.ICON, kind: IconKind.FONT_AWESOME, faName: icon };
  } else if (icon.emoji) {
    kind = IconKind.EMOJI;
  } else if (icon.file) {
    kind = IconKind.FILE;
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
export const DEFAULT_BENCH_ICON = makeIcon({ faName: "fas fa-circle-dot" });
export const DEFAULT_ENUM_ICON = makeIcon({ faName: "fas fa-caret-circle-down" });

export const ICON_BY_NODE_TYPE: Partial<Record<NodeType, IconData>> = _makeIcons<NodeType>({
  // root
  [NodeType.BENCH]: "fas fa-circle-dot",
  [NodeType.ENVIRONMENT]: "fas fa-globe",
  [NodeType.BRANCH]: "fas fa-code-branch",

  // source
  [NodeType.PACKAGE]: "fas fa-box-open",
  [NodeType.DEPENDENCY]: "fas fa-turn-down-right",
  [NodeType.SPACE]: "fas fa-galaxy",
  [NodeType.LINK]: "fas fa-link",
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.ISSUE]: "fas fa-square-exclamation",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.TRIGGER]: "fas fa-bolt",
  [NodeType.FIELD]: "fas fa-font",
  [NodeType.QUERY]: "fas fa-magnifying-glass",
  [NodeType.VIEW]: "fas fa-browser",
  [NodeType.STEP]: "fas fa-step-forward",

  // auth
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

  // resources
  [NodeType.SERVER]: "fas fa-server",
  [NodeType.MACHINE]: "fas fa-desktop",
  [NodeType.STORE]: "fas fa-database",
  [NodeType.DRIVE]: "fas fa-hdd",
  [NodeType.BLOB]: "fas fa-file",

  // user
  [NodeType.HANDLE]: "fas fa-at",
  [NodeType.USER]: "fas fa-user",
  [NodeType.ORGANIZATION]: "fas fa-building",
  [NodeType.CLIENT]: "fas fa-desktop",
});

export const ICON_BY_STRUCT_TYPE: Partial<Record<StructType, IconData>> = _makeIcons<StructType>({
  // core
  [StructType.PATH]: "fas fa-path",
  [StructType.TYPE_INFO]: "fas fa-tilde",
  [StructType.CONTEXT]: "fas fa-ellipsis-h",
  [StructType.SCHEDULE]: "fas fa-calendar",
  [StructType.PROJECTION]: "fas fa-project-diagram",
  // files
  [StructType.FILE]: "fas fa-file",
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
  // flow
  [StructType.STEP_CONNECTION]: "fas fa-arrow-right",
  // text
  [StructType.TEXT]: "fas fa-text",
  [StructType.TEXT_LINE]: "fas fa-grip-lines",
});

export const ICON_BY_OBJECT_TYPE: Partial<Record<ObjectType, IconData>> = {
  ...ICON_BY_NODE_TYPE,
  ...ICON_BY_STRUCT_TYPE,
};

export const ICON_BY_BENCH_TYPE: Partial<Record<BenchType, IconData>> = {
  ...ICON_BY_OBJECT_TYPE,
};

export const ICON_BY_BLOCK_TYPE: Partial<Record<BlockType, IconData>> = _makeIcons<BlockType>({
  [BlockType.MODULE]: "fas fa-box-open",
  [BlockType.PAGE]: "fas fa-memo",
  [BlockType.BLANK]: "fas fa-empty-set",
  [BlockType.ALIAS]: "fas fa-link",
  [BlockType.CLASS]: "fas fa-objects-column",

  [BlockType.CHOICE]: "fas fa-circle-chevron-down",
  [BlockType.SIGNAL]: "fas fa-signal-stream",
  [BlockType.PROTOCOL]: "fas fa-list-check",

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
  [ViewType.EXPLORE]: "fas fa-compass",
  [ViewType.OUTLINE]: "fas fa-list-tree",
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

export const ICON_BY_VISIBILITY: Partial<Record<Visibility, IconData>> = _makeIcons({
  [Visibility.PAGE]: "fas fa-memo-pad",
  [Visibility.MODULE]: "fas fa-box-open",
  [Visibility.BENCH]: "fas fa-circle-dot",
  [Visibility.PUBLIC]: "fas fa-globe",
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

export const ICON_BY_FORMAT_HINT: Partial<Record<FormatHint, IconData>> = _makeIcons({
  // string
  [FormatHint.TITLE]: "fas fa-heading",
  [FormatHint.EMAIL]: "fas fa-at",
  [FormatHint.URL]: "fas fa-link",
  [FormatHint.MARKDOWN]: "fas fa-m",
  [FormatHint.CODE]: "fas fa-code",
  [FormatHint.EMOJI]: "fas fa-smile",
  // number
  [FormatHint.PHONE]: "fas fa-phone",
  [FormatHint.RATING]: "fas fa-star",
  [FormatHint.SLIDER]: "fas fa-slider",
  // files
  [FormatHint.IMAGE]: "fas fa-image",
  [FormatHint.VIDEO]: "fas fa-video",
  [FormatHint.AUDIO]: "fas fa-volume",
});

export const ICON_BY_FIELD_ZONE: Partial<Record<FieldZone, IconData>> = _makeIcons({
  [FieldZone.VARIABLE]: "fas fa-sliders",
  [FieldZone.MEMBER]: "fas fa-objects-column",
  [FieldZone.INPUT]: "fas fa-arrow-down-right",
  [FieldZone.OUTPUT]: "fas fa-arrow-up-right",
  [FieldZone.OPTION]: "fas fa-circle-small",
});

export const ICON_BY_LEVEL: Record<LogLevel, IconData> = {
  [LogLevel.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.TRACE]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.DEBUG]: makeIcon({ faName: "fas fa-bug" }),
  [LogLevel.INFO]: makeIcon({ faName: "fas fa-circle-check" }),
  [LogLevel.WARNING]: makeIcon({ faName: "fas fa-exclamation-triangle" }),
  [LogLevel.ERROR]: makeIcon({ faName: "fas fa-exclamation-circle" }),
  [LogLevel.CRITICAL]: makeIcon({ faName: "fas fa-skull" }),
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

export const ENUM_ICONS_BY_TYPE: Partial<Record<EnumType, Record<any, IconData>>> = {
  [EnumType.NODE_TYPE]: ICON_BY_NODE_TYPE,
  [EnumType.STRUCT_TYPE]: ICON_BY_STRUCT_TYPE,
  [EnumType.OBJECT_TYPE]: ICON_BY_OBJECT_TYPE,
  [EnumType.BENCH_TYPE]: ICON_BY_BENCH_TYPE,
  [EnumType.BLOCK_TYPE]: ICON_BY_BLOCK_TYPE,
  [EnumType.VIEW_TYPE]: ICON_BY_VIEW_TYPE,
  [EnumType.ALIGNMENT]: ICON_BY_ALIGNMENT,
  [EnumType.ANCHOR]: ICON_BY_ANCHOR,
  [EnumType.VISIBILITY]: ICON_BY_VISIBILITY,
  [EnumType.PRIMITIVE_TYPE]: ICON_BY_PRIMITIVE_TYPE,
  [EnumType.FORMAT_HINT]: ICON_BY_FORMAT_HINT,
  [EnumType.FIELD_ZONE]: ICON_BY_FIELD_ZONE,
  [EnumType.LOG_LEVEL]: ICON_BY_LEVEL,
  [EnumType.EDIT_TYPE]: ICON_BY_EDIT_TYPE,
};

export function getNodeIcon(
  node:
    | AnyNodeData
    | {
        metatype: ObjectType;
        type?: BlockType | ViewType | StepType;
        primitiveType?: PrimitiveType;
        benchType?: BenchType;
      },
) {
  if ((node as any).icon != null) {
    return (node as any).icon;
  } else if (node.metatype == ObjectType.BLOCK) {
    const icon = ICON_BY_BLOCK_TYPE[(node as BlockData).type! as BlockType];
    if (icon != null) return icon;
  } else if (node.metatype == ObjectType.VIEW) {
    const icon = ICON_BY_VIEW_TYPE[(node as ViewData).type! as ViewType];
    if (icon != null) return icon;
  } else if (node.metatype == ObjectType.FIELD) {
    if ((node as FieldData).zone == FieldZone.OPTION) {
      return ICON_BY_FIELD_ZONE[FieldZone.OPTION];
    } else if ("primitiveType" in node) {
      const icon = ICON_BY_PRIMITIVE_TYPE[node.primitiveType!];
      if (icon != null) return icon;
    } else if ("benchType" in node) {
      const icon = ICON_BY_BENCH_TYPE[node.benchType!];
      if (icon != null) return icon;
    }
  }
  return ICON_BY_NODE_TYPE[node.metatype! as unknown as NodeType] ?? DEFAULT_MISSING_ICON;
}

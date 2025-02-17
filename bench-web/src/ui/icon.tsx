import {
  ActionType,
  Alignment,
  Anchor,
  BenchType,
  BlockType,
  CacheMode,
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
  IconType,
  InterruptionType,
  Severity,
  NodeMode,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PipeType,
  PrimitiveType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  Region,
  RegionContinent,
  ResourceStatus,
  RunSpanType,
  RunStatus,
  StructType,
  TypeFormat,
  TypeKind,
  ViewType,
  type AnyNodeData,
  type IconData,
  PipeTrigger,
  ActionCategory,
  ToolFilter,
  TextLineType,
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import _AVAILABLE_EMOJI_ICONS from "@/assets/emoji-icons.json";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/language/core/const";
import type { TypeIdentity } from "@/language/core/type";
import { unpackSubnodeProperty } from "@/language/core/node";
import { unpackPartialNode } from "@/language/core/value";
import { isNode } from "@/proto/wiring";
import { supergraph } from "@/globals";
import { getColorHex, makeColor } from "@/ui/style";
import { IS_DEV, IS_DEVELOPER_MODE } from "@/utils/globals";

/**
 * FontAwesome icons
 *
 * fa-icons is generated with:
 * curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
 * | jq 'to_entries | map(select(.value.free | index("solid") or index("brands")) | {"id": .key, label: .value.label, unicode: .value.unicode, aliases: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
 * > src/assets/fa-icons.json
 */

export type FontAwesomeIcon = {
  id: string;
  title: string;
  unicode: string;
  aliases?: string[];
  family: "fas" | "fab";
  faName: string;
};
export function fontAwesomeIcon(data: FontAwesomeIcon, color?: ColorData | undefined): IconData {
  return {
    metatype: ObjectType.ICON,
    type: IconType.FONT_AWESOME,
    faName: `${data.family} fa-${data.id}`,
    color,
  };
}
export const AVAILABLE_FA_ICONS: FontAwesomeIcon[] = _AVAILABLE_FA_ICONS.map((i) => ({
  ...i,
  faName: `${i.family} fa-${i.id}`,
})) as FontAwesomeIcon[];

/**
 * Emoji icons
 * curl https://raw.githubusercontent.com/muan/unicode-emoji-json/main/data-by-emoji.json \
 * | jq 'to_entries | map({ id: .key, title: .value.name, emoji: .key })' \
 * > src/assets/emoji-icons.json
 */

export type EmojiIcon = {
  id: string;
  title: string;
  emoji: string;
  aliases?: string[];
};
export function emojiIcon(data: EmojiIcon, color?: ColorData | undefined): IconData {
  return {
    metatype: ObjectType.ICON,
    type: IconType.EMOJI,
    emoji: data.emoji,
    color,
  };
}
export const AVAILABLE_EMOJI_ICONS: EmojiIcon[] = _AVAILABLE_EMOJI_ICONS;

export const AVAILABLE_ICONS_BY_ID: Record<string, FontAwesomeIcon | EmojiIcon> = Object.fromEntries(
  [...AVAILABLE_FA_ICONS, ...AVAILABLE_EMOJI_ICONS].map((i) => [i.id, i]),
);
type IconInlineProps = Pick<IconData, "emoji" | "faName" | "vscName"> & {
  color?: ColorType | ColorData;
  shade?: ColorShade;
  forceColor?: "inherit" | ColorType;
  fallbackColor?: ColorType;
};

/** Inline functional Icon component */
export const IconInline: FunctionalComponent<IconInlineProps> = (props) => {
  let colorHex;
  if (props.forceColor == "inherit") {
    colorHex = undefined;
  } else if (props.color != null) {
    colorHex = getColorHex(props.color, props.shade);
  } else if (props.forceColor != null) {
    colorHex = getColorHex(props.forceColor, props.shade);
  } else if (props.fallbackColor != null) {
    colorHex = getColorHex(props.fallbackColor, props.shade);
  } else {
    colorHex = undefined;
  }
  if (props.faName) {
    // font awesome
    return <i class={`${props.faName} text-center`} style={{ color: colorHex }} />;
  } else if (props.emoji) {
    // emoji
    return <span style={{ color: colorHex }}>{props.emoji}</span>;
  } else {
    // invalid icon
    if (IS_DEV || IS_DEVELOPER_MODE.value) return <span class="text-danger-500">?icon: {JSON.stringify(props)}</span>;
    else return <span style={{ color: colorHex }}>???</span>;
  }
};
IconInline.props = ["emoji", "faName", "vscName", "color", "fallbackColor", "shade", "ignoreColor"];

/** Inline functional Avatar component */
export const AvatarInline: FunctionalComponent<
  IconInlineProps & { size?: "regular" | "medium" | "large" | "title" }
> = (props) => {
  let backgroundColorHex;
  if (props.forceColor == "inherit") {
    backgroundColorHex = undefined;
  } else if (props.color != null) {
    backgroundColorHex = getColorHex(props.color, props.shade);
  } else if (props.forceColor != null) {
    backgroundColorHex = getColorHex(props.forceColor, props.shade);
  } else if (props.fallbackColor != null) {
    backgroundColorHex = getColorHex(props.fallbackColor, props.shade);
  } else {
    backgroundColorHex = undefined;
  }

  let sizeClasses;
  if (props.size === "medium") {
    if (props.faName) {
      sizeClasses = "w-9 h-9 text-lg";
    } else {
      sizeClasses = "w-9 h-9 text-xl";
    }
  } else if (props.size === "large") {
    if (props.faName) {
      sizeClasses = "w-10 h-10 text-lg";
    } else {
      sizeClasses = "w-10 h-10 text-3xl";
    }
  } else if (props.size === "title") {
    if (props.faName) {
      sizeClasses = "w-12 h-12 text-xl";
    } else {
      sizeClasses = "w-12 h-12 text-4xl";
    }
  } else {
    // regular
    if (props.faName) {
      sizeClasses = "w-6 h-6 text-sm";
    } else {
      sizeClasses = "w-6 h-6 text-md";
    }
  }
  let baseClasses = `rounded-full text-center inline-flex items-center justify-center ${sizeClasses}`;
  if (backgroundColorHex == null) {
    baseClasses += " bg-gray-100 border border-gray-200";
  }

  if (props.faName) {
    // font awesome
    return (
      <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
        <i class={`${props.faName} text-white`} />
      </div>
    );
  } else if (props.emoji) {
    // emoji
    return (
      <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
        {props.emoji}
      </div>
    );
  } else {
    // invalid icon
    if (IS_DEV || IS_DEVELOPER_MODE.value) {
      return <span class="text-danger-500">?icon: {JSON.stringify(props)}</span>;
    } else {
      return (
        <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
          <span class="text-white">???</span>
        </div>
      );
    }
  }
};
AvatarInline.props = ["emoji", "faName", "vscName", "color", "fallbackColor", "shade", "ignoreColor", "size"];

export function getIconMetadata(icon: IconData): FontAwesomeIcon | EmojiIcon | undefined {
  if (icon.type == IconType.FONT_AWESOME) {
    const id = icon.faName!.split(" ")[1].slice(3);
    return AVAILABLE_ICONS_BY_ID[id];
  } else if (icon.type == IconType.EMOJI) {
    return AVAILABLE_ICONS_BY_ID[icon.emoji!];
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
  let type: IconType;
  if (typeof icon == "string") {
    if (icon.startsWith("fa")) {
      return { metatype: ObjectType.ICON, type: IconType.FONT_AWESOME, faName: icon };
    } else {
      return { metatype: ObjectType.ICON, type: IconType.EMOJI, emoji: icon };
    }
  } else if (icon.emoji) {
    type = IconType.EMOJI;
  } else if (icon.faName) {
    type = IconType.FONT_AWESOME;
  } else {
    throw new Error(`unexpected icon ${icon}`);
  }
  const color = icon.color != null && typeof icon.color != "object" ? makeColor(icon.color) : icon.color;
  return { metatype: ObjectType.ICON, type, ...icon, color };
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
  [NodeType.BENCH]: "fas fa-volleyball",
  [NodeType.HANDLE]: "fas fa-at",
  [NodeType.USER]: "fas fa-user",
  [NodeType.ORGANIZATION]: "fas fa-building",
  [NodeType.CLIENT]: "fas fa-desktop",

  // auth
  [NodeType.MEMBERSHIP]: "fas fa-book-user",
  [NodeType.INVITE]: "fas fa-circle-nodes",

  // resource
  [NodeType.SCALER]: "fas fa-scale-unbalanced",
  [NodeType.MACHINE]: "fas fa-computer-classic",
  [NodeType.STORE]: "fas fa-database",
  [NodeType.FILE]: "fas fa-file",
  [NodeType.SECRET]: "fas fa-key",
  [NodeType.BROWSER]: "fas fa-globe",

  // source
  [NodeType.PACKAGE]: "fas fa-box-open",
  [NodeType.DEPENDENCY]: "fas fa-turn-down-right",
  [NodeType.PAGE]: "far fa-file",
  [NodeType.BLOCK]: "fas fa-cube",
  [NodeType.CHOICE]: "fas fa-circle-chevron-down",
  [NodeType.CLASS]: "fas fa-shapes",
  [NodeType.FIELD]: "fas fa-triangle",
  [NodeType.FLOW]: "fas fa-diagram-project",
  [NodeType.ACTION]: "fas fa-step-forward",
  [NodeType.PIPE]: "fas fa-pipe-section",
  [NodeType.TRIGGER]: "fas fa-bolt",
  [NodeType.VIEW]: "fas fa-window-frame",
  [NodeType.DATABASE]: "fas fa-database",
  [NodeType.CHANNEL]: "fas fa-hashtag",
  [NodeType.ROLE]: "fas fa-user-tag",
  [NodeType.IDENTITY]: "fas fa-snowman-head",

  // state
  [NodeType.THREAD]: "fas fa-reel",
  [NodeType.MESSAGE]: "fas fa-message",
  [NodeType.RECORD]: "fas fa-database",

  // runtime
  [NodeType.SESSION]: "fas fa-circle-play",
  [NodeType.RUN]: "fas fa-play",
  [NodeType.RUN_SPAN]: "fas fa-play",
  [NodeType.LOG]: "fas fa-file-alt",
  [NodeType.INTERRUPTION]: "fas fa-hand",

  // misc
  [NodeType.SKIP]: "fas fa-ban",
  [NodeType.EMPTY]: "fas fa-empty-set",
});

export const ICON_BY_STRUCT_TYPE: Partial<Record<StructType, IconData>> = _makeIcons<StructType>({
  // core
  [StructType.PATH]: "fas fa-road",
  [StructType.TYPE]: "fas fa-tilde",
  [StructType.SCHEDULE]: "fas fa-calendar",
  [StructType.COMPUTED_VALUE]: "fas fa-divide",
  // files
  [StructType.ICON]: "fas fa-face-smile",
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
  [StructType.RUN_FRAME]: "fas fa-play",
  [StructType.RUN_TRACE]: "fas fa-play",
  [StructType.BREAKPOINT]: "fas fa-pause",
});

export const ICON_BY_OBJECT_TYPE: Partial<Record<ObjectType, IconData>> = {
  ...ICON_BY_NODE_TYPE,
  ...ICON_BY_STRUCT_TYPE,
};

export const ICON_BY_BENCH_TYPE: Partial<Record<BenchType, IconData>> = {
  ...ICON_BY_OBJECT_TYPE,
};

export const ICON_BY_TEXT_LINE_TYPE: Partial<Record<TextLineType, IconData>> = _makeIcons<TextLineType>({
  // plain
  [TextLineType.PARAGRAPH]: "fas fa-align-left",
  // heading
  [TextLineType.HEADING_1]: "fas fa-heading",
  [TextLineType.HEADING_2]: "fas fa-heading",
  [TextLineType.HEADING_3]: "fas fa-heading",
  [TextLineType.HEADING_4]: "fas fa-heading",
  // highlight
  [TextLineType.CALLOUT]: "fas fa-circle-exclamation",
  [TextLineType.QUOTE]: "fas fa-quote-left",
  // list
  [TextLineType.LIST_UNORDERED]: "fas fa-list-ul",
  [TextLineType.LIST_ORDERED]: "fas fa-list-ol",
  // presentation
  [TextLineType.DIVIDER]: "fas fa-horizontal-rule",
  // table
  [TextLineType.TABLE]: "fas fa-table",
  [TextLineType.TABLE_ROW]: "fas fa-table-rows",
  // code
  [TextLineType.CODE]: "fas fa-code",
});

export const ICON_BY_BLOCK_TYPE: Partial<Record<BlockType, IconData>> = _makeIcons<BlockType>({
  // nodes
  ...ICON_BY_NODE_TYPE,
  // text (like ICON_BY_TEXT_LINE_TYPE but +10000)
  ...Object.fromEntries(
    Object.entries(ICON_BY_TEXT_LINE_TYPE).map(([key, value]) => {
      return [Number.parseInt(key) + 10_000, value];
    }),
  ),
});

export const ICON_BY_ACTION_TYPE: Partial<Record<ActionType, IconData>> = _makeIcons<ActionType>({
  // flow
  [ActionType.START]: "fas fa-circle-play",
  [ActionType.COMPLETE]: "fas fa-flag-checkered",
  [ActionType.FAIL]: "fas fa-triangle-exclamation",
  // tool
  [ActionType.CODE]: "fas fa-code",
  [ActionType.TOOL]: "fas fa-screwdriver-wrench",
  // generic
  [ActionType.ACT]: "fas fa-hammer",
  [ActionType.THINK]: "fas fa-brain-circuit",
  [ActionType.ROUTE]: "fas fa-split",
  [ActionType.GENERATE]: "fas fa-wand-magic-sparkles",
  [ActionType.TRANSFORM]: "fas fa-arrows-rotate",
  [ActionType.EXTRACT]: "fas fa-filter",
  [ActionType.CLASSIFY]: "fas fa-tags",
  [ActionType.SUMMARIZE]: "fas fa-file-lines",
  [ActionType.COMPARE]: "fas fa-code-compare",
  [ActionType.TRANSLATE]: "fas fa-language",
  [ActionType.CHANGE]: "fas fa-pen-to-square",
  // read
  [ActionType.GET]: "fas fa-magnifying-glass",
  [ActionType.SEARCH]: "fas fa-magnifying-glass",
  // write
  [ActionType.CREATE]: "fas fa-plus",
  [ActionType.DUPLICATE]: "fas fa-clone",
  [ActionType.UPDATE]: "fas fa-pencil",
  [ActionType.DELETE]: "fas fa-trash",
  // async
  [ActionType.SEND]: "fas fa-inbox-out",
  [ActionType.RECEIVE]: "fas fa-inbox-in",
  [ActionType.WAIT]: "fas fa-clock",
  [ActionType.YIELD]: "fas fa-hand",
  // environment
  [ActionType.LOOK]: "fas fa-eye",
  // application
  [ActionType.CLICK]: "fas fa-arrow-pointer",
  [ActionType.PRESS]: "fas fa-keyboard",
  [ActionType.TYPE]: "fas fa-keyboard",
  [ActionType.SCROLL]: "fas fa-computer-mouse-scrollwheel",
  [ActionType.SELECT]: "fas fa-lasso",
  [ActionType.DRAG]: "fas fa-hand-pointer",
  [ActionType.GO_BACKWARD]: "fas fa-arrow-turn-left",
  [ActionType.GO_FORWARD]: "fas fa-arrow-turn-right",
  // web
  [ActionType.GO_TO_URL]: "fas fa-link",
  [ActionType.GO_TO_TAB]: "fas fa-sidebar",
  [ActionType.OPEN_TAB]: "fas fa-plus",
  [ActionType.CLOSE_TAB]: "fas fa-minus",
  // containers
  // ...
  // misc
  [ActionType.TEXT]: "fas fa-align-left",
});

export const ICON_BY_ACTION_CATEGORY: Partial<Record<ActionCategory, IconData>> = _makeIcons<ActionCategory>({
  [ActionCategory.FLOW]: "fas fa-diagram-project",
  [ActionCategory.READ]: "fas fa-magnifying-glass",
  [ActionCategory.WRITE]: "fas fa-pencil",
  [ActionCategory.COMMUNICATE]: "fas fa-inbox",
  [ActionCategory.ENVIRONMENT]: "fas fa-island-tropical",
  [ActionCategory.APPLICATION]: "fas fa-desktop",
  [ActionCategory.WEB]: "fas fa-globe",
});

export const ICON_BY_TOOL_FILTER: Partial<Record<ToolFilter, IconData>> = _makeIcons<ToolFilter>({
  [ToolFilter.ANY]: "fas fa-infinity",
  [ToolFilter.SELECT_BUILIN]: "fas fa-gear",
  [ToolFilter.SELECT_CUSTOM]: "fas fa-user",
  [ToolFilter.SELECT]: "fas fa-hexagon",
});

export const ICON_BY_PIPE_TYPE: Partial<Record<PipeType, IconData>> = _makeIcons<PipeType>({
  [PipeType.CALL]: "fas fa-arrow-right-long",
  [PipeType.SELECT]: "fas fa-square-dashed",
});

export const ICON_BY_PIPE_TRIGGER: Partial<Record<PipeTrigger, IconData>> = _makeIcons<PipeTrigger>({
  [PipeTrigger.ON_COMPLETED]: "fas fa-circle-check",
  [PipeTrigger.ON_FAILED]: "fas fa-circle-exclamation",
  [PipeTrigger.ON_TERMINATED]: "fas fa-circle-stop",
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
  [ViewType.ACTION]: "fas fa-shoe-prints",
  [ViewType.TYPE]: "fas fa-objects-column",
  [ViewType.OBJECT]: "fas fa-cubes",
  [ViewType.PIPE]: "fas fa-tachometer-alt",
  [ViewType.RUN]: "fas fa-play",

  // utilities
  [ViewType.TREE]: "fas fa-list-tree",
  [ViewType.CREATE]: "fas fa-plus",
  [ViewType.CHAT]: "fas fa-message",
  [ViewType.FEED]: "fas fa-list-timeline",

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
  [PrimitiveType.DURATION]: "fas fa-stopwatch",
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

export const ICON_BY_SEVERITY: Record<Severity, IconData> = {
  [Severity.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [Severity.TRACE]: makeIcon({ faName: "fas fa-bug" }),
  [Severity.DEBUG]: makeIcon({ faName: "fas fa-bug" }),
  [Severity.INFO]: makeIcon({ faName: "fas fa-circle-check" }),
  [Severity.WARNING]: makeIcon({ faName: "fas fa-circle-exclamation" }),
  [Severity.ERROR]: makeIcon({ faName: "fas fa-circle-exclamation" }),
  [Severity.PANIC]: makeIcon({ faName: "fas fa-skull" }),
};

export const ICON_BY_RUN_STATUS: Record<RunStatus, IconData> = {
  [RunStatus.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [RunStatus.SCHEDULED]: makeIcon({ faName: "fas fa-clock" }),
  [RunStatus.QUEUED]: makeIcon({ faName: "fas fa-hourglass" }),
  [RunStatus.RUNNING]: makeIcon({ faName: "fas fa-circle-notch" }),
  [RunStatus.PAUSED]: makeIcon({ faName: "fas fa-circle-pause" }),
  [RunStatus.YIELDED]: makeIcon({ faName: "fas fa-circle-pause" }),
  [RunStatus.WAITING]: makeIcon({ faName: "fas fa-circle-pause" }),
  [RunStatus.COMPLETED]: makeIcon({ faName: "fas fa-circle-check" }),
  [RunStatus.CANCELLED]: makeIcon({ faName: "fas fa-circle-xmark" }),
  [RunStatus.ABORTED]: makeIcon({ faName: "fas fa-skull" }),
  [RunStatus.FAILED]: makeIcon({ faName: "fas fa-circle-exclamation" }),
};

export const ICON_BY_RUN_SPAN_TYPE: Record<RunSpanType, IconData> = {
  [RunSpanType.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [RunSpanType.ATTEMPT]: makeIcon({ faName: "fas fa-play" }),
  [RunSpanType.ACQUIRE]: makeIcon({ faName: "fas fa-toolbox" }),
  [RunSpanType.WAIT]: makeIcon({ faName: "fas fa-hourglass-end" }),
  [RunSpanType.DELEGATE]: makeIcon({ faName: "fas fa-play" }),
  [RunSpanType.FLOW_PLAN]: makeIcon({ faName: "fas fa-hexagon-nodes" }),
  [RunSpanType.MODEL_PREPARE]: makeIcon({ faName: "fas fa-hexagon-nodes" }),
  [RunSpanType.MODEL_GENERATE]: makeIcon({ faName: "fas fa-hexagon-nodes" }),
  [RunSpanType.MODEL_PARSE]: makeIcon({ faName: "fas fa-hexagon-nodes" }),
  [RunSpanType.FILE_DOWNLOAD]: makeIcon({ faName: "fas fa-download" }),
  [RunSpanType.FILE_PREPARE_DOWNLOAD]: makeIcon({ faName: "fas fa-download" }),
  [RunSpanType.FILE_UPLOAD]: makeIcon({ faName: "fas fa-upload" }),
  [RunSpanType.FILE_PREPARE_UPLOAD]: makeIcon({ faName: "fas fa-upload" }),
};

export const ICON_BY_INTERRUPTION_TYPE: Record<InterruptionType, IconData> = {
  [InterruptionType.UNSPECIFIED]: makeIcon({ faName: "fas fa-bug" }),
  [InterruptionType.YIELD]: makeIcon({ faName: "fas fa-hand" }),
  [InterruptionType.PAUSE]: makeIcon({ faName: "fas fa-pause" }),
  [InterruptionType.WAIT]: makeIcon({ faName: "fas fa-hourglass-start" }),
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
  [NodeMode.ARCHIVE]: "fas fa-box-archive",
});

export const ICON_BY_REGION_CONTINENT: Partial<Record<RegionContinent, IconData>> = _makeIcons<RegionContinent>({
  [RegionContinent.EUROPE]: "🇪🇺",
  [RegionContinent.NORTH_AMERICA]: "🇺🇸",
  [RegionContinent.SOUTH_AMERICA]: "fas fas fa-globe-americas",
  [RegionContinent.MIDDLE_EAST]: "fas fas fa-globe-asia",
  [RegionContinent.AFRICA]: "fas fas fa-globe-africa",
  [RegionContinent.ASIA]: "fas fas fa-globe-asia",
  [RegionContinent.AUSTRALIA]: "🇦🇺",
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
  [EnumType.ACTION_TYPE]: ICON_BY_ACTION_TYPE,
  [EnumType.ACTION_CATEGORY]: ICON_BY_ACTION_CATEGORY,
  [EnumType.TOOL_FILTER]: ICON_BY_TOOL_FILTER,
  [EnumType.PIPE_TYPE]: ICON_BY_PIPE_TYPE,
  [EnumType.PIPE_TRIGGER]: ICON_BY_PIPE_TRIGGER,
  [EnumType.VIEW_TYPE]: ICON_BY_VIEW_TYPE,
  [EnumType.ALIGNMENT]: ICON_BY_ALIGNMENT,
  [EnumType.ANCHOR]: ICON_BY_ANCHOR,
  [EnumType.PRIMITIVE_TYPE]: ICON_BY_PRIMITIVE_TYPE,
  [EnumType.TYPE_KIND]: ICON_BY_TYPE_KIND,
  [EnumType.TYPE_FORMAT]: ICON_BY_TYPE_FORMAT,
  [EnumType.FILE_TYPE]: ICON_BY_FILE_TYPE,
  [EnumType.FILE_FORMAT]: ICON_BY_FILE_FORMAT,
  [EnumType.FIELD_ZONE]: ICON_BY_FIELD_TYPE,
  [EnumType.SEVERITY]: ICON_BY_SEVERITY,
  [EnumType.EDIT_TYPE]: ICON_BY_EDIT_TYPE,
  [EnumType.NODE_MODE]: ICON_BY_NODE_MODE,
  [EnumType.REGION_CONTINENT]: ICON_BY_REGION_CONTINENT,
  [EnumType.REGION]: ICON_BY_REGION,
  [EnumType.RESOURCE_STATUS]: ICON_BY_RESOURCE_STATUS,
  [EnumType.CACHE_MODE]: ICON_BY_CACHE_MODE,
  [EnumType.TEXT_LINE_TYPE]: ICON_BY_TEXT_LINE_TYPE,
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
  } else if (node.kind == TypeKind.BASED_NODE && node.baseTypePtr != null) {
    const base = supergraph.get(node.baseTypePtr);
    if (base != null) {
      const icon = getNodeIcon(base);
      if (icon != null) return icon;
    }
    if (node.benchType == BenchType.FIELD) {
      return ICON_BY_BLOCK_TYPE[BlockType.CHOICE];
    }
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
  const prop = PROPERTY_INFOS_BY_TYPE[nodeType]?.[allProperties["type" as any]];
  if (prop?.enumType != null) {
    const enumIcons = ICONS_BY_ENUM_TYPE[prop.enumType];
    if (enumIcons?.[subtype] != null) {
      return enumIcons[subtype];
    }
  }
  return undefined;
}

/** Gets the icon for a Node (or reference). */
export function getNodeIcon(
  node: { metatype: NodeType | ObjectType } & Partial<AnyNodeData | NodeReferenceData>,
  options?: { base?: AnyNodeData | undefined | null; defaultToUndefined?: boolean },
): IconData | undefined {
  if ((node as any).icon != null) {
    // already has specific icon
    return (node as any).icon;
  } else if (isNode(node, NodeType.FIELD)) {
    // more specific icons for fields
    const icon = getTypeIcon(node);
    if (icon != null) return icon;
  } else if (BASED_NODE_TYPES.includes(node.metatype as any)) {
    // base node
    const basePtr = getBaseFromNode(node as AnyNodeData);
    const base = options?.base ?? (basePtr != null ? supergraph.get(basePtr) : undefined);
    if (base != null) {
      const icon = getNodeIcon(base, options);
      if (icon != null) return icon;
    }
  } else if (isNode(node, NodeType.ACTION) && node.toolPtr != null && node.type == ActionType.TOOL) {
    // tool node (delegate)
    const tool = supergraph.get(node.toolPtr);
    if (tool != null) {
      const icon = getNodeIcon(tool, options);
      if (icon != null) return icon;
    }
  } else if (isNode(node, NodeType.ACTION) && node.type == ActionType.CREATE && node.subnodePacked != null) {
    // create action
    const nodePartialPacked = unpackSubnodeProperty(
      NodeType.ACTION,
      ActionType.CREATE,
      node.subnodePacked,
      "nodePartialPacked",
    );
    const nodePartialType = (nodePartialPacked as any)?.["1"] as NodeType | undefined;
    if (nodePartialType != null) {
      const { node: nodePartial } = unpackPartialNode(nodePartialPacked, nodePartialType, 0);
      if (isNode(nodePartial, nodePartialType)) {
        const icon = getNodeIcon(nodePartial, options);
        if (icon != null) return icon;
      }
    }
  }

  // get icon for subtype
  const nodeType = (node as any).metatype as NodeType;
  const nodeSubtype = (node as any).type as number;
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

/** Gets the name of a Node. */
export function getNodeName(
  node: AnyNodeData,
  options?: { base?: AnyNodeData | undefined | null },
): string | undefined {
  if (node.metatype == ObjectType.RUN || node.metatype == ObjectType.INTERRUPTION) {
    // base node without own name
    const basePtr = getBaseFromNode(node);
    const base = options?.base ?? (basePtr != null ? supergraph.get(basePtr) : undefined);
    if (base != null) {
      return (base as any).title ?? (base as any).name;
    }
  } else if (isNode(node, NodeType.VIEW) && node.nodePtr != null) {
    // 'quasi' base from view
    const base = options?.base ?? supergraph.get(node.nodePtr);
    if (base != null) {
      return (base as any).title ?? (base as any).name;
    }
  }

  // default to name / title
  return (node as any).title ?? (node as any).name;
}

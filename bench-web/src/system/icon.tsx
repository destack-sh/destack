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
  NodeVisibility,
  FormatHint,
  PrimitiveType,
  StructType,
  BenchType,
  FieldKind,
} from "@/proto/wire";
import type { FunctionalComponent } from "vue";
// file is generated with:
// curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
//  | jq 'to_entries | map(select(.value.free | index("solid") or index("brands")) | {"id": .key, label: .value.label, unicode: .value.unicode, alias: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
//  > fa-icons.json
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import { IS_DEBUG, isDeveloperMode } from "@/utils/globals";

export type IconMetadata = {
  id: string;
  title: string;
  unicode: string;
  alias: string[];
  family: "fas" | "fab";
  faName: string;
};
export function metadataToIcon(metadata: IconMetadata): IconData {
  return {
    metatype: ObjectType.ICON,
    kind: IconKind.FONT_AWESOME,
    faName: `${metadata.family} fa-${metadata.id}`,
    setProperties: [],
  };
}

export const AVAILABLE_FA_ICONS: IconMetadata[] = _AVAILABLE_FA_ICONS.map((i) => ({
  ...i,
  faName: `${i.family} fa-${i.id}`,
})) as IconMetadata[];
export const AVAILABLE_ICONS_BY_ID: Record<string, IconMetadata> = Object.fromEntries(
  AVAILABLE_FA_ICONS.map((i) => [i.id, i]),
);

export const IconInline: FunctionalComponent<Pick<IconData, "emoji" | "file" | "faName">> = (props) => {
  if (props.faName) {
    // font awesome
    return <i class={props.faName + "  text-center"} />;
  } else if (props.emoji) {
    return <span>{props.emoji}</span>;
  } else {
    if (IS_DEBUG || isDeveloperMode.value) return <span>{JSON.stringify(props)}`</span>;
    else return <span>???</span>;
  }
};

export function getIconMetadata(icon: IconData): IconMetadata | undefined {
  if (icon.kind == IconKind.FONT_AWESOME) {
    const id = icon.faName!.split(" ")[1].split("-")[1];
    return AVAILABLE_ICONS_BY_ID[id];
  } else {
    return undefined;
  }
}

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
export const DEFAULT_ENUM_ICON = makeIcon({ faName: "fas fa-caret-circle-down" });

export const ICON_BY_NODE_TYPE: Partial<Record<NodeType, IconData>> = _makeIcons<NodeType>({
  // root
  [NodeType.BENCH]: "fas fa-fort",
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
  [NodeType.STEP]: "fas fa-step-forward",

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
  [ViewType.CREATE]: "fas fa-plus",
  [ViewType.TYPE]: "fas fa-objects-column",
  [ViewType.VARIABLE]: "fas fa-sliders",

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

export const ICON_BY_VISIBILITY: Partial<Record<NodeVisibility, IconData>> = _makeIcons({
  [NodeVisibility.PAGE]: "fas fa-memo-pad",
  [NodeVisibility.MODULE]: "fas fa-box-open",
  [NodeVisibility.BENCH]: "fas fa-fort",
  [NodeVisibility.PUBLIC]: "fas fa-globe",
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

export const ICON_BY_FIELD_KIND: Partial<Record<FieldKind, IconData>> = _makeIcons({
  [FieldKind.VARIABLE]: "fas fa-sliders",
  [FieldKind.MEMBER]: "fas fa-objects-column",
  [FieldKind.INPUT]: "fas fa-arrow-down-right",
  [FieldKind.OUTPUT]: "fas fa-arrow-up-right",
  [FieldKind.OPTION]: "fas fa-chevron-circle-down",
});

export const ENUM_ICONS_BY_TYPE: Partial<Record<EnumType, Record<any, IconData>>> = {
  [EnumType.NODE_TYPE]: ICON_BY_NODE_TYPE,
  [EnumType.STRUCT_TYPE]: ICON_BY_STRUCT_TYPE,
  [EnumType.OBJECT_TYPE]: ICON_BY_OBJECT_TYPE,
  [EnumType.BENCH_TYPE]: ICON_BY_BENCH_TYPE,
  [EnumType.BLOCK_TYPE]: ICON_BY_BLOCK_TYPE,
  [EnumType.VIEW_TYPE]: ICON_BY_VIEW_TYPE,
  [EnumType.NODE_VISIBILITY]: ICON_BY_VISIBILITY,
  [EnumType.PRIMITIVE_TYPE]: ICON_BY_PRIMITIVE_TYPE,
  [EnumType.FORMAT_HINT]: ICON_BY_FORMAT_HINT,
  [EnumType.FIELD_KIND]: ICON_BY_FIELD_KIND,
};

export function getNodeIcon(node: AnyNodeData | { metatype: ObjectType; type?: BlockType | ViewType }) {
  if ((node as any).icon != null) {
    return (node as any).icon;
  } else if (node.metatype == ObjectType.BLOCK) {
    const icon = ICON_BY_BLOCK_TYPE[(node as BlockData).type! as BlockType];
    if (icon != null) return icon;
  } else if (node.metatype == ObjectType.VIEW) {
    const icon = ICON_BY_VIEW_TYPE[(node as ViewData).type! as ViewType];
    if (icon != null) return icon;
  }
  return ICON_BY_NODE_TYPE[node.metatype! as unknown as NodeType] ?? DEFAULT_MISSING_ICON;
}

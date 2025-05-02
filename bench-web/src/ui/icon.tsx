import _AVAILABLE_EMOJI_ICONS from "@/assets/emoji-icons.json";
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import { supergraph } from "@/globals";
import { getBaseFromNode } from "@/language/core/const";
import { renderTextLine } from "@/language/core/text";
import type { TypeIdentity } from "@/language/core/type";
import { getSilentFileDownload } from "@/language/resource/file";
import {
  ActionType,
  BASED_NODE_TYPES,
  BenchType,
  BlockType,
  BlockTypeOptionInfo,
  ColorData,
  ColorShade,
  ColorType,
  ENUM_OPTION_INFO_BY_TYPE,
  FieldData,
  IconType,
  NodeReferenceData,
  NodeType,
  NodeTypeOptionInfo,
  ObjectType,
  PrimitiveTypeOptionInfo,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  StructType,
  StructTypeOptionInfo,
  TypeFormatOptionInfo,
  TypeKind,
  type AnyNodeData,
  type IconData,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { getColorHex, makeColor } from "@/ui/style";
import type { FunctionalComponent } from "vue";

/**
 * FontAwesome icons
 *
 * `fa-icons.json` is generated with:
 * curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json \
 * | jq 'to_entries
 *   | map(
 *       select(.value.free | index("solid") or index("brands"))
 *       | {
 *           id: .key,
 *           label: .value.label,
 *           unicode: .value.unicode,
 *           aliases: .value.search.terms,
 *           family: (
 *             if .value.free | index("brands") then "fab"
 *             else "fas"
 *             end
 *           )
 *         }
 *     )' \
 * > src/assets/fa-icons.json
 *
 * Includes solid ("fas") and brand ("fab") icons.
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
  id: i.id,
  title: i.label,
  unicode: i.unicode,
  aliases: i.aliases,
  family: i.family as "fas" | "fab",
  faName: `${i.family} fa-${i.id}`,
})) satisfies FontAwesomeIcon[];
export const AVAILABLE_FA_BRAND_ICONS: FontAwesomeIcon[] = AVAILABLE_FA_ICONS.filter((i) => i.family == "fab");
export const AVAILABLE_FA_SOLID_ICONS: FontAwesomeIcon[] = AVAILABLE_FA_ICONS.filter((i) => i.family == "fas");

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
type IconInlineProps = Pick<IconData, "emoji" | "faName" | "vscName" | "filePtr" | "fileUrl"> & {
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
  } else if (props.forceColor != null) {
    colorHex = getColorHex(props.forceColor, props.shade);
  } else if (props.color != null) {
    colorHex = getColorHex(props.color, props.shade);
  } else if (props.fallbackColor != null) {
    colorHex = getColorHex(props.fallbackColor, props.shade);
  } else {
    colorHex = undefined;
  }
  if (props.faName) {
    // font awesome
    return <span class={`${props.faName} text-center`} style={{ color: colorHex }} />;
  } else if (props.emoji) {
    // emoji
    return <span style={{ color: colorHex }}>{props.emoji}</span>;
  } else if (props.filePtr) {
    // file
    const download = getSilentFileDownload(props.filePtr);
    if (download?.getUrl.value != null) {
      // NOTE :Cleanup: why does it take this terrible inline-block/absolute hack to get IconInline images to align with the text?
      return (
        <div class="relative inline-block h-3 min-w-[1em]">
          <img src={download.getUrl.value} class="absolute -bottom-[2px] rounded-full" />
        </div>
      );
    } else {
      // downloading (skeleton)
      return (
        <span class="fas fa-circle animate-pulse" style={{ color: getColorHex(ColorType.GRAY, ColorShade.S200) }} />
      );
    }
  } else if (props.fileUrl) {
    // file url
    return <img src={props.fileUrl} class="rounded-full" />;
  }

  // invalid icon
  return <span class="fas fa-xmark rounded-xs border border-red-400 text-red-400" />;
};
IconInline.props = [
  "emoji",
  "faName",
  "vscName",
  "filePtr",
  "fileUrl",
  "color",
  "forceColor",
  "fallbackColor",
  "shade",
  "ignoreColor",
];

/** Inline functional Avatar component */
export const AvatarInline: FunctionalComponent<
  IconInlineProps & { size?: "regular" | "medium" | "large" | "title" }
> = (props) => {
  let backgroundColorHex;
  if (props.forceColor == "inherit") {
    backgroundColorHex = undefined;
  } else if (props.forceColor != null) {
    backgroundColorHex = getColorHex(props.forceColor, props.shade);
  } else if (props.color != null) {
    backgroundColorHex = getColorHex(props.color, props.shade);
  } else if (props.fallbackColor != null) {
    backgroundColorHex = getColorHex(props.fallbackColor, props.shade);
  } else {
    backgroundColorHex = undefined;
  }

  let sizeClasses;
  if (props.size === "medium") {
    if (props.faName) {
      sizeClasses = "w-10 h-10 text-lg";
    } else {
      sizeClasses = "w-10 h-10 text-xl";
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

  if (props.faName) {
    // font awesome
    if (backgroundColorHex == null) {
      baseClasses += " bg-gray-400 border border-gray-200";
    }
    return (
      <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
        <i class={`${props.faName} text-gray-900`} />
      </div>
    );
  } else if (props.emoji) {
    // emoji
    if (backgroundColorHex == null) {
      baseClasses += " border border-gray-200";
    }
    return (
      <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
        {props.emoji}
      </div>
    );
  } else if (props.filePtr) {
    // file
    const download = getSilentFileDownload(props.filePtr);
    if (download?.getUrl.value != null) {
      return <img src={download.getUrl.value} class={baseClasses + " rounded-full"} />;
    } else {
      // downloading (skeleton)
      return <div class={baseClasses + " animate-pulse rounded-full bg-gray-200"} />;
    }
  } else if (props.fileUrl) {
    // file url
    return <img src={props.fileUrl} class={baseClasses + " rounded-full"} />;
  }

  // invalid icon
  return (
    <div class={baseClasses + " border border-red-400"}>
      <span class="fas fa-xmark text-red-400" />
    </div>
  );
};
AvatarInline.props = [
  "emoji",
  "faName",
  "vscName",
  "filePtr",
  "fileUrl",
  "color",
  "forceColor",
  "fallbackColor",
  "shade",
  "ignoreColor",
  "size",
];

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
type IconIn = string | (Pick<IconData, "emoji" | "faName" | "filePtr" | "fileUrl"> & { color?: ColorIn });
const ICON_BY_STRING: Record<string, IconData> = {};

export function makeIcon(icon: IconIn): IconData {
  let type: IconType;
  if (typeof icon == "string") {
    if (icon in ICON_BY_STRING) {
      return ICON_BY_STRING[icon];
    } else if (
      icon.endsWith(".ico") ||
      icon.endsWith(".svg") ||
      icon.endsWith(".png") ||
      icon.endsWith(".jpg") ||
      icon.endsWith(".jpeg")
    ) {
      const i = { metatype: ObjectType.ICON, type: IconType.FILE_URL, fileUrl: icon };
      ICON_BY_STRING[icon] = i;
      return i;
    } else if (icon.startsWith("fa")) {
      const i = { metatype: ObjectType.ICON, type: IconType.FONT_AWESOME, faName: icon };
      ICON_BY_STRING[icon] = i;
      return i;
    } else {
      const i = { metatype: ObjectType.ICON, type: IconType.EMOJI, emoji: icon };
      ICON_BY_STRING[icon] = i;
      return i;
    }
  } else if (icon.emoji) {
    type = IconType.EMOJI;
  } else if (icon.faName) {
    type = IconType.FONT_AWESOME;
  } else if (icon.filePtr) {
    type = IconType.FILE;
  } else if (icon.fileUrl) {
    type = IconType.FILE_URL;
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

export const DEFAULT_MISSING_ICON = makeIcon({ faName: "fas fa-question" });
export const DEFAULT_VIEW_ICON = makeIcon({ faName: "fas fa-browser" });
export const DEFAULT_USER_ICON = makeIcon({ faName: "fas fa-user-tie" });
export const DEFAULT_ENUM_ICON = makeIcon({ faName: "fas fa-caret-circle-down" });
export const DEFAULT_SYSTEM_ICON = makeIcon({ faName: "fas fa-gear" });

/** Resolves the Icon for a Type */
export function getTypeIcon(node: Partial<FieldData> | TypeIdentity): IconData | undefined {
  if (node.primitiveType != null) {
    if (node.format != null) {
      const icon = TypeFormatOptionInfo[node.format]?.icon;
      if (icon != null) return makeIcon(icon);
    }
    const icon = PrimitiveTypeOptionInfo[node.primitiveType]?.icon;
    if (icon != null) return makeIcon(icon);
  } else if (node.kind == TypeKind.BASED_NODE && node.baseTypePtr != null) {
    const base = supergraph.get(node.baseTypePtr);
    if (base != null) {
      const icon = getNodeIcon(base);
      if (icon != null) return icon;
    }
    if (node.benchType == BenchType.FIELD) {
      return makeIcon(BlockTypeOptionInfo[BlockType.CHOICE]!.icon!);
    }
  } else if (node.benchType != null) {
    const icon =
      NodeTypeOptionInfo[node.benchType as unknown as NodeType]?.icon ??
      StructTypeOptionInfo[node.benchType as unknown as StructType]?.icon;
    if (icon != null) return makeIcon(icon);
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
  } else if (isNode(node, NodeType.ACTION) && node.type == ActionType.TOOL && node.toolPtr != null) {
    // tool node (delegate)
    const tool = supergraph.get(node.toolPtr);
    if (tool != null) {
      const icon = getNodeIcon(tool, options);
      if (icon != null) return icon;
    }
  }

  const nodeType = (node as any).metatype as NodeType;

  // subtype icon
  if ((node as any).type != null) {
    const nodeProperties = PROPERTY_INFOS_BY_TYPE[(node as any).metatype as ObjectType];
    const nodePropertiesEnum = PROPERTY_ENUM_BY_TYPE[(node as any).metatype as ObjectType];
    const enumType = nodeProperties[(nodePropertiesEnum as any)?.type]?.enumType!;
    const icon = ENUM_OPTION_INFO_BY_TYPE[enumType]?.[(node as any).type]?.icon;
    if (icon != null) {
      return makeIcon(icon);
    }
  }

  // generic icon for node type
  if (NodeTypeOptionInfo[nodeType]?.icon != null) {
    return makeIcon(NodeTypeOptionInfo[nodeType].icon);
  }

  // missing icon
  if (options?.defaultToUndefined) {
    return undefined;
  } else {
    return DEFAULT_MISSING_ICON;
  }
}

/** Gets the name of a Node. */
export function getNodeTitle(
  node: AnyNodeData,
  options?: { base?: AnyNodeData | undefined | null },
): string | undefined {
  if (node.metatype == ObjectType.RUN || node.metatype == ObjectType.INTERRUPTION) {
    // base node without own name
    const basePtr = getBaseFromNode(node);
    const base = options?.base ?? (basePtr != null ? supergraph.get(basePtr) : undefined);
    if (base != null) {
      return getNodeTitle(base, options);
    }
  } else if (isNode(node, NodeType.VIEW) && node.nodePtr != null) {
    // 'quasi' base from view
    const base = options?.base ?? supergraph.get(node.nodePtr);
    if (base != null) {
      return getNodeTitle(base, options);
    }
  }

  // default to name / title
  if ((node as any).title != null) {
    return renderTextLine((node as any).title);
  } else if ((node as any).name != null) {
    return (node as any).name;
  } else {
    return undefined;
  }
}

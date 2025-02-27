import _AVAILABLE_EMOJI_ICONS from "@/assets/emoji-icons.json";
import _AVAILABLE_FA_ICONS from "@/assets/fa-icons.json";
import { supergraph } from "@/globals";
import { BASED_NODE_TYPES, getBaseFromNode } from "@/language/core/const";
import type { TypeIdentity } from "@/language/core/type";
import {
  ActionType,
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
import { IS_DEV, IS_DEVELOPER_MODE } from "@/utils/globals";
import type { FunctionalComponent } from "vue";

/**
 * FontAwesome icons
 *
 * fa-icons is generated with:
 * curl https://raw.githubusercontent.com/FortAwesome/Font-Awesome/6.x/metadata/icons.json
 * | jq 'to_entries | map(select(.value.free | index("solid")) | {"id": .key, label: .value.label, unicode: .value.unicode, aliases: .value.search.terms, family: (if .value.free | index("solid") then "fas" else "fab" end)})'
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
  id: i.id,
  title: i.label,
  unicode: i.unicode,
  aliases: i.aliases,
  family: i.family as "fas" | "fab",
  faName: `${i.family} fa-${i.id}`,
})) satisfies FontAwesomeIcon[];

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

  if (props.faName) {
    // font awesome
    if (backgroundColorHex == null) {
      baseClasses += " bg-gray-400 border border-gray-200";
    }
    return (
      <div class={baseClasses} style={{ backgroundColor: backgroundColorHex }}>
        <i class={`${props.faName} text-white`} />
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
const ICON_BY_STRING: Record<string, IconData> = {};

export function makeIcon(icon: IconIn): IconData {
  let type: IconType;
  if (typeof icon == "string") {
    if (icon in ICON_BY_STRING) {
      return ICON_BY_STRING[icon];
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

/** Resolves the icon for a type :FieldIcon */
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

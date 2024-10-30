import {
  AnyNodeData,
  BlockType,
  ColorData,
  ColorShade,
  ColorType,
  LogLevel,
  NodeType,
  ObjectType,
  RunStatus,
  StepType,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { Casing, toCasing } from "@/utils/string";

export const COLOR_SHADE_INDEX: Record<ColorShade, number> = {
  [ColorShade.UNSPECIFIED]: 7,
  [ColorShade.S50]: 0,
  [ColorShade.S100]: 1,
  [ColorShade.S200]: 2,
  [ColorShade.S300]: 3,
  [ColorShade.S400]: 4,
  [ColorShade.S500]: 5,
  [ColorShade.S600]: 6,
  [ColorShade.S700]: 7,
  [ColorShade.S800]: 8,
  [ColorShade.S900]: 9,
  [ColorShade.S950]: 10,
};

// prettier-ignore
export const COLOR_HEX_BY_TYPE: Partial<Record<ColorType, string[]>> = {
  // NOTE: This is exactly the Tailwind color palette. We inline it for direct access (and to avoid Tailwind JIT performance issues).
  [ColorType.GRAY]: ["#fafaf9", "#f5f5f4", "#e7e5e4", "#d6d3d1", "#a8a29e", "#78716c", "#57534e", "#44403c", "#292524", "#1c1917", "#0c0a09"],
  [ColorType.RED]: ["#fef2f2", "#fee2e2", "#fecaca", "#fca5a5", "#f87171", "#ef4444", "#dc2626", "#b91c1c", "#991b1b", "#7f1d1d", "#450a0a"],
  [ColorType.ORANGE]: ["#fff7ed", "#ffedd5", "#fed7aa", "#fdba74", "#fb923c", "#f97316", "#ea580c", "#c2410c", "#9a3412", "#7c2d12", "#431407"],
  [ColorType.AMBER]: ["#fffbeb", "#fef3c7", "#fde68a", "#fcd34d", "#fbbf24", "#f59e0b", "#d97706", "#b45309", "#92400e", "#78350f", "#451a03"],
  [ColorType.YELLOW]: ["#fefce8", "#fef9c3", "#fef08a", "#fde047", "#facc15", "#eab308", "#ca8a04", "#a16207", "#854d0e", "#713f12", "#422006"],
  [ColorType.LIME]: ["#f7fee7", "#ecfccb", "#d9f99d", "#bef264", "#a3e635", "#84cc16", "#65a30d", "#4d7c0f", "#3f6212", "#365314", "#1a2e05"],
  [ColorType.GREEN]: ["#f0fdf4", "#dcfce7", "#bbf7d0", "#86efac", "#4ade80", "#22c55e", "#16a34a", "#15803d", "#166534", "#14532d", "#052e16"],
  [ColorType.EMERALD]: ["#ecfdf5", "#d1fae5", "#a7f3d0", "#6ee7b7", "#34d399", "#10b981", "#059669", "#047857", "#065f46", "#064e3b", "#022c22"],
  [ColorType.TEAL]: ["#f0fdfa", "#ccfbf1", "#99f6e4", "#5eead4", "#2dd4bf", "#14b8a6", "#0d9488", "#0f766e", "#115e59", "#134e4a", "#042f2e"],
  [ColorType.CYAN]: ["#ecfeff", "#cffafe", "#a5f3fc", "#67e8f9", "#22d3ee", "#06b6d4", "#0891b2", "#0e7490", "#155e75", "#164e63", "#083344"],
  [ColorType.SKY]: ["#f0f9ff", "#e0f2fe", "#bae6fd", "#7dd3fc", "#38bdf8", "#0ea5e9", "#0284c7", "#0369a1", "#075985", "#0c4a6e", "#082f49"],
  [ColorType.BLUE]: ["#eff6ff", "#dbeafe", "#bfdbfe", "#93c5fd", "#60a5fa", "#3b82f6", "#2563eb", "#1d4ed8", "#1e40af", "#1e3a8a", "#172554"],
  [ColorType.INDIGO]: ["#eef2ff", "#e0e7ff", "#c7d2fe", "#a5b4fc", "#818cf8", "#6366f1", "#4f46e5", "#4338ca", "#3730a3", "#312e81", "#1e1b4b"],
  [ColorType.VIOLET]: ["#f5f3ff", "#ede9fe", "#ddd6fe", "#c4b5fd", "#a78bfa", "#8b5cf6", "#7c3aed", "#6d28d9", "#5b21b6", "#4c1d95", "#2e1065"],
  [ColorType.PURPLE]: ["#faf5ff", "#f3e8ff", "#e9d5ff", "#d8b4fe", "#c084fc", "#a855f7", "#9333ea", "#7e22ce", "#6b21a8", "#581c87", "#3b0764"],
  [ColorType.FUCHSIA]: ["#fdf4ff", "#fae8ff", "#f5d0fe", "#f0abfc", "#e879f9", "#d946ef", "#c026d3", "#a21caf", "#86198f", "#701a75", "#4a044e"],
  [ColorType.PINK]: ["#fdf2f8", "#fce7f3", "#fbcfe8", "#f9a8d4", "#f472b6", "#ec4899", "#db2777", "#be185d", "#9d174d", "#831843", "#500724"],
  [ColorType.ROSE]: ["#fff1f2", "#ffe4e6", "#fecdd3", "#fda4af", "#fb7185", "#f43f5e", "#e11d48", "#be123c", "#9f1239", "#881337", "#4c0519"],
};
// semantic
COLOR_HEX_BY_TYPE[ColorType.SUCCESS] = COLOR_HEX_BY_TYPE[ColorType.EMERALD];
COLOR_HEX_BY_TYPE[ColorType.HINT] = COLOR_HEX_BY_TYPE[ColorType.GRAY];
COLOR_HEX_BY_TYPE[ColorType.WARNING] = COLOR_HEX_BY_TYPE[ColorType.AMBER];
COLOR_HEX_BY_TYPE[ColorType.DANGER] = COLOR_HEX_BY_TYPE[ColorType.RED];
// surface
COLOR_HEX_BY_TYPE[ColorType.PRIMARY] = COLOR_HEX_BY_TYPE[ColorType.BLUE];
COLOR_HEX_BY_TYPE[ColorType.SECONDARY] = COLOR_HEX_BY_TYPE[ColorType.GRAY];
COLOR_HEX_BY_TYPE[ColorType.ACCENT] = COLOR_HEX_BY_TYPE[ColorType.INDIGO];
COLOR_HEX_BY_TYPE[ColorType.CANVAS] = COLOR_HEX_BY_TYPE[ColorType.GRAY];

export const REAL_COLORS: ColorType[] = Object.values(ColorType).filter(
  (v) => typeof v == "number" && v >= ColorType.GRAY,
) as ColorType[];

export function getRandomColorType(options?: { except?: ColorType[] }): ColorType {
  if (options?.except == null) {
    return REAL_COLORS[Math.floor(Math.random() * REAL_COLORS.length)];
  } else {
    const availableColors = REAL_COLORS.filter((v) => !options!.except!.includes(v));
    if (availableColors.length === 0) {
      // fall back to all colors
      return REAL_COLORS[Math.floor(Math.random() * REAL_COLORS.length)];
    } else {
      return availableColors[Math.floor(Math.random() * availableColors.length)];
    }
  }
}

export function getColorHex(color: ColorType | ColorData, shade?: ColorShade): string | undefined {
  if (typeof color == "number") {
    return COLOR_HEX_BY_TYPE[color]?.[COLOR_SHADE_INDEX[shade ?? ColorShade.S600]];
  } else if (color.type != null) {
    return COLOR_HEX_BY_TYPE[color.type]?.[COLOR_SHADE_INDEX[shade ?? color.shade ?? ColorShade.S600]];
  } else if (color.hex != null) {
    return color.hex;
  } else {
    return undefined;
  }
}

export function getRunColorHex(status: RunStatus, shade?: ColorShade): string | undefined {
  return getColorHex(COLOR_BY_RUN_STATUS[status], shade);
}

export function getColorTitle(color: ColorType | ColorData): string | null {
  if (typeof color == "number") {
    return toCasing(ColorType[color], Casing.CAMEL);
  } else if (color.type != null) {
    return toCasing(ColorType[color.type], Casing.CAMEL);
  } else if (color.hex != null) {
    return `#${color.hex}`;
  } else {
    return null;
  }
}

export function makeColor(type: ColorType, shade?: ColorShade): ColorData {
  return { metatype: ObjectType.COLOR, type, shade };
}

//
//
//

export const COLOR_BY_NODE_TYPE: Partial<Record<NodeType, ColorType>> = {
  [NodeType.FIELD]: ColorType.EMERALD,
  [NodeType.RECORD]: ColorType.SKY,
};

export const COLOR_BY_BLOCK_TYPE: Partial<Record<BlockType, ColorType>> = {
  [BlockType.PAGE]: ColorType.GRAY,
  [BlockType.TEXT]: ColorType.GRAY,
  // state
  [BlockType.DATABASE]: ColorType.SKY,
  [BlockType.VALUE]: ColorType.SKY,
  // types
  [BlockType.CLASS]: ColorType.EMERALD,
  [BlockType.CHOICE]: ColorType.EMERALD,
  [BlockType.SIGNAL]: ColorType.EMERALD,
  [BlockType.NOTIFICATION]: ColorType.EMERALD,
  // runnables
  [BlockType.ACTION]: ColorType.ORANGE,
  [BlockType.FLOW]: ColorType.ORANGE,
  // view
  [BlockType.VIEW]: ColorType.YELLOW,
  // auth
  [BlockType.ROLE]: ColorType.PURPLE,
  [BlockType.IDENTITY]: ColorType.PURPLE,
};

export const COLOR_BY_STEP_TYPE: Partial<Record<StepType, ColorType>> = {
  [StepType.START]: ColorType.AMBER,
  [StepType.COMPLETE]: ColorType.AMBER,
  [StepType.TRIGGER]: ColorType.AMBER,
  [StepType.ACTION]: ColorType.ORANGE,
  [StepType.FAIL]: ColorType.DANGER,
  [StepType.TEXT]: ColorType.GRAY,
  [StepType.LOOP]: ColorType.GRAY,
};

export const COLOR_BY_LOG_LEVEL: Record<LogLevel, ColorType> = {
  [LogLevel.UNSPECIFIED]: ColorType.GRAY,
  [LogLevel.TRACE]: ColorType.GRAY,
  [LogLevel.DEBUG]: ColorType.GRAY,
  [LogLevel.INFO]: ColorType.GRAY,
  [LogLevel.WARNING]: ColorType.WARNING,
  [LogLevel.ERROR]: ColorType.DANGER,
  [LogLevel.CRITICAL]: ColorType.DANGER,
};

export const COLOR_BY_RUN_STATUS: Record<RunStatus, ColorType> = {
  [RunStatus.UNSPECIFIED]: ColorType.GRAY,
  [RunStatus.SCHEDULED]: ColorType.GRAY,
  [RunStatus.QUEUED]: ColorType.GRAY,
  [RunStatus.RUNNING]: ColorType.GRAY,
  [RunStatus.PAUSED]: ColorType.GRAY,
  [RunStatus.SUSPENDED]: ColorType.GRAY,
  [RunStatus.COMPLETED]: ColorType.SUCCESS,
  [RunStatus.CANCELLED]: ColorType.WARNING,
  [RunStatus.ABORTED]: ColorType.WARNING,
  [RunStatus.FAILED]: ColorType.DANGER,
};

export function getLogColorHex(level: LogLevel, shade?: ColorShade): string | undefined {
  return getColorHex(COLOR_BY_LOG_LEVEL[level], shade);
}

export function getNodeColorHex(node: AnyNodeData, shade?: ColorShade): string | undefined {
  if (isNode(node, NodeType.BLOCK)) {
    if (COLOR_BY_BLOCK_TYPE[node.type] != null) {
      return getColorHex(COLOR_BY_BLOCK_TYPE[node.type]!, shade);
    }
  } else if (isNode(node, NodeType.STEP)) {
    if (COLOR_BY_STEP_TYPE[node.type] != null) {
      return getColorHex(COLOR_BY_STEP_TYPE[node.type]!, shade);
    }
  }

  if (COLOR_BY_NODE_TYPE[node.metatype as unknown as NodeType] != null) {
    return getColorHex(COLOR_BY_NODE_TYPE[node.metatype as unknown as NodeType]!, shade);
  }

  return undefined;
}

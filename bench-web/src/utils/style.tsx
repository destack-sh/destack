import { LogLevel } from "@/proto/wire";

export const BG_COLOR_BY_LEVEL: Record<LogLevel, string> = {
  [LogLevel.UNSPECIFIED]: "bg-gray-500",
  [LogLevel.TRACE]: "bg-gray-500",
  [LogLevel.DEBUG]: "bg-gray-500",
  [LogLevel.INFO]: "bg-hint-500",
  [LogLevel.WARNING]: "bg-yellow-400",
  [LogLevel.ERROR]: "bg-danger-500",
  [LogLevel.FATAL]: "bg-danger-500",
};
export const ACCENT_COLOR_BY_LEVEL: Record<LogLevel, string> = {
  [LogLevel.UNSPECIFIED]: "text-gray-500",
  [LogLevel.TRACE]: "text-gray-500",
  [LogLevel.DEBUG]: "text-gray-500",
  [LogLevel.INFO]: "text-hint-500",
  [LogLevel.WARNING]: "text-warning-400",
  [LogLevel.ERROR]: "text-danger-500",
  [LogLevel.FATAL]: "text-danger-500",
};

import { ColorData, ColorShade, ColorType, ObjectType } from "@/proto/wire";

export const TAILWIND_COLOR_NAME_BY_TYPE: Record<ColorType, string> = {
  [ColorType.UNSPECIFIED]: "gray",
  // surface
  [ColorType.PRIMARY]: "primary",
  [ColorType.SECONDARY]: "secondary",
  [ColorType.ACCENT]: "accent",
  [ColorType.CANVAS]: "canvas",
  // semantic
  [ColorType.SUCCESS]: "success",
  [ColorType.HINT]: "hint",
  [ColorType.WARNING]: "warning",
  [ColorType.DANGER]: "danger",
  // real
  [ColorType.GRAY]: "gray",
  [ColorType.RED]: "red",
  [ColorType.ORANGE]: "orange",
  [ColorType.AMBER]: "amber",
  [ColorType.YELLOW]: "yellow",
  [ColorType.LIME]: "lime",
  [ColorType.GREEN]: "green",
  [ColorType.EMERALD]: "emerald",
  [ColorType.TEAL]: "teal",
  [ColorType.CYAN]: "cyan",
  [ColorType.SKY]: "sky",
  [ColorType.BLUE]: "blue",
  [ColorType.INDIGO]: "indigo",
  [ColorType.VIOLET]: "violet",
  [ColorType.PURPLE]: "purple",
  [ColorType.FUCHSIA]: "fuchsia",
  [ColorType.PINK]: "pink",
  [ColorType.ROSE]: "rose",
};

const REAL_COLORS: ColorType[] = Object.values(ColorType).filter(
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

export function getColorClass(color: ColorType | ColorData, shade?: ColorShade): string | null {
  if (typeof color === "number") {
    shade = shade ?? ColorShade.S700;
    return `text-${TAILWIND_COLOR_NAME_BY_TYPE[color]}-${shade}`;
  } else if (color.type != null) {
    shade = shade ?? color.shade ?? ColorShade.S700;
    return `text-${TAILWIND_COLOR_NAME_BY_TYPE[color.type]}-${shade}`;
  } else {
    return null;
  }
}

export function getBackgroundColorClass(color: ColorType | ColorData, shade?: ColorShade): string | null {
  if (typeof color === "number") {
    shade = shade ?? ColorShade.S700;
    return `bg-${TAILWIND_COLOR_NAME_BY_TYPE[color]}-${shade}`;
  } else if (color.type != null) {
    shade = shade ?? color.shade ?? ColorShade.S700;
    return `bg-${TAILWIND_COLOR_NAME_BY_TYPE[color.type]}-${shade}`;
  } else {
    return null;
  }
}

export function makeColor(type: ColorType, shade?: ColorShade): ColorData {
  return {
    metatype: ObjectType.COLOR,
    id: type,
    type,
    shade,
  };
}


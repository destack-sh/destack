import { IconData, LogLevel } from "@/proto/wire";
import { makeIcon } from "@/system/icon";

export const BG_COLOR_BY_LEVEL: Record<LogLevel, string> = {
  [LogLevel.UNSPECIFIED]: "bg-gray-500",
  [LogLevel.TRACE]: "bg-gray-500",
  [LogLevel.DEBUG]: "bg-gray-500",
  [LogLevel.INFO]: "bg-success-500",
  [LogLevel.WARNING]: "bg-yellow-400",
  [LogLevel.ERROR]: "bg-danger-500",
  [LogLevel.FATAL]: "bg-danger-500",
};
export const ACCENT_COLOR_BY_LEVEL: Record<LogLevel, string> = {
  [LogLevel.UNSPECIFIED]: "text-gray-500",
  [LogLevel.TRACE]: "text-gray-500",
  [LogLevel.DEBUG]: "text-gray-500",
  [LogLevel.INFO]: "text-success-500",
  [LogLevel.WARNING]: "text-warning-400",
  [LogLevel.ERROR]: "text-danger-500",
  [LogLevel.FATAL]: "text-danger-500",
};

export const DEFAULT_ICON_BY_LEVEL: Record<LogLevel, IconData> = {
  [LogLevel.UNSPECIFIED]: makeIcon({ name: "fas fa-bug" }),
  [LogLevel.TRACE]: makeIcon({ name: "fas fa-bug" }),
  [LogLevel.DEBUG]: makeIcon({ name: "fas fa-bug" }),
  [LogLevel.INFO]: makeIcon({ name: "fas fa-circle-check" }),
  [LogLevel.WARNING]: makeIcon({ name: "fas fa-exclamation-triangle" }),
  [LogLevel.ERROR]: makeIcon({ name: "fas fa-exclamation-circle" }),
  [LogLevel.FATAL]: makeIcon({ name: "fas fa-skull" }),
};

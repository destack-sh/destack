/**
 * Simple logging with our log levels (trace, debug, info, warn, error)
 */

import { LogLevel } from "@/proto/wire";
import { isDeveloperMode } from "@/utils/globals";

const CONSOLE_METHOD_MAP: Record<LogLevel, keyof typeof console> = {
  [LogLevel.UNSPECIFIED]: "log",
  [LogLevel.TRACE]: "debug",
  [LogLevel.DEBUG]: "debug",
  [LogLevel.INFO]: "info",
  [LogLevel.WARNING]: "warn",
  [LogLevel.ERROR]: "error",
  [LogLevel.FATAL]: "error",
};

const LOG_LEVEL_INDEX: Record<LogLevel, number> = {
  [LogLevel.UNSPECIFIED]: 0,
  [LogLevel.TRACE]: 1,
  [LogLevel.DEBUG]: 2,
  [LogLevel.INFO]: 3,
  [LogLevel.WARNING]: 4,
  [LogLevel.ERROR]: 5,
  [LogLevel.FATAL]: 6,
};

export class Logger {
  public static globalInstance: Logger;

  log(level: LogLevel, ...args: any[]) {
    const minLevel = isDeveloperMode.value ? LogLevel.TRACE : LogLevel.DEBUG;
    if (LOG_LEVEL_INDEX[level] < LOG_LEVEL_INDEX[minLevel]) return;

    const method = CONSOLE_METHOD_MAP[level];
    const levelName = LogLevel[level].toLowerCase();
    (console as any)[method](`[${levelName}]`, ...args);
  }

  trace(...args: any[]) {
    this.log(LogLevel.TRACE, ...args);
  }

  debug(...args: any[]) {
    this.log(LogLevel.DEBUG, ...args);
  }

  info(...args: any[]) {
    this.log(LogLevel.INFO, ...args);
  }

  warn(...args: any[]) {
    this.log(LogLevel.WARNING, ...args);
  }

  error(...args: any[]) {
    this.log(LogLevel.ERROR, ...args);
  }

  fatal(...args: any[]) {
    this.log(LogLevel.FATAL, ...args);
  }
}

Logger.globalInstance = new Logger();
export const log = Logger.globalInstance;

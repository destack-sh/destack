/**
 * Simple logging with our log levels (trace, debug, info, warn, error)
 */

import { Severity } from "@/proto/wire";
import { IS_DEV, IS_DEVELOPER_MODE } from "@/utils/globals";

const CONSOLE_METHOD_MAP: Record<Severity, keyof typeof console> = {
  [Severity.UNSPECIFIED]: "log",
  [Severity.TRACE]: "debug",
  [Severity.DEBUG]: "debug",
  [Severity.INFO]: "info",
  [Severity.WARNING]: "warn",
  [Severity.ERROR]: "error",
  [Severity.PANIC]: "error",
};

const LOG_LEVEL_INDEX: Record<Severity, number> = {
  [Severity.UNSPECIFIED]: 0,
  [Severity.TRACE]: 1,
  [Severity.DEBUG]: 2,
  [Severity.INFO]: 3,
  [Severity.WARNING]: 4,
  [Severity.ERROR]: 5,
  [Severity.PANIC]: 6,
};

export class Logger {
  public static globalInstance: Logger;

  log(level: Severity, ...args: any[]) {
    const minLevel = IS_DEVELOPER_MODE.value ? Severity.TRACE : Severity.INFO;
    if (LOG_LEVEL_INDEX[level] < LOG_LEVEL_INDEX[minLevel]) return;

    const method = CONSOLE_METHOD_MAP[level];
    const levelName = Severity[level].toLowerCase();
    (console as any)[method](`[${levelName}]`, ...args);
  }

  trace(...args: any[]) {
    if (IS_DEV) {
      // trace info is only available in developer builds
      this.log(Severity.TRACE, ...args);
    }
  }

  debug(...args: any[]) {
    this.log(Severity.DEBUG, ...args);
  }

  info(...args: any[]) {
    this.log(Severity.INFO, ...args);
  }

  warn(...args: any[]) {
    this.log(Severity.WARNING, ...args);
  }

  error(...args: any[]) {
    this.log(Severity.ERROR, ...args);
  }

  panic(...args: any[]) {
    this.log(Severity.PANIC, ...args);
  }
}

Logger.globalInstance = new Logger();
export const log = Logger.globalInstance;

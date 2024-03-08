/**
 * Simple logging with our log levels (trace, debug, info, warn, error)
 */

import { LogLevel } from "@/proto/wire";

const CONSOLE_METHOD_MAP: Record<LogLevel, keyof typeof console> = {
  [LogLevel.UNSPECIFIED]: "log",
  [LogLevel.TRACE]: "trace",
  [LogLevel.DEBUG]: "debug",
  [LogLevel.INFO]: "info",
  [LogLevel.WARNING]: "warn",
  [LogLevel.ERROR]: "error",
  [LogLevel.FATAL]: "error",
};

export class Logger {
  public static globalInstance: Logger;

  constructor(readonly name: string) {
    this.name = name;
  }

  log(level: LogLevel, ...args: any[]) {
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

Logger.globalInstance = new Logger("global");
export const log = Logger.globalInstance;

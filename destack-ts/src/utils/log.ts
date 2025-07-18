import { IS_DEV, IS_PROD, IS_WEB } from "@destack/utils/env";
import pino from "pino";

let rootLogger: pino.Logger | null = null;

export function setupLogging() {
  if (rootLogger != null) {
    return;
  }

  if (IS_WEB) {
    // log straight to console with colors
    rootLogger = pino({
      level: IS_DEV ? "trace" : "debug",
      browser: {
        asObject: true,
        write: (log: pino.LogDescriptor) => {
          const levelLabel = pino.levels.labels[log.level];
          let colorStart = "";
          let colorEnd = "\x1b[0m";
          if (levelLabel === "trace") {
            colorStart = "\x1b[90m"; // gray
          } else if (levelLabel === "debug") {
            colorStart = "\x1b[36m"; // cyan
          } else if (levelLabel === "info") {
            colorStart = "\x1b[32m"; // green
          } else if (levelLabel === "warn") {
            colorStart = "\x1b[33m"; // yellow
          } else if (levelLabel === "error" || levelLabel === "fatal") {
            colorStart = "\x1b[31m"; // red
          } else {
            colorStart = "";
            colorEnd = "";
          }
          console.log(`${colorStart}[${levelLabel}]${colorEnd} ${log.msg}`);
        },
      },
    });
  } else if (IS_PROD) {
    // log to file
    rootLogger = pino({ level: "debug" });
  } else {
    // log to console with pretty formatting
    const transport = pino.transport({
      target: "pino-pretty",
      options: { colorize: true, translateTime: "HH:MM:ss.l", ignore: "pid,hostname" },
    });
    rootLogger = pino({ level: "trace" }, transport);
  }
}

setupLogging();

/** Get a logger for a given file */
export function getLogger(name: string) {
  if (rootLogger == null) {
    throw new Error("Logger not initialized");
  }
  return rootLogger.child({ name });
}

import { IS_DEV, IS_PROD, IS_WEB } from "@destack/utils/env";
import pino from "pino";

let rootLogger: pino.Logger;

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
        switch (levelLabel) {
          case "trace":
            colorStart = "\x1b[90m"; // gray
            break;
          case "debug":
            colorStart = "\x1b[36m"; // cyan
            break;
          case "info":
            colorStart = "\x1b[32m"; // green
            break;
          case "warn":
            colorStart = "\x1b[33m"; // yellow
            break;
          case "error":
            colorStart = "\x1b[31m"; // red
            break;
          case "fatal":
            colorStart = "\x1b[35m"; // magenta
            break;
          default:
            colorStart = "";
            colorEnd = "";
        }
        console.log(`${colorStart}[${levelLabel}] ${log.msg}${colorEnd}`);
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
    options: { colorize: true, translateTime: "HH:MM:ss", ignore: "pid,hostname" },
  });
  rootLogger = pino({ level: "trace" }, transport);
}

/** Get a logger for a given file */
export function getLogger(filename: string) {
  const name = filename.split("/").pop()?.split(".")[0];
  return rootLogger.child({ name });
}

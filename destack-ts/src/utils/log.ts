import { IS_DEV, IS_PROD, IS_WEB } from "@destack/utils/env";
import pino from "pino";

let rootLogger: pino.Logger;

if (IS_WEB) {
  // log straight to console
  rootLogger = pino({
    level: IS_DEV ? "trace" : "debug",
    browser: {
      asObject: true,
      write: (log: pino.LogDescriptor) => {
        console.log(`[${pino.levels.labels[log.level]}] ${log.msg}`);
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

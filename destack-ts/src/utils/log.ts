import { IS_PROD } from "@destack/utils/env";
import pino from "pino";

const transport = IS_PROD
  ? undefined
  : pino.transport({
      target: "pino-pretty",
      options: { colorize: true, translateTime: "HH:MM:ss", ignore: "pid,hostname" },
    });

const rootLogger = pino({ level: "info" }, transport);

function setupLogging() {
  // ...?
}

/** Get a logger for a given file */
export function getLogger(filename: string) {
  const name = filename.split("/").pop()?.split(".")[0];
  return rootLogger.child({ name });
}

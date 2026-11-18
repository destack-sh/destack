import { SeverityNumber } from "@opentelemetry/api";

/** How much a record matters: OpenTelemetry's severities, from detail to a fatal failure. */
export type Level = "debug" | "info" | "warn" | "error" | "fatal";

/** The levels, from detail to a failure ending the process. */
export const LEVELS = ["debug", "info", "warn", "error", "fatal"] as const;

/** The OpenTelemetry severity number and text of each level. */
const SEVERITIES: Readonly<Record<Level, readonly [SeverityNumber, string]>> = {
    debug: [SeverityNumber.DEBUG, "DEBUG"],
    info: [SeverityNumber.INFO, "INFO"],
    warn: [SeverityNumber.WARN, "WARN"],
    error: [SeverityNumber.ERROR, "ERROR"],
    fatal: [SeverityNumber.FATAL, "FATAL"],
};

/** Levels. */
export const Level = {
    /** Read a level's OpenTelemetry severity number and text. */
    severity(level: Level): readonly [SeverityNumber, string] {
        return SEVERITIES[level];
    },

    /** Read a level from its name, such as a variable's value, refusing any other name. */
    parse(name: string): Level {
        const level = LEVELS.find((each) => each === name);
        if (level === undefined) {
            throw new TypeError(
                `level '${name}' is invalid: use debug, info, warn, error or fatal`,
            );
        }

        return level;
    },
};

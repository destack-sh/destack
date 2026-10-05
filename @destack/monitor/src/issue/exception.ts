import { SERVICE_ERROR_STATUSES } from "@destack/schema";
import type { Entry } from "../entry/index.ts";

/** The capture levels by the lowest OpenTelemetry severity each starts at. */
const LEVELS = [
    [21, "fatal"],
    [17, "error"],
    [13, "warning"],
    [1, "info"],
] as const;

/** The level of a captured exception. */
export type ExceptionLevel = (typeof LEVELS)[number][1];

/** An exception record read from its OpenTelemetry attributes. */
export interface Exception {
    /** The error type: a service error code, an error's own code or its name. */
    readonly errorType: string;
    /** The exception's class name, absent for a captured message. */
    readonly type?: string;
    /** The message. */
    readonly message: string;
    /** The stack trace as the runtime wrote it. */
    readonly stacktrace?: string;
    /** Whether the exception escaped the code recording it. */
    readonly isEscaped: boolean;
    /** The level, from the record's severity. */
    readonly level: ExceptionLevel;
    /** The grouping the capture asked for, `{{ default }}` standing for the default one. */
    readonly fingerprint?: readonly string[];
}

/** Read exception records. */
export const Exception = {
    /** Read the exception an entry records, absent for any other entry. */
    of(entry: Entry): Exception | undefined {
        // read the exception records only
        const attributes = entry.attributes;
        const message = attributes["exception.message"];
        if (entry.kind !== "log" || entry.name !== "exception" || typeof message !== "string") {
            return undefined;
        }

        // read the type, the trace and the requested grouping
        const type = text(attributes["exception.type"]);
        const stacktrace = text(attributes["exception.stacktrace"]);
        const fingerprint = attributes["exception.fingerprint"];
        const severity = entry.severity ?? 17;

        return {
            errorType: text(attributes["error.type"]) ?? type ?? "Error",
            ...(type === undefined ? {} : { type }),
            message,
            ...(stacktrace === undefined ? {} : { stacktrace }),
            isEscaped: attributes["exception.escaped"] === true,
            level: LEVELS.find(([lowest]) => severity >= lowest)?.[1] ?? "info",
            ...(Array.isArray(fingerprint) ? { fingerprint: fingerprint.map(String) } : {}),
        };
    },

    /** Report whether an exception is a declared outcome its caller receives: a service error refusing the caller, never an issue. */
    isExpected(exception: Pick<Exception, "errorType">): boolean {
        const statuses: Readonly<Record<string, number>> = SERVICE_ERROR_STATUSES;
        const status = statuses[exception.errorType];

        return status !== undefined && status < 500;
    },
};

/** Read an attribute value as text, absent unless it is a non-empty string. */
function text(value: Entry["attributes"][string] | undefined): string | undefined {
    return typeof value === "string" && value !== "" ? value : undefined;
}

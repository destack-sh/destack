import { SERVICE_ERROR_STATUSES } from "@destack/error";
import type { Attributes } from "../event/index.ts";

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
    /** Read the exception OpenTelemetry's exception attributes describe at a severity, absent without a message. */
    read(attributes: Attributes, severity: number): Exception | undefined {
        // read exceptions with a message only
        const message = attributes["exception.message"];
        if (typeof message !== "string") {
            return undefined;
        }

        // read the type, the trace and the requested grouping
        const type = text(attributes["exception.type"]);
        const stacktrace = text(attributes["exception.stacktrace"]);
        const fingerprint = attributes["exception.fingerprint"];
        const level = severity === 0 ? 17 : severity;

        return {
            errorType: text(attributes["error.type"]) ?? type ?? "Error",
            ...(type === undefined ? {} : { type }),
            message,
            ...(stacktrace === undefined ? {} : { stacktrace }),
            isEscaped: attributes["exception.escaped"] === true,
            level: LEVELS.find(([lowest]) => level >= lowest)?.[1] ?? "info",
            ...(fingerprint === undefined ? {} : { fingerprint: parseFingerprint(fingerprint) }),
        };
    },

    /** Report whether an exception is a declared outcome its caller receives: a service error refusing the caller, never an issue. */
    isExpected(exception: Pick<Exception, "errorType">): boolean {
        const statuses: Readonly<Record<string, number>> = SERVICE_ERROR_STATUSES;
        const status = statuses[exception.errorType];

        return status !== undefined && status < 500;
    },
};

/** Read a requested grouping: a list, or the list written as JSON, as a key keeps lists. */
function parseFingerprint(value: Attributes[string]): readonly string[] {
    // read a list as it is, and a key's list from its JSON
    const list: unknown = Array.isArray(value) ? value : JSON.parse(String(value));
    if (!Array.isArray(list)) {
        throw new TypeError("exception.fingerprint is a list of strings");
    }

    return list.map(String);
}

/** Read an attribute value as text, absent unless it is a non-empty string. */
function text(value: Attributes[string] | undefined): string | undefined {
    return typeof value === "string" && value !== "" ? value : undefined;
}

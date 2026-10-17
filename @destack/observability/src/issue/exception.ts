import { CALL_ERROR_STATUSES } from "@destack/error";
import { StackFrame } from "@destack/package/stack";
import { FRAMES_ATTRIBUTE } from "@destack/telemetry/otlp";
import type { Attributes } from "../event/index.ts";

/** The capture levels by the lowest OpenTelemetry severity each starts at. */
const LEVELS = [
    [21, "fatal"],
    [17, "error"],
    [13, "warning"],
    [1, "info"],
] as const;

/** The fingerprint part standing for the default grouping. */
const DEFAULT_PART = "{{ default }}";

/** The level of a captured exception. */
export type ExceptionLevel = (typeof LEVELS)[number][1];

/** The fields of an exception. */
type ExceptionFields = Pick<
    Exception,
    "errorType" | "type" | "message" | "stacktrace" | "frames" | "isEscaped" | "level" | "grouping"
>;

/** An exception record read from its OpenTelemetry attributes. */
export class Exception {
    /** The error type: a call error code, an error's own code or its name. */
    readonly errorType: string;
    /** The exception's class name, absent for a captured message. */
    readonly type: string | undefined;
    /** The message. */
    readonly message: string;
    /** The stack trace as the runtime wrote it. */
    readonly stacktrace: string | undefined;
    /** Its frames, as the emitter symbolicated them through its build, else as the trace reads. */
    readonly frames: readonly StackFrame[];
    /** Whether the exception escaped the code recording it. */
    readonly isEscaped: boolean;
    /** The level, from the record's severity. */
    readonly level: ExceptionLevel;
    /** The grouping the capture asked for, `{{ default }}` standing for the default one. */
    readonly grouping: readonly string[] | undefined;

    /** Take an exception's fields. */
    constructor(fields: ExceptionFields) {
        // take the fields
        this.errorType = fields.errorType;
        this.type = fields.type;
        this.message = fields.message;
        this.stacktrace = fields.stacktrace;
        this.frames = fields.frames;
        this.isEscaped = fields.isEscaped;
        this.level = fields.level;
        this.grouping = fields.grouping;
    }

    /** Read the exception OpenTelemetry's exception attributes describe at a severity. */
    static read(attributes: Attributes, severity: number): Exception | undefined {
        // read exceptions with a message only
        const message = attributes["exception.message"];
        if (typeof message !== "string") {
            return undefined;
        }

        // read the type, the trace and the requested grouping
        const type = text(attributes["exception.type"]);
        const stacktrace = text(attributes["exception.stacktrace"]);
        const symbolicated = text(attributes[FRAMES_ATTRIBUTE]);
        const grouping = attributes["exception.fingerprint"];
        const level = severity === 0 ? 17 : severity;

        return new Exception({
            errorType: text(attributes["error.type"]) ?? type ?? "Error",
            type,
            message,
            stacktrace,
            frames:
                symbolicated === undefined
                    ? StackFrame.read(stacktrace ?? "")
                    : StackFrame.array().parse(JSON.parse(symbolicated)),
            isEscaped: attributes["exception.escaped"] === true,
            level: LEVELS.find(([lowest]) => level >= lowest)?.[1] ?? "info",
            grouping: grouping === undefined ? undefined : parseGrouping(grouping),
        });
    }

    /** Whether the exception is a declared outcome its caller receives. */
    get isExpected(): boolean {
        const statuses: Readonly<Record<string, number>> = CALL_ERROR_STATUSES;
        const status = statuses[this.errorType];

        return status !== undefined && status < 500;
    }

    /** The exception's identity within its trace: its type, message and stack trace. */
    get identity(): string {
        return JSON.stringify([this.type, this.message, this.stacktrace]);
    }

    /** Digest the exception's grouping: its error type and in-app symbols, or the requested one. */
    async fingerprint(): Promise<string> {
        // group by the in-app frames' symbols, else their source locations, one per recursion
        const parts = this.frames
            .filter((frame) => frame.isInApp)
            .map((frame) => frame.moniker ?? `${frame.path ?? frame.file}:${frame.function ?? ""}`)
            .filter((part, index, all) => part !== all[index - 1]);

        // group a failure without in-app frames by its message
        const grouping = [this.errorType, ...(parts.length === 0 ? [this.message] : parts)];

        // replace the grouping by the capture's, expanding its default part
        const requested = this.grouping?.flatMap((part) =>
            part === DEFAULT_PART ? grouping : [part],
        );
        const bytes = new TextEncoder().encode(JSON.stringify(requested ?? grouping));

        return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)).toHex();
    }
}

/** Read a requested grouping: a list, or the list written as JSON, as a key keeps lists. */
function parseGrouping(value: Attributes[string]): readonly string[] {
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

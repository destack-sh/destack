import { CALL_ERROR_STATUSES } from "@destack/error";
import { StackFrame } from "@destack/package/stack";
import { Digest, type DigestCrypto, present } from "@destack/schema";
import { FRAMES_ATTRIBUTE } from "@destack/telemetry/otlp";
import { ObservabilityError } from "../error/error.ts";
import { type Attributes, textOf } from "../event/index.ts";

/** The error type of an exception naming none, as OpenTelemetry's `error.type` falls back. */
const UNKNOWN_ERROR_TYPE = "_OTHER";

/** The capture levels by the lowest OpenTelemetry severity each starts at. */
const LEVELS = [
    [21, "fatal"],
    [17, "error"],
    [13, "warning"],
    [1, "info"],
] as const;

/** OpenTelemetry's severity number of a record without a severity. */
const UNSPECIFIED_SEVERITY = 0;

/** The severity an exception without one takes: OpenTelemetry's ERROR. */
const ERROR_SEVERITY = 17;

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
        const type = textOf(attributes["exception.type"]);
        const stacktrace = textOf(attributes["exception.stacktrace"]);
        const symbolicated = textOf(attributes[FRAMES_ATTRIBUTE]);
        const grouping = attributes["exception.fingerprint"];
        const level = severity === UNSPECIFIED_SEVERITY ? ERROR_SEVERITY : severity;

        return new Exception({
            errorType: textOf(attributes["error.type"]) ?? type ?? UNKNOWN_ERROR_TYPE,
            type,
            message,
            stacktrace,
            frames:
                symbolicated === undefined
                    ? StackFrame.read(stacktrace ?? "")
                    : readFrames(symbolicated),
            isEscaped: attributes["exception.escaped"] === true,
            level: present(
                LEVELS.find(([lowest]) => level >= lowest),
                `the level of severity ${level}`,
            )[1],
            grouping: grouping === undefined ? undefined : parseGrouping(grouping),
        });
    }

    /** Whether the exception is a declared outcome its caller receives. */
    get isExpected(): boolean {
        const statuses: Readonly<Record<string, number>> = CALL_ERROR_STATUSES;
        const status = statuses[this.errorType];

        return status !== undefined && status < 500;
    }

    /** Identify the exception within a trace by its type, message and stack trace. */
    identity(trace: string): string {
        return JSON.stringify([trace, this.type, this.message, this.stacktrace]);
    }

    /** Digest the exception's grouping: its error type and in-app symbols, or the requested one. */
    async fingerprint(crypto: DigestCrypto): Promise<Digest> {
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

        return await Digest.of(JSON.stringify(requested ?? grouping), crypto);
    }
}

/** Read a requested grouping: a list, or the list written as JSON, as a key keeps lists. */
function parseGrouping(value: Attributes[string]): readonly string[] {
    // read a list as it is, and a key's list from its JSON
    const list: unknown = Array.isArray(value) ? value : parseJson(String(value));
    if (!Array.isArray(list)) {
        throw ObservabilityError.of("FINGERPRINT_INVALID", "exception.fingerprint is not a list");
    }

    return list.map(String);
}

/** Read a key's JSON, refusing text that is no JSON. */
function parseJson(json: string): unknown {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw ObservabilityError.of(
            "FINGERPRINT_INVALID",
            "exception.fingerprint is not valid JSON",
            {
                cause: error,
            },
        );
    }
}

/** Read an emitter's symbolicated frames, refusing text that is not a JSON list of frames. */
function readFrames(text: string): StackFrame[] {
    // refuse text that is not JSON
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch (error) {
        throw ObservabilityError.of("FRAMES_INVALID", "the exception's frames are not JSON", {
            cause: error,
        });
    }

    // refuse JSON that is not a list of frames
    const frames = StackFrame.array().safeParse(parsed);
    if (!frames.success) {
        throw ObservabilityError.of(
            "FRAMES_INVALID",
            "the exception's frames are not a list of frames",
        );
    }

    return frames.data;
}

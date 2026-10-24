import { Uuid } from "@destack/schema";
import type { Context } from "@opentelemetry/api";
import type { LogRecordProcessor, ReadWriteLogRecord } from "@opentelemetry/sdk-logs";
import type { Span, SpanProcessor } from "@opentelemetry/sdk-trace";
import { Random } from "@destack/environment";

/** The attribute of the session a signal belongs to, from OpenTelemetry semantic conventions. */
export const SESSION_ATTRIBUTE = "session.id";

/** The event names of the log records starting and ending a session. */
export const SESSION_EVENTS = { start: "session.start", end: "session.end" } as const;

/** The attribute of a session's end recording how it ended. */
export const SESSION_STATUS_ATTRIBUTE = "session.status";

/** How a session ended: without failures, with handled ones, or by an escaped one. */
export const SESSION_STATUSES = ["ok", "errored", "crashed"] as const;

/** How a session ended. */
export type SessionStatus = (typeof SESSION_STATUSES)[number];

/** One run of an application a person uses, such as a page load or a desktop window. */
export interface Session {
    /** The session's identifier, which every signal it records carries. */
    readonly id: string;
    /** When it started, in Unix milliseconds. */
    readonly startedAt: number;
    /** How it went until this snapshot: failed by the worst exception it recorded. */
    readonly status: SessionStatus;
}

/** Stamps signals with the running session and marks it by the exceptions it records. */
export class SessionProcessor {
    /** The randomness the sessions' identifiers come from. */
    readonly #random: Random;
    /** The running session as of its last exception, absent between sessions. */
    #session: Session | undefined;

    /** Stamp sessions identified from a randomness. */
    constructor(random: Random) {
        this.#random = random;
    }

    /** The span processor stamping spans with the running session. */
    readonly spans: SpanProcessor = {
        onStart: (span: Span) => {
            if (this.#session !== undefined) {
                span.setAttribute(SESSION_ATTRIBUTE, this.#session.id);
            }
        },
        onEnd: () => {},
        forceFlush: async () => {},
        shutdown: async () => {},
    };

    /** The log record processor that stamps records with the running session. */
    readonly logs: LogRecordProcessor = {
        onEmit: (record: ReadWriteLogRecord, _context?: Context) => this.#emit(record),
        forceFlush: async () => {},
        shutdown: async () => {},
    };

    /** The running session, absent between sessions. */
    get session(): Session | undefined {
        return this.#session;
    }

    /** Start a session, replacing the running one. */
    start(now: number): Session {
        this.#session = { id: Uuid.v4(this.#random), startedAt: now, status: "ok" };

        return this.#session;
    }

    /** End the running session, returning it, absent between sessions. */
    end(): Session | undefined {
        const ended = this.#session;
        this.#session = undefined;

        return ended;
    }

    /** Stamp a log record with the running session. */
    #emit(record: ReadWriteLogRecord): void {
        // stamp records of the running session only
        const session = this.#session;
        if (session === undefined) {
            return;
        }
        record.setAttribute(SESSION_ATTRIBUTE, session.id);

        // mark the session by the worst exception it records
        if (record.eventName === "exception") {
            const status = record.attributes["exception.escaped"] === true ? "crashed" : "errored";
            if (SESSION_STATUSES.indexOf(status) > SESSION_STATUSES.indexOf(session.status)) {
                this.#session = { ...session, status };
            }
        }
    }
}

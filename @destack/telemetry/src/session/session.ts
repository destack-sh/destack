import type { Context } from "@opentelemetry/api";
import type { LogRecordProcessor, ReadWriteLogRecord } from "@opentelemetry/sdk-logs";
import type { Span, SpanProcessor } from "@opentelemetry/sdk-trace";

/** The attribute naming the session a signal belongs to, from the OpenTelemetry semantic conventions. */
export const SESSION_ATTRIBUTE = "session.id";

/** How a session ended: without failures, with handled ones, or by an escaped one. */
export const SESSION_STATUSES = ["ok", "errored", "crashed"] as const;

/** How a session ended. */
export type SessionStatus = (typeof SESSION_STATUSES)[number];

/** One run of an application a person uses, such as a page load or a desktop window, from its start to its end. */
export interface Session {
    /** The session's identifier, which every signal it records carries. */
    readonly id: string;
    /** When it started, in Unix milliseconds. */
    readonly startedAt: number;
    /** How it goes so far: failed by the worst exception it recorded. */
    status: SessionStatus;
}

/** Stamps signals with the running session and marks it by the exceptions it records, after OpenTelemetry's session processors. */
export class SessionProcessor {
    /** The running session, absent between sessions. */
    #session: Session | undefined;

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

    /** The log record processor stamping records with the running session and marking it by their exceptions. */
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
    start(now = Date.now()): Session {
        this.#session = { id: crypto.randomUUID(), startedAt: now, status: "ok" };

        return this.#session;
    }

    /** End the running session, returning it, absent between sessions. */
    end(): Session | undefined {
        const ended = this.#session;
        this.#session = undefined;

        return ended;
    }

    /** Stamp a log record with the running session, marking it errored by a handled exception and crashed by an escaped one. */
    #emit(record: ReadWriteLogRecord): void {
        // stamp records of the running session only
        const session = this.#session;
        if (session === undefined) {
            return;
        }
        record.setAttribute(SESSION_ATTRIBUTE, session.id);

        // mark the session by the worst exception it records
        if (record.eventName !== "exception") {
            return;
        }
        const status = record.attributes["exception.escaped"] === true ? "crashed" : "errored";
        if (SESSION_STATUSES.indexOf(status) > SESSION_STATUSES.indexOf(session.status)) {
            session.status = status;
        }
    }
}

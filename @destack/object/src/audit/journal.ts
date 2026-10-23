import { Failure } from "@destack/error";
import {
    asc,
    eq,
    isNull,
    lte,
    min,
    inArray,
    type DatabaseConnection,
    type TransactionOptions,
    type ReconcileController,
} from "@destack/db";
import { type EventKind, type EventStore, EventTime } from "@destack/event";
import { canonicalize, Digest, Duration } from "@destack/schema";
import { CallError } from "../error/call.ts";
import { type CallKey } from "./key.ts";
import {
    REQUEST_LIFETIME_MILLISECONDS,
    RequestId,
    ObjectReference,
    type Outcome,
} from "@destack/sync";

import { ObjectError } from "../error/index.ts";
import { call } from "./call.ts";
import { AuditCaller } from "./actor.ts";
import { CallEvent, type JournalEntry, MAX_EXECUTION_BYTES } from "./execution.ts";
import { Replay } from "./replay.ts";
import { journal } from "../stack/db.ts";
import { Clock } from "@destack/environment";
import type {} from "@destack/package/import-meta";

/** The most calls one prune removes: about a millisecond of deletes. */
const MAX_PRUNE_CALLS = 1000;
/** How long calls stay by default: as long as a request identifier stays retryable. */
const LIFETIME: Duration = { milliseconds: REQUEST_LIFETIME_MILLISECONDS };

/** The options of a journal. */
export interface JournalOptions {
    /** The event log with the audited calls as call events, over the journal's database. */
    readonly events: EventStore;
    /** How long calls stay for retries: a request identifier's retry lifetime. */
    readonly lifetime?: Duration;
    /** The clock of the journal's server. */
    readonly clock: Clock;
}

/** How the journal records a call: audited or not, its caller and its place in its request. */
export interface CallRecording {
    /** Whether the call is audited, recorded as a call event once it ends. */
    readonly isAudited: boolean;
    /** The caller of the request the call belongs to. */
    readonly caller?: string;
    /** The call's place in its request, the first when absent. */
    readonly position?: number;
}

/** The caller and scope of a request. */
export interface RequestIdentity {
    /** The authenticated principal. */
    readonly caller: string;
    /** The account, space or other scope. */
    readonly scope: string;
    /** The request identifier. */
    readonly requestId: string;
}

/** The recent calls of one database, replayed to retries, audited ones recorded as call events. */
export class Journal {
    /** The journal table. */
    readonly table = journal;
    /** The journal's database. */
    readonly database: DatabaseConnection;
    /** Read the key sensitive inputs are fingerprinted under, only when a request has them. */
    readonly #key: CallKey;
    /** The event log with the audited calls. */
    readonly events: EventStore;
    /** How long calls stay, in milliseconds. */
    readonly lifetime: number;
    /** The clock retry periods and lifetimes run on. */
    readonly clock: Clock;

    /** Journal a database's calls, fingerprinting sensitive inputs under the deployment's key. */
    constructor(database: DatabaseConnection, key: CallKey, options: JournalOptions) {
        // store the database, the key, the event log and the lifetime
        this.database = database;
        this.#key = key;
        this.events = options.events;
        this.lifetime = Duration.milliseconds(options.lifetime ?? LIFETIME);
        this.clock = options.clock;
    }

    /** Fingerprint a redacted input and the sensitive values it left out, those under the key. */
    async fingerprint(redacted: unknown, sensitive: readonly unknown[]): Promise<string> {
        // digest the sensitive values under the deployment's key, absent for none
        const keyed =
            sensitive.length === 0
                ? undefined
                : new Uint8Array(
                      await crypto.subtle.sign(
                          "HMAC",
                          await this.#key(),
                          new TextEncoder().encode(canonicalize(sensitive)),
                      ),
                  ).toHex();

        return Digest.json(keyed === undefined ? redacted : { redacted, keyed });
    }

    /** Record a call, or its outcome once it ends, an ended audited one as a call event. */
    async record(
        entry: JournalEntry,
        options: CallRecording,
        transaction?: DatabaseConnection,
    ): Promise<void> {
        // bound what the call event records
        if (
            new TextEncoder().encode(canonicalize(Journal.recorded(entry))).byteLength >
            MAX_EXECUTION_BYTES
        ) {
            throw new ObjectError("INVALID_CALL", {
                message: `call exceeds ${MAX_EXECUTION_BYTES} bytes`,
            });
        }

        // record in the caller's transaction, or in a separate one of the journal
        if (transaction === undefined) {
            await this.database.transaction((opened) => this.#record(entry, options, opened));
        } else {
            await this.#record(entry, options, transaction);
        }
    }

    /** Insert a call or its outcome, appending an ended audited one to the call events. */
    async #record(
        entry: JournalEntry,
        options: CallRecording,
        transaction: DatabaseConnection,
    ): Promise<void> {
        // key the call for replay when a retry should see its outcome again
        const execution = entry.execution;
        const request = Journal.#replayKey(execution, options.caller);
        const startedAt = execution.startedAt;
        const row = {
            id: execution.id,
            scope: execution.context.scope,
            method: entry.method,
            caller: options.caller ?? null,
            request,
            position: options.position ?? 0,
            call: entry,
            startedAt,
            finishedAt: execution.finishedAt ?? null,
            expiresAt: startedAt + this.lifetime,
        };

        // insert or finish the call, and append an ended audited call, in one round trip
        const isAppended = options.isAudited && execution.outcome !== undefined;
        const [written, appended] = await Promise.all([
            transaction
                .insert(journal)
                .values(row)
                .onConflictDoUpdate({
                    target: journal.id,
                    set: { request, call: entry, finishedAt: execution.finishedAt ?? null },
                    setWhere: isNull(journal.finishedAt),
                })
                .returning({ id: journal.id }),
            isAppended
                ? this.events
                      .append(
                          Journal.kind,
                          [Journal.event(CallEvent.parse(Journal.recorded(entry)))],
                          transaction,
                      )
                      .then(
                          () => undefined,
                          (error: unknown) => {
                              // raise the event's refusal of other contents for the journal to name first
                              if (error instanceof Error && CallError.of(error) !== undefined) {
                                  return error;
                              }
                              throw error;
                          },
                      )
                : undefined,
        ]);

        // accept a repeat of a finished call, refusing other contents, then the event's refusal
        if (written.length === 0) {
            await Journal.#requireRepeat(transaction, entry);
        }
        if (appended !== undefined) {
            throw appended;
        }
    }

    /** The call events type's kind, which the journal appends to. */
    static get kind(): EventKind {
        return call.eventKind();
    }

    /** Replicate a call without its input and a success's value, as its call event records it. */
    static recorded<Entry extends JournalEntry>(entry: Entry): Entry {
        const outcome = entry.execution.outcome;

        return {
            ...entry,
            input: {},
            execution: {
                ...entry.execution,
                ...(outcome?.kind === "success" ? { outcome: { kind: "success" } } : {}),
            },
        };
    }

    /** Write an ended call as an event of its scope. */
    static event(ended: CallEvent) {
        const { execution } = ended;
        const { target } = execution;

        return {
            scope: execution.context.scope,
            id: execution.id,
            time: EventTime.of(execution.startedAt),
            keys: {
                ...ObjectReference.columns(
                    "actor",
                    AuditCaller.actor(execution.context.caller).subject,
                ),
                method: ended.method,
                packageId: execution.context.package.id,
                category: execution.category,
                outcome: execution.outcome.kind,
                target: target.id,
                targetType: target.type,
            },
            data: ended,
        };
    }

    /** Key a call for replay when a retry of its request should see its final outcome again. */
    static #replayKey(
        execution: JournalEntry["execution"],
        caller: string | undefined,
    ): string | null {
        const { requestId } = execution;
        if (
            requestId === undefined ||
            execution.digest === undefined ||
            caller === undefined ||
            !Replay.isFinal(execution.outcome)
        ) {
            return null;
        }

        return Journal.#request({ caller, scope: execution.context.scope, requestId });
    }

    /** Require a call already recorded as finished to repeat it exactly. */
    static async #requireRepeat(database: DatabaseConnection, entry: JournalEntry): Promise<void> {
        const [existing] = await database
            .select({ call: journal.call })
            .from(journal)
            .where(eq(journal.id, entry.execution.id));
        if (existing === undefined || canonicalize(existing.call) !== canonicalize(entry)) {
            throw new ObjectError("CALL_CONTENTS_DIFFER", {
                message: `call ${entry.execution.id} has conflicting contents`,
            });
        }
    }

    /** Execute a request once in one transaction, replaying the calls a previous run recorded. */
    async execute<Authorized>(
        request: RequestIdentity,
        fingerprint: string,
        steps: {
            /** Authorize the request, returning what running it needs. */
            readonly authorize: (transaction: DatabaseConnection) => Promise<Authorized>;
            /** Run the authorized request, recording its calls, and return their results. */
            readonly run: (
                transaction: DatabaseConnection,
                authorized: Authorized,
            ) => Promise<unknown[]>;
        },
        options: TransactionOptions = {},
    ): Promise<unknown[]> {
        // reject a key expired or in the future on the journal's clock
        RequestId.expiry(request.requestId, this.clock.now());

        try {
            return await this.database.transaction(async (transaction) => {
                // authorize before replaying a recorded request
                const authorized = await steps.authorize(transaction);

                return (
                    (await this.replay(request, fingerprint, transaction)) ??
                    steps.run(transaction, authorized)
                );
            }, options);
        } catch (error) {
            // answer the outcome a concurrent replica committed, which made this replica fail
            const committed = await this.replay(request, fingerprint);
            if (committed !== undefined) {
                return committed;
            }
            throw error;
        }
    }

    /** Answer a request a previous replica recorded, with its results or its failure. */
    async replay(
        request: RequestIdentity,
        fingerprint: string,
        database: DatabaseConnection = this.database,
    ): Promise<unknown[] | undefined> {
        const recorded = await this.#recorded(database, request);

        return recorded.length === 0 ? undefined : Journal.#replay(recorded, fingerprint);
    }

    /** Read the outcome of a request's last call, absent before it ran. */
    async outcome(
        request: RequestIdentity,
        database: DatabaseConnection = this.database,
    ): Promise<Outcome | undefined> {
        const recorded = await this.#recorded(database, request);

        return recorded.at(-1)?.execution.outcome;
    }

    /** Remove the oldest calls past their lifetime, at most a batch, returning how many. */
    async #prune(now: number): Promise<number> {
        // read the oldest expired calls, then delete them
        const rows = await this.database
            .select({ id: journal.id })
            .from(journal)
            .where(lte(journal.expiresAt, now))
            .orderBy(asc(journal.expiresAt))
            .limit(MAX_PRUNE_CALLS);
        if (rows.length > 0) {
            await this.database.delete(journal).where(
                inArray(
                    journal.id,
                    rows.map((row) => row.id),
                ),
            );
        }

        return rows.length;
    }

    /** Remove the calls past their request's retry lifetime as they expire. */
    controller(): ReconcileController {
        return {
            name: "journal",
            watches: [],
            keys: () => [],
            list: async () => ["expiry"],
            reconcile: async () => {
                // remove a batch and look again at the earliest expiry, or a lifetime on when none is left
                const now = this.clock.now();
                if ((await this.#prune(now)) === MAX_PRUNE_CALLS) {
                    return 0;
                }
                const next = (await this.#earliestExpiry()) ?? now + this.lifetime;

                return Math.max(0, next - now);
            },
        };
    }

    /** Read the earliest expiry of the recorded calls. */
    async #earliestExpiry(): Promise<number | undefined> {
        const [row] = await this.database
            .select({ expiresAt: min(journal.expiresAt) })
            .from(journal);

        return row?.expiresAt ?? undefined;
    }

    /** Read a request's recorded calls in order. */
    async #recorded(
        database: DatabaseConnection,
        request: RequestIdentity,
    ): Promise<JournalEntry[]> {
        const rows = await database
            .select({ call: journal.call })
            .from(journal)
            .where(eq(journal.request, Journal.#request(request)))
            .orderBy(asc(journal.position));

        return rows.map((row) => row.call);
    }

    /** Key a request by its caller, scope and identifier's key. */
    static #request(request: RequestIdentity): string {
        return canonicalize([request.caller, request.scope, RequestId.key(request.requestId)]);
    }

    /** Replay a request's recorded calls: their results, or the failure that ended them. */
    static #replay(calls: readonly JournalEntry[], fingerprint: string): unknown[] {
        // refuse other input
        if (calls.some((entry) => entry.execution.digest !== fingerprint)) {
            throw new ObjectError("REQUEST_REUSED", {
                message: "request identifier has already been used",
            });
        }

        // throw a recorded failure, else return the results
        const values: unknown[] = [];
        for (const entry of calls) {
            const outcome = entry.execution.outcome;
            if (outcome === undefined) {
                throw new TypeError(`replayed call ${entry.execution.id} has no outcome`);
            } else if (outcome.kind !== "success") {
                throw Failure.error(outcome.error);
            }
            values.push(outcome.value);
        }

        return values;
    }
}

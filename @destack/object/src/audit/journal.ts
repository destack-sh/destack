import {
    asc,
    eq,
    isNull,
    lte,
    min,
    inArray,
    type DatabaseConnection,
    type TransactionOptions,
} from "@destack/db";
import { type EventStore, EventTime } from "@destack/event";
import { canonicalize, Clock, Digest, Duration } from "@destack/schema";
import { errorOf, CallError } from "@destack/runtime/error";
import {
    REQUEST_LIFETIME_MILLISECONDS,
    RequestId,
    type CallKey,
    type RequestIdentity,
} from "@destack/runtime/request";
import type { Controller } from "@destack/runtime/control";
import type { Outcome } from "@destack/sync";
import { ObjectError } from "../error/index.ts";
import { call } from "./call.ts";
import type { EventKindOf } from "../object/object.ts";
import { AuditActor, AuditCaller } from "./actor.ts";
import { AuditCall, type JournalEntry, MAX_EXECUTION_BYTES } from "./execution.ts";
import { Replay } from "./replay.ts";
import { journal } from "../stack/db.ts";

/** The most calls one prune removes: about a millisecond of deletes. */
const MAX_PRUNE_CALLS = 1000;
/** How long calls stay by default: as long as a request identifier stays retryable. */
const LIFETIME: Duration = { milliseconds: REQUEST_LIFETIME_MILLISECONDS };

/** The options of a journal. */
export interface JournalOptions {
    /** The event log keeping the audited calls as call events, over the journal's database. */
    readonly events: EventStore;
    /** How long calls stay for retries: a request identifier's retry lifetime. */
    readonly lifetime?: Duration;
    /** The clock of the server keeping the journal. */
    readonly clock: Clock;
}

/** How the journal records a call: audited or not, its caller and its place in its request. */
export interface CallRecording {
    /** Whether the call is audited, kept as a call event once it ends. */
    readonly isAudited: boolean;
    /** The caller of the request the call belongs to. */
    readonly caller?: string;
    /** The call's place in its request, the first when absent. */
    readonly position?: number;
}

/** The recent calls of one database, replayed to retries, each audited one kept as a call event. */
export class Journal {
    /** The journal table. */
    readonly table = journal;
    /** The database keeping the journal. */
    readonly database: DatabaseConnection;
    /** Read the key sensitive inputs are fingerprinted under, only when a request carries them. */
    readonly #key: CallKey;
    /** The event log keeping the audited calls. */
    readonly #events: EventStore;
    /** How long calls stay, in milliseconds. */
    readonly lifetime: number;
    /** The clock retry periods and lifetimes run on. */
    readonly clock: Clock;

    /** Keep a database's calls, fingerprinting sensitive inputs under the deployment's key. */
    constructor(database: DatabaseConnection, key: CallKey, options: JournalOptions) {
        // keep the database, the key, the event log and the lifetime
        this.database = database;
        this.#key = key;
        this.#events = options.events;
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
        // bound what the call event keeps
        if (
            new TextEncoder().encode(canonicalize(Journal.kept(entry))).byteLength >
            MAX_EXECUTION_BYTES
        ) {
            throw new ObjectError("INVALID_CALL", `call exceeds ${MAX_EXECUTION_BYTES} bytes`);
        }

        // record in the caller's transaction, or in one of the journal's own
        if (transaction === undefined) {
            await this.database.transaction((own) => this.#record(entry, options, own));
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

        // insert the call, or replace a running one with its outcome
        const written = await transaction
            .insert(journal)
            .values(row)
            .onConflictDoUpdate({
                target: journal.id,
                set: { request, call: entry, finishedAt: execution.finishedAt ?? null },
                setWhere: isNull(journal.finishedAt),
            })
            .returning({ id: journal.id });

        // accept a repeat of a finished call, refusing other contents
        if (written.length === 0) {
            await Journal.#requireRepeat(transaction, entry);
        }
        // append a newly ended audited call to the call events
        else if (options.isAudited && execution.outcome !== undefined) {
            await this.#events.append(
                Journal.kind,
                [Journal.#event(AuditCall.parse(Journal.kept(entry)))],
                transaction,
            );
        }
    }

    /** The call events type's kind, which the journal appends to. */
    static get kind(): EventKindOf<typeof call> {
        return call.eventKind();
    }

    /** Copy a call without its input and a success's value, as its call event keeps it. */
    static kept<Entry extends JournalEntry>(entry: Entry): Entry {
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
    static #event(ended: AuditCall) {
        const { execution } = ended;
        const { target } = execution;

        return {
            scope: execution.context.scope,
            id: execution.id,
            time: EventTime.of(execution.startedAt),
            keys: {
                actor: AuditActor.key(AuditCaller.actor(execution.context.caller)),
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
            throw new ObjectError(
                "CONFLICT",
                `call ${entry.execution.id} has conflicting contents`,
            );
        }
    }

    /** Execute a request once in one transaction, replaying the calls a previous copy recorded. */
    async execute<Authorized>(
        request: RequestIdentity,
        fingerprint: string,
        steps: {
            /** Authorize the request, returning what running it needs. */
            authorize(transaction: DatabaseConnection): Promise<Authorized>;
            /** Run the authorized request, recording its calls, and return their results. */
            run(transaction: DatabaseConnection, authorized: Authorized): Promise<unknown[]>;
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
            // answer the outcome a concurrent copy committed, which made this copy fail
            const committed = await this.replay(request, fingerprint);
            if (committed !== undefined) {
                return committed;
            }
            throw error;
        }
    }

    /** Answer a request a previous copy recorded, with its results or its failure. */
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
    controller(): Controller {
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
            throw new CallError("CONFLICT", {
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
                throw errorOf(outcome.error);
            }
            values.push(outcome.value);
        }

        return values;
    }
}

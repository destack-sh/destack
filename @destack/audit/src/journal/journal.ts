import {
    Change,
    and,
    asc,
    eq,
    integer,
    isNull,
    isNotNull,
    lte,
    min,
    or,
    inArray,
    text,
    json,
    boolean,
    identifier,
    defineTable,
    index,
    uniqueIndex,
    type DatabaseConnection,
    type TransactionOptions,
    CHAIN_TERMS,
} from "@destack/db";
import { Digest, Duration, canonicalize } from "@destack/schema";
import { errorOf, ServiceError } from "@destack/service/error";
import { domainFailure } from "@destack/service/server";
import {
    REQUEST_LIFETIME_MILLISECONDS,
    RequestId,
    type CallKey,
    type RequestIdentity,
} from "@destack/service/request";
import type { Controller } from "@destack/service/control";
import { Failure, type Outcome } from "@destack/sync";
import { AuditError } from "../error/index.ts";
import { AuditHistory } from "../history/history.ts";
import { AuditCall } from "../record/call.ts";
import { MAX_EXECUTION_BYTES } from "../record/execution.ts";

/** The statuses of failures a retry replays instead of running again. */
const FINAL_STATUSES = new Set([400, 403, 404, 409, 412, 422]);
/** The most calls one prune removes: about a millisecond of deletes. */
const MAX_PRUNE_CALLS = 1000;
/** The most calls one delivery carries by default: a history inserts a hundred in tens of milliseconds. */
const DELIVERY_CALLS = 100;
/** How long delivered calls stay by default: as long as a request identifier stays retryable. */
const LIFETIME: Duration = { milliseconds: REQUEST_LIFETIME_MILLISECONDS };
/** The most calls one read returns by default. */
const READ_CALLS = 1000;
/** The longest delivery to a history, in milliseconds: well above a hundred-call insert. */
const DELIVERY_TIMEOUT_MILLISECONDS = 30_000;

/** The calls a service database executed, kept for retries and delivered to the audit history. */
export const journal = defineTable(
    "journal",
    {
        /** The call's identity. */
        id: identifier("id", "call").primaryKey(),
        /** The scope whose history receives the call. */
        scope: text("scope").notNull(),
        /** The object type and method, such as page.create. */
        method: text("method").notNull(),
        /** The caller of the request the call belongs to, whose clients follow its outcome. */
        caller: text("caller"),
        /** The caller, scope and request a retry replays the call under, absent for outcomes a retry runs again. */
        request: text("request"),
        /** The call's place in its request. */
        position: integer("position").notNull(),
        /** Whether the audit history receives the call. */
        isAudited: boolean("is_audited").notNull(),
        /** The complete call with its execution. */
        call: json("call", AuditCall).notNull(),
        /** The start time, in UTC epoch milliseconds. */
        startedAt: integer("started_at").notNull(),
        /** The end time, in UTC epoch milliseconds, absent while the call runs. */
        finishedAt: integer("finished_at"),
        /** The time the call leaves the journal once delivered, in UTC epoch milliseconds. */
        expiresAt: integer("expires_at").notNull(),
        /** The time the history accepted the call as it is now, absent while pending. */
        deliveredAt: integer("delivered_at"),
    },
    {
        log: {},
        constraints: (entry) => [
            uniqueIndex("journal_request").on(entry.request, entry.position),
            index("journal_scope_time").on(entry.scope, entry.startedAt),
            index("journal_caller").on(entry.scope, entry.caller),
            index("journal_delivery").on(entry.isAudited, entry.deliveredAt, entry.startedAt),
            index("journal_expiry").on(entry.expiresAt),
        ],
    },
);

/** The columns of a journal change the journal's controller reads. */
type JournalTimes = Pick<typeof journal.$inferSelect, "isAudited" | "deliveredAt">;

/** The history a journal delivers its audited calls to. */
export interface AuditDestination {
    /** Store a batch's calls once in one transaction. */
    ingest(
        batch: { readonly calls: AuditCall[] },
        options?: { signal?: AbortSignal },
    ): Promise<unknown>;
}

/** The options of a journal. */
export interface JournalOptions {
    /** The most calls one delivery carries. */
    readonly batch?: number;
    /** How long delivered calls stay for retries: a request identifier's retry lifetime. */
    readonly lifetime?: Duration;
}

/** The calls of one service database: executed once per request, replayed on retries, and delivered to the audit history. */
export class Journal {
    /** The journal table. */
    readonly table = journal;
    /** The database keeping the journal. */
    readonly database: DatabaseConnection;
    /** Read the key sensitive inputs are fingerprinted under, only when a request carries them. */
    readonly #key: CallKey;
    /** The most calls one delivery carries. */
    readonly batch: number;
    /** How long delivered calls stay, in milliseconds. */
    readonly lifetime: number;

    /** Keep a database's calls, fingerprinting sensitive inputs under the deployment's key. */
    constructor(database: DatabaseConnection, key: CallKey, options: JournalOptions = {}) {
        // keep the database, the key and the tuning
        this.database = database;
        this.#key = key;
        this.batch = options.batch ?? DELIVERY_CALLS;
        this.lifetime = Duration.milliseconds(options.lifetime ?? LIFETIME);
    }

    /** Fingerprint a redacted input and the sensitive values it left out, the latter under the key. */
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

    /** Record a call, or its outcome once it ends, inside a transaction when given. */
    async record(
        value: AuditCall,
        options: {
            readonly isAudited: boolean;
            readonly caller?: string;
            readonly position?: number;
        },
        transaction?: DatabaseConnection,
    ): Promise<void> {
        // read the call with its execution
        const call = AuditCall.parse(value);
        const execution = call.execution;

        // bound what the history keeps
        if (
            new TextEncoder().encode(canonicalize(AuditHistory.kept(call))).byteLength >
            MAX_EXECUTION_BYTES
        ) {
            throw new AuditError("INVALID_EVENT", `call exceeds ${MAX_EXECUTION_BYTES} bytes`);
        }

        // key the call for replay when a retry should see its outcome again
        const request =
            execution.requestId !== undefined &&
            execution.digest !== undefined &&
            options.caller !== undefined &&
            Journal.isFinal(execution.outcome)
                ? Journal.#request({
                      caller: options.caller,
                      scope: execution.context.scope,
                      requestId: execution.requestId,
                  })
                : null;

        // insert the call, or replace a running one with its outcome
        const startedAt = execution.startedAt;
        const row = {
            id: execution.id,
            scope: execution.context.scope,
            method: call.method,
            caller: options.caller ?? null,
            request,
            position: options.position ?? 0,
            isAudited: options.isAudited,
            call,
            startedAt,
            finishedAt: execution.finishedAt ?? null,
            expiresAt: startedAt + this.lifetime,
            deliveredAt: null,
        };
        const database = transaction ?? this.database;
        const written = await database
            .insert(journal)
            .values(row)
            .onConflictDoUpdate({
                target: journal.id,
                set: { request, call, finishedAt: execution.finishedAt ?? null, deliveredAt: null },
                setWhere: isNull(journal.finishedAt),
            })
            .returning({ id: journal.id });

        // accept a repeat of a finished call, and refuse other contents
        if (written.length === 0) {
            const [existing] = await database
                .select({ call: journal.call })
                .from(journal)
                .where(eq(journal.id, execution.id));
            if (existing === undefined || canonicalize(existing.call) !== canonicalize(call)) {
                throw new AuditError("CONFLICT", `call ${execution.id} has conflicting contents`);
            }
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
        // reject an expired or future key
        RequestId.expiry(request.requestId);

        try {
            return await this.database.transaction(async (transaction) => {
                // authorize, then replay a recorded request
                const authorized = await steps.authorize(transaction);
                const previous = await this.#recorded(transaction, request);
                if (previous.length > 0) {
                    return Journal.#replay(previous, fingerprint);
                }

                return steps.run(transaction, authorized);
            }, options);
        } catch (error) {
            // answer the outcome a concurrent copy committed, which made this copy fail
            const committed = await this.#recorded(this.database, request);
            if (committed.length > 0) {
                return Journal.#replay(committed, fingerprint);
            }
            throw error;
        }
    }

    /** Read recorded calls oldest first: the audited ones unless asked for every call, the pending ones when asked. */
    async read(
        options: {
            readonly limit?: number;
            readonly isAudited?: boolean;
            readonly isPending?: boolean;
        } = {},
    ): Promise<AuditCall[]> {
        const rows = await this.database
            .select({ call: journal.call })
            .from(journal)
            .where(
                and(
                    options.isAudited === false ? undefined : eq(journal.isAudited, true),
                    options.isPending === true ? isNull(journal.deliveredAt) : undefined,
                ),
            )
            .orderBy(asc(journal.startedAt), asc(journal.id))
            .limit(options.limit ?? READ_CALLS);

        return rows.map((row) => row.call);
    }

    /** Read the outcome of a request's last call, absent before it ran. */
    async outcome(
        request: RequestIdentity,
        database: DatabaseConnection = this.database,
    ): Promise<Outcome | undefined> {
        const recorded = await this.#recorded(database, request);

        return recorded.at(-1)?.execution.outcome;
    }

    /** Describe a final failure as the outcome a retry replays, absent for one a retry runs again. */
    static failure(
        error: unknown,
    ): { readonly kind: "failure"; readonly error: Failure } | undefined {
        const failure = error instanceof ServiceError ? error : domainFailure(error);
        const outcome =
            failure === undefined
                ? undefined
                : ({ kind: "failure", error: Failure.of(failure) } as const);

        return Journal.isFinal(outcome) ? outcome : undefined;
    }

    /** Report whether a retry replays an outcome instead of running the call again. */
    static isFinal(outcome: Outcome | undefined): boolean {
        return (
            outcome !== undefined &&
            (outcome.kind === "success" ||
                (outcome.kind !== "cancelled" &&
                    FINAL_STATUSES.has(outcome.error.status) &&
                    outcome.error.code !== "INSUFFICIENT_GRANT"))
        );
    }

    /** Remove delivered calls past their lifetime, up to a limit, returning how many. */
    async prune(limit: number, now = Date.now()): Promise<number> {
        // bound the batch
        if (!Number.isInteger(limit) || limit < 1 || limit > MAX_PRUNE_CALLS) {
            throw new ServiceError("BAD_REQUEST", {
                message: `journal cleanup limit must be between 1 and ${MAX_PRUNE_CALLS}`,
            });
        }

        // remove the oldest expired calls the history has, or audit never receives
        const rows = await this.database
            .select({ id: journal.id })
            .from(journal)
            .where(
                and(
                    lte(journal.expiresAt, now),
                    or(isNotNull(journal.deliveredAt), eq(journal.isAudited, false)),
                ),
            )
            .orderBy(asc(journal.expiresAt))
            .limit(limit);
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

    /** Deliver the oldest pending audited calls to a history as one batch, returning how many. */
    async deliver(history: AuditDestination, signal?: AbortSignal): Promise<number> {
        // read the oldest pending batch
        const rows = await this.#pending(this.batch);
        if (rows.length === 0) {
            return 0;
        }

        // deliver what the history keeps within a timeout, then mark the delivered versions
        const timeout = AbortSignal.timeout(DELIVERY_TIMEOUT_MILLISECONDS);
        await history.ingest(
            { calls: rows.map((row) => AuditHistory.kept(row.call)) },
            { signal: signal === undefined ? timeout : AbortSignal.any([signal, timeout]) },
        );
        const now = Date.now();
        for (let start = 0; start < rows.length; start += CHAIN_TERMS) {
            await this.database
                .update(journal)
                .set({ deliveredAt: now })
                .where(
                    or(
                        ...rows
                            .slice(start, start + CHAIN_TERMS)
                            .map((row) =>
                                and(
                                    eq(journal.id, row.id),
                                    row.finishedAt === null
                                        ? isNull(journal.finishedAt)
                                        : eq(journal.finishedAt, row.finishedAt),
                                ),
                            ),
                    ),
                );
        }

        return rows.length;
    }

    /** Deliver audited calls to a history as they commit, and remove calls past their lifetime once removable. */
    controller(history?: AuditDestination): Controller {
        return {
            name: "journal",
            watches: history === undefined ? [] : [journal],
            keys: (change) => {
                // read only the journal's changes
                if (!Change.of(change, journal)) {
                    return [];
                }

                // deliver a pending audited call, and look again once a call is delivered
                const after: JournalTimes | null = Change.after(change);
                const before: JournalTimes | null = Change.before(change);
                const isPending =
                    history !== undefined &&
                    after?.isAudited === true &&
                    after.deliveredAt === null;
                const isDelivered =
                    before?.deliveredAt === null && typeof after?.deliveredAt === "number";

                return [...(isPending ? ["audit"] : []), ...(isDelivered ? ["expiry"] : [])];
            },
            list: async () => [
                ...(history !== undefined && (await this.#pending(1)).length > 0 ? ["audit"] : []),
                "expiry",
            ],
            reconcile: async (key) => {
                // look again while full batches remain to deliver
                if (key === "audit") {
                    if (history === undefined) {
                        throw new TypeError("the journal delivers to no history");
                    }
                    const delivered = await this.deliver(history);

                    return delivered === this.batch ? 0 : undefined;
                }

                // remove a batch, then look again at the earliest removable expiry, or a lifetime on when no later call expires sooner
                const now = Date.now();
                if ((await this.prune(MAX_PRUNE_CALLS, now)) === MAX_PRUNE_CALLS) {
                    return 0;
                }
                const next = (await this.#removable()) ?? now + this.lifetime;

                return Math.max(0, next - now);
            },
        };
    }

    /** Read the earliest expiry of the calls a prune may remove. */
    async #removable(): Promise<number | undefined> {
        const [row] = await this.database
            .select({ expiresAt: min(journal.expiresAt) })
            .from(journal)
            .where(or(isNotNull(journal.deliveredAt), eq(journal.isAudited, false)));

        return row?.expiresAt ?? undefined;
    }

    /** Read a request's recorded calls in order. */
    async #recorded(database: DatabaseConnection, request: RequestIdentity): Promise<AuditCall[]> {
        const rows = await database
            .select({ call: journal.call })
            .from(journal)
            .where(eq(journal.request, Journal.#request(request)))
            .orderBy(asc(journal.position));

        return rows.map((row) => row.call);
    }

    /** Key a request by its caller, scope and identifier. */
    static #request(request: RequestIdentity): string {
        return canonicalize([request.caller, request.scope, request.requestId]);
    }

    /** Read the oldest pending audited calls. */
    async #pending(limit: number) {
        return this.database
            .select({ id: journal.id, call: journal.call, finishedAt: journal.finishedAt })
            .from(journal)
            .where(and(eq(journal.isAudited, true), isNull(journal.deliveredAt)))
            .orderBy(asc(journal.startedAt), asc(journal.id))
            .limit(limit);
    }

    /** Replay a request's recorded calls: their results, or the failure that ended them. */
    static #replay(calls: readonly AuditCall[], fingerprint: string): unknown[] {
        // refuse other input
        if (calls.some((call) => call.execution.digest !== fingerprint)) {
            throw new ServiceError("CONFLICT", {
                message: "request identifier has already been used",
            });
        }

        // throw a recorded failure, else return the results
        const values: unknown[] = [];
        for (const call of calls) {
            const outcome = call.execution.outcome;
            if (outcome === undefined) {
                throw new TypeError(`replayed call ${call.execution.id} has no outcome`);
            } else if (outcome.kind !== "success") {
                throw errorOf(outcome.error);
            }
            values.push(outcome.value);
        }

        return values;
    }
}

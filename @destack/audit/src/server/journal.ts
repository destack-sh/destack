import {
    Change,
    and,
    asc,
    eq,
    isNull,
    isNotNull,
    lte,
    min,
    or,
    inArray,
    type DatabaseConnection,
    type TransactionOptions,
    CHAIN_TERMS,
} from "@destack/db";
import { Digest, Duration, canonicalize } from "@destack/schema";
import { errorOf, ServiceError } from "@destack/service/error";
import {
    REQUEST_LIFETIME_MILLISECONDS,
    RequestId,
    type CallKey,
    type RequestIdentity,
} from "@destack/service/request";
import type { Controller } from "@destack/service/control";
import type { Outcome } from "@destack/sync";
import { AuditError } from "../error/index.ts";
import { AuditHistory } from "../history/history.ts";
import { AuditCall } from "../record/call.ts";
import { MAX_EXECUTION_BYTES } from "../record/execution.ts";
import { Replay } from "../record/replay.ts";
import { journal } from "../stack/db.ts";

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
    /** Read the current time in Unix milliseconds, the clock of the server keeping the journal, the system clock by default. */
    readonly clock?: () => number;
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
    /** Read the current time in Unix milliseconds, which retry periods and lifetimes run on. */
    readonly clock: () => number;

    /** Keep a database's calls, fingerprinting sensitive inputs under the deployment's key. */
    constructor(database: DatabaseConnection, key: CallKey, options: JournalOptions = {}) {
        // keep the database, the key and the tuning
        this.database = database;
        this.#key = key;
        this.batch = options.batch ?? DELIVERY_CALLS;
        this.lifetime = Duration.milliseconds(options.lifetime ?? LIFETIME);
        this.clock = options.clock ?? Date.now;
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

        // key the call for replay when a retry should see its outcome again, and insert the call, or replace a running one with its outcome
        const request = Journal.#replayKey(execution, options.caller);
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
            await Journal.#requireRepeat(database, call);
        }
    }

    /** Key a call for replay when a retry of its request should see its final outcome again, null otherwise. */
    static #replayKey(
        execution: AuditCall["execution"],
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
    static async #requireRepeat(database: DatabaseConnection, call: AuditCall): Promise<void> {
        const [existing] = await database
            .select({ call: journal.call })
            .from(journal)
            .where(eq(journal.id, call.execution.id));
        if (existing === undefined || canonicalize(existing.call) !== canonicalize(call)) {
            throw new AuditError("CONFLICT", `call ${call.execution.id} has conflicting contents`);
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
        RequestId.expiry(request.requestId, this.clock());

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

    /** Answer a request a previous copy recorded with its results or its failure, absent before it ran. */
    async replay(
        request: RequestIdentity,
        fingerprint: string,
        database: DatabaseConnection = this.database,
    ): Promise<unknown[] | undefined> {
        const recorded = await this.#recorded(database, request);

        return recorded.length === 0 ? undefined : Journal.#replay(recorded, fingerprint);
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

    /** Remove delivered calls past their lifetime, up to a limit, returning how many. */
    async prune(limit: number, now = this.clock()): Promise<number> {
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

        // deliver what the history keeps within a timeout and mark the delivered versions
        const timeout = AbortSignal.timeout(DELIVERY_TIMEOUT_MILLISECONDS);
        await history.ingest(
            { calls: rows.map((row) => AuditHistory.kept(row.call)) },
            { signal: signal === undefined ? timeout : AbortSignal.any([signal, timeout]) },
        );
        const now = this.clock();
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

                // remove a batch and look again at the earliest removable expiry, or a lifetime on when no later call expires sooner
                const now = this.clock();
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

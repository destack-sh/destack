import {
    and,
    eq,
    lte,
    isNull,
    asc,
    defineTable,
    text,
    integer,
    json,
    primaryKey,
    index,
    type DatabaseConnection,
    type TransactionOptions,
} from "@destack/db";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { schema } from "@destack/schema";
import { domainFailure, ServiceError } from "../error/index.ts";
import { RequestId, type RequestFingerprint, type RequestIdentity } from "../request/index.ts";

/** The statuses of failures a retry of the same request reports again. */
const FINAL_STATUSES = new Set([400, 403, 404, 409, 412, 422]);
/** The most records one prune removes, bounding the rows one DELETE locks and returns. */
const MAX_PRUNE_LIMIT = 1000;

/** A final failure of a request, which every retry reports again. */
export const Failure = schema.object({
    /** The error code, such as CONFLICT. */
    code: schema.string(),
    /** The HTTP status. */
    status: schema.number().int(),
    /** The readable message. */
    message: schema.string(),
    /** The structured details. */
    data: schema.json().optional(),
});
/** A final failure of a request, which every retry reports again. */
export type Failure = schema.Infer<typeof Failure>;

/** The outcome of a request: its public result or the failure it reports on every retry. */
export const Outcome = schema.union([
    schema.object({
        /** The public result, including a valid JSON null. */
        value: schema.json(),
    }),
    schema.object({
        /** The failure. */
        error: Failure,
    }),
]);
/** The outcome of a request: its public result or the failure it reports on every retry. */
export type Outcome = schema.Infer<typeof Outcome>;

/** The record of every request a service executed: one transaction each, with its outcome. */
export class Journal {
    /** The table declared by the consuming service. */
    readonly table: ReturnType<typeof defineJournal>;

    /** Bind a journal table without opening a connection or starting transactions. */
    constructor(table: ReturnType<typeof defineJournal>) {
        this.table = table;
    }

    /** Execute a request once in one transaction: authorize it, replay its outcome or run it, and record the outcome. */
    async execute(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
        steps: {
            /** Check the caller may make the request, before replaying or running it. */
            authorize?(transaction: DatabaseConnection): Promise<void>;
            /** Run the request, returning its public result. */
            run(transaction: DatabaseConnection): Promise<unknown>;
        },
        options: TransactionOptions = {},
    ): Promise<unknown> {
        // reject an expired or future key before any work
        const expiresAt = RequestId.expiry(request.requestId);

        try {
            return await database.transaction(async (transaction) => {
                // authorize, then replay an executed request with the same fingerprint
                await steps.authorize?.(transaction);
                const claim = await this.claim(transaction, request, fingerprint);
                if (claim.kind === "replay") {
                    return claim.value;
                }

                // run the request and record its result with its changes
                const result = await steps.run(transaction);
                await this.complete(transaction, request, result);

                return result;
            }, options);
        } catch (error) {
            // record a final failure, so retries of the request report it again
            await this.reject(database, request, fingerprint, error, expiresAt);
            throw error;
        }
    }

    /** Read the recorded outcome of a request, absent while it has none. */
    async outcome(
        database: DatabaseConnection,
        request: RequestIdentity,
    ): Promise<Outcome | undefined> {
        const row = await database
            .select({ outcome: this.table.outcome })
            .from(this.table)
            .where(this.key(request))
            .get();

        return row?.outcome ?? undefined;
    }

    /** Claim a request in the transaction executing it, or replay its recorded outcome. */
    async claim(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
    ): Promise<JournalClaim> {
        // keep the claim, the request's changes and its outcome within one commit
        requireTransaction(database);

        // reject old keys before examining storage, including after retention cleanup
        const expiresAt = RequestId.expiry(request.requestId);
        const inserted = await database
            .insert(this.table)
            .values({
                ...request,
                transaction: null,
                digest: fingerprint.digest,
                createdAt: Date.now(),
                expiresAt,
            })
            .onConflictDoNothing()
            .returning({ requestId: this.table.requestId });

        // claim a request seen for the first time
        if (inserted.length !== 0) {
            return { kind: "new" };
        }

        // wait for the first transaction on the key, then replay its outcome for the same input
        const previous = await database.select().from(this.table).where(this.key(request)).get();
        if (!previous || previous.outcome === null || previous.digest !== fingerprint.digest) {
            throw new ServiceError("CONFLICT", {
                message: "request identifier has already been used",
            });
        }

        return { kind: "replay", value: replay(previous.outcome) };
    }

    /** Record a request's result in the transaction executing it, pointing at its changes. */
    async complete(
        database: DatabaseConnection,
        request: RequestIdentity,
        value: unknown,
    ): Promise<void> {
        // record the result and the transaction identity its logged changes carry
        requireTransaction(database);
        const outcome = { value: schema.json().parse(value) };
        const transaction = (await database.log.currentTransaction()) ?? null;
        const updated = await database
            .update(this.table)
            .set({ outcome, transaction })
            .where(and(this.key(request), isNull(this.table.outcome)))
            .returning({ requestId: this.table.requestId });
        if (updated.length !== 1) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "request claim is missing or already complete",
            });
        }
    }

    /** Record the final failure of a request whose transaction rolled back. */
    async reject(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
        error: unknown,
        expiresAt: number,
    ): Promise<void> {
        // leave transient failures and challenges to retries of the same request
        const outcome = Journal.failure(error);
        if (outcome === undefined) {
            return;
        }

        // record the failure once, keeping an outcome a concurrent attempt recorded
        await database
            .insert(this.table)
            .values({
                ...request,
                transaction: null,
                digest: fingerprint.digest,
                outcome,
                createdAt: Date.now(),
                expiresAt,
            })
            .onConflictDoNothing();
    }

    /** Describe a final failure as the outcome retries report, absent for transient failures. */
    static failure(error: unknown): Outcome | undefined {
        // keep failures a retry of the same request would repeat
        const failure = error instanceof ServiceError ? error : domainFailure(error);
        if (
            failure === undefined ||
            !FINAL_STATUSES.has(failure.status) ||
            failure.code === "INSUFFICIENT_GRANT"
        ) {
            return undefined;
        }

        return {
            error: {
                code: failure.code,
                status: failure.status,
                message: failure.message,
                ...(failure.data === undefined ? {} : { data: schema.json().parse(failure.data) }),
            },
        };
    }

    /** Remove a bounded batch after its immutable retry deadlines. */
    async prune(database: DatabaseConnection, limit: number): Promise<number> {
        // bound the batch
        if (!Number.isInteger(limit) || limit < 1 || limit > MAX_PRUNE_LIMIT) {
            throw new ServiceError("BAD_REQUEST", {
                message: `request cleanup limit must be between 1 and ${MAX_PRUNE_LIMIT}`,
            });
        }

        // find the expiry that closes a batch of at most the limit
        const now = Date.now();
        const last = await database
            .select({ expiresAt: this.table.expiresAt })
            .from(this.table)
            .where(lte(this.table.expiresAt, now))
            .orderBy(asc(this.table.expiresAt))
            .offset(limit - 1)
            .limit(1)
            .get();

        // remove the batch, with the records sharing its last expiry
        const deleted = await database
            .delete(this.table)
            .where(lte(this.table.expiresAt, last?.expiresAt ?? now))
            .returning({ requestId: this.table.requestId });

        return deleted.length;
    }

    /** Select one request within its caller and authority. */
    private key(request: RequestIdentity) {
        return and(
            eq(this.table.caller, request.caller),
            eq(this.table.scope, request.scope),
            eq(this.table.requestId, request.requestId),
        );
    }
}

/** The result of claiming a request. */
export type JournalClaim =
    | {
          /** This transaction must execute the request. */
          kind: "new";
      }
    | {
          /** A committed request supplied its original result. */
          kind: "replay";
          /** The original public result, including a valid JSON null. */
          value: schema.Infer<ReturnType<typeof schema.json>>;
      };

/** Declare a service's journal of executed requests with the shared retry and retention protocol. */
export function defineJournal(name: string, module?: ModuleMetadata) {
    // qualify the journal by the declaring package, not by this one
    const owner = declaringModule(module, "defineJournal");

    return defineTable(
        name,
        {
            /** Authenticated caller identity. */
            caller: text("caller").notNull(),
            /** Account or space containing the request. */
            scope: text("scope").notNull(),
            /** Timestamped idempotency key. */
            requestId: text("request_id").notNull(),
            /** The transaction identity the request's logged changes carry, absent without a log. */
            transaction: text("transaction"),
            /** The fingerprint of the request's input without its sensitive values. */
            digest: text("digest").notNull(),
            /** The recorded outcome, absent while the claiming transaction runs. */
            outcome: json("outcome", Outcome),
            /** Persisted request time. */
            createdAt: integer("created_at").notNull(),
            /** Immutable retry deadline derived from the request identifier. */
            expiresAt: integer("expires_at").notNull(),
        },
        {
            log: {},
            constraints: (journal) => [
                primaryKey({ columns: [journal.caller, journal.scope, journal.requestId] }),
                index(`${name}_expiry`).on(journal.expiresAt),
            ],
        },
        owner,
    );
}

/** Require a database transaction, so an outcome never publishes apart from its changes. */
function requireTransaction(database: DatabaseConnection): void {
    if (!database.driver.transaction) {
        throw new ServiceError("INTERNAL_SERVER_ERROR", {
            message: "the request journal requires a database transaction",
        });
    }
}

/** Replay a recorded outcome: return its result or throw its failure. */
function replay(outcome: Outcome): schema.Infer<ReturnType<typeof schema.json>> {
    // return a recorded result
    if ("value" in outcome) {
        return outcome.value;
    }

    // report a recorded failure again
    const { code, status, message, data } = outcome.error;
    throw new ServiceError(code, { status, message, ...(data === undefined ? {} : { data }) });
}

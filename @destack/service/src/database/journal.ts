import {
    and,
    eq,
    lte,
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
import type { DatabaseTier } from "@destack/db";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { schema } from "@destack/schema";
import { canonicalize, digest } from "@destack/schema/json";
import { ServiceError } from "../error/index.ts";
import { domainFailure } from "../server/error.ts";
import { RequestId, type RequestFingerprint, type RequestIdentity } from "../request/index.ts";

/** The statuses of failures a retry reports again. */
const FINAL_STATUSES = new Set([400, 403, 404, 409, 412, 422]);
/** The most records one prune removes. */
const MAX_PRUNE_LIMIT = 1000;

/** A final failure of a request. */
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
/** A final failure of a request. */
export type Failure = schema.Infer<typeof Failure>;

/** The outcome of a request. */
export const Outcome = schema.union([
    schema.object({
        /** The result. */
        value: schema.json(),
    }),
    schema.object({
        /** The failure. */
        error: Failure,
    }),
]);
/** The outcome of a request. */
export type Outcome = schema.Infer<typeof Outcome>;

/** Read the deployment's key that sensitive inputs are fingerprinted under, the same on every instance. */
export type JournalKey = () => Promise<CryptoKey>;

/** The outcomes of executed requests, found again by their request identifier and input fingerprint. */
export class Journal {
    /** The journal table. */
    readonly table: ReturnType<typeof defineJournal>;
    /** Read the key sensitive inputs are fingerprinted under, only when a request carries them. */
    readonly #key: JournalKey;

    /** Keep a journal table, fingerprinting sensitive inputs under the deployment's key. */
    constructor(table: ReturnType<typeof defineJournal>, key: JournalKey) {
        this.table = table;
        this.#key = key;
    }

    /** Import a deployment's key from 32 secret bytes. */
    static importKey(bytes: Uint8Array<ArrayBuffer>): Promise<CryptoKey> {
        return crypto.subtle.importKey("raw", bytes, { name: "HMAC", hash: "SHA-256" }, false, [
            "sign",
        ]);
    }

    /** Derive the key of one installation from the host's key, so no installation reads another's. */
    static async derive(key: CryptoKey, installation: string): Promise<Uint8Array<ArrayBuffer>> {
        const label = new TextEncoder().encode(`journal:${installation}`);

        return new Uint8Array(await crypto.subtle.sign("HMAC", key, label));
    }

    /** Fingerprint a redacted input and the sensitive values it left out, the latter under the key. */
    async fingerprint(
        redacted: unknown,
        sensitive: readonly unknown[],
    ): Promise<RequestFingerprint> {
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

        return { digest: await digest(keyed === undefined ? redacted : { redacted, keyed }) };
    }

    /** Execute a request once in one transaction and record its outcome. */
    async execute(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
        steps: {
            /** Authorize the request. */
            authorize?(transaction: DatabaseConnection): Promise<void>;
            /** Run the request, returning its result. */
            run(transaction: DatabaseConnection): Promise<unknown>;
        },
        options: TransactionOptions = {},
    ): Promise<unknown> {
        // reject an expired or future key
        const expiresAt = RequestId.expiry(request.requestId);

        try {
            return await database.transaction(async (transaction) => {
                // authorize, then replay a recorded request
                await steps.authorize?.(transaction);
                const previous = await transaction
                    .select()
                    .from(this.table)
                    .where(this.key(request))
                    .get();
                if (previous !== undefined) {
                    return replay(previous, fingerprint);
                }

                // run the request and record its outcome
                const value = await steps.run(transaction);
                await transaction.insert(this.table).values({
                    ...request,
                    transaction: transaction.log.stamp(),
                    digest: fingerprint.digest,
                    outcome: { value: schema.json().parse(value) },
                    createdAt: Date.now(),
                    expiresAt,
                });

                return value;
            }, options);
        } catch (error) {
            // record a final failure
            await this.reject(database, request, fingerprint, error, expiresAt);
            throw error;
        }
    }

    /** Read the outcome of a request. */
    async outcome(
        database: DatabaseConnection,
        request: RequestIdentity,
    ): Promise<Outcome | undefined> {
        const row = await database
            .select({ outcome: this.table.outcome })
            .from(this.table)
            .where(this.key(request))
            .get();

        return row?.outcome;
    }

    /** Record the final failure of a request. */
    async reject(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
        error: unknown,
        expiresAt: number,
    ): Promise<void> {
        // skip transient failures
        const outcome = Journal.failure(error);
        if (outcome === undefined) {
            return;
        }

        // record the failure once
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

    /** Describe a final failure as an outcome. */
    static failure(error: unknown): Outcome | undefined {
        // keep final failures
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

    /** Remove expired records, up to a limit. */
    async prune(database: DatabaseConnection, limit: number): Promise<number> {
        // bound the batch
        if (!Number.isInteger(limit) || limit < 1 || limit > MAX_PRUNE_LIMIT) {
            throw new ServiceError("BAD_REQUEST", {
                message: `request cleanup limit must be between 1 and ${MAX_PRUNE_LIMIT}`,
            });
        }

        // find the expiry that closes the batch
        const now = Date.now();
        const last = await database
            .select({ expiresAt: this.table.expiresAt })
            .from(this.table)
            .where(lte(this.table.expiresAt, now))
            .orderBy(asc(this.table.expiresAt))
            .offset(limit - 1)
            .limit(1)
            .get();

        // remove the batch
        const deleted = await database
            .delete(this.table)
            .where(lte(this.table.expiresAt, last?.expiresAt ?? now))
            .returning({ requestId: this.table.requestId });

        return deleted.length;
    }

    /** Select one request. */
    private key(request: RequestIdentity) {
        return and(
            eq(this.table.caller, request.caller),
            eq(this.table.scope, request.scope),
            eq(this.table.requestId, request.requestId),
        );
    }
}

/** Declare a service's journal table, in the tier of the service's objects. */
export function defineJournal(
    name: string,
    options: { readonly tier?: DatabaseTier } = {},
    module?: ModuleMetadata,
) {
    // qualify the journal by the declaring package
    const owner = declaringModule(module, "defineJournal");

    return defineTable(
        name,
        {
            /** The caller. */
            caller: text("caller").notNull(),
            /** The account or space of the request. */
            scope: text("scope").notNull(),
            /** The request identifier. */
            requestId: text("request_id").notNull(),
            /** The transaction identity of the request's logged changes. */
            transaction: text("transaction"),
            /** The digest of the input without its sensitive values. */
            digest: text("digest").notNull(),
            /** The recorded outcome. */
            outcome: json("outcome", Outcome).notNull(),
            /** The request time. */
            createdAt: integer("created_at").notNull(),
            /** The retry deadline. */
            expiresAt: integer("expires_at").notNull(),
        },
        {
            ...options,
            log: {},
            constraints: (journal) => [
                primaryKey({ columns: [journal.caller, journal.scope, journal.requestId] }),
                index(`${name}_expiry`).on(journal.expiresAt),
            ],
        },
        owner,
    );
}

/** Replay a recorded outcome. */
function replay(
    recorded: { readonly digest: string; readonly outcome: Outcome },
    fingerprint: RequestFingerprint,
): schema.Infer<ReturnType<typeof schema.json>> {
    // refuse other input
    const { outcome } = recorded;
    if (recorded.digest !== fingerprint.digest) {
        throw new ServiceError("CONFLICT", { message: "request identifier has already been used" });
    }
    // return a recorded result
    else if ("value" in outcome) {
        return outcome.value;
    }

    // throw a recorded failure
    const { code, status, message, data } = outcome.error;
    throw new ServiceError(code, { status, message, ...(data === undefined ? {} : { data }) });
}

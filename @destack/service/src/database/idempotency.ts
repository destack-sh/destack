import {
    and,
    eq,
    lte,
    isNull,
    asc,
    table,
    text,
    integer,
    json,
    primaryKey,
    index,
    type DatabaseConnection,
} from "@destack/db";
import { schema } from "@destack/schema";
import { ServiceError } from "../error/index.ts";
import { requestExpiry, type RequestIdentity } from "../request/index.ts";

/** A transaction-local request journal using the service's own database and schema history. */
export class IdempotencyStore {
    /** The table declared by the consuming service. */
    readonly table: ReturnType<typeof defineRequestTable>;

    /** Bind a request table without opening a connection or starting transactions. */
    constructor(table: ReturnType<typeof defineRequestTable>) {
        this.table = table;
    }

    /** Claim a mutation or retrieve its completed result under current caller authorization. */
    async begin(
        database: DatabaseConnection,
        request: RequestIdentity,
        fingerprint: RequestFingerprint,
        matches: (
            stored: schema.Infer<ReturnType<typeof schema.json>>,
        ) => boolean | Promise<boolean>,
    ): Promise<RequestClaim> {
        // keep the claim, mutation and response within one commit
        if (!database.connection.transaction) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "request persistence requires a database transaction",
            });
        }

        // reject old keys before examining storage, including after retention cleanup
        const expiresAt = requestExpiry(request.requestId);
        const inserted = await database
            .insert(this.table)
            .values({
                ...request,
                digest: fingerprint.digest,
                keyId: fingerprint.keyId ?? null,
                createdAt: Date.now(),
                expiresAt,
            })
            .onConflictDoNothing()
            .returning({ requestId: this.table.requestId });
        if (inserted.length !== 0) {
            return { kind: "new" };
        }

        // unique-key coordination waits for the first transaction before reading its result
        const previous = await database.select().from(this.table).where(this.key(request)).get();
        if (!previous || previous.response === null || !(await matches(previous.digest))) {
            throw new ServiceError("CONFLICT", {
                message: "request identifier has already been used",
            });
        }

        return { kind: "replay", value: previous.response.value };
    }

    /** Persist the public result in the transaction that commits the mutation and its audit. */
    async complete(
        database: DatabaseConnection,
        request: RequestIdentity,
        value: unknown,
    ): Promise<void> {
        // prevent publishing a response separately from its mutation
        if (!database.connection.transaction) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "request persistence requires a database transaction",
            });
        }

        // store the response once for this request
        const result = schema.json().parse(value);
        const updated = await database
            .update(this.table)
            .set({ response: { value: result } })
            .where(and(this.key(request), isNull(this.table.response)))
            .returning({ requestId: this.table.requestId });
        if (updated.length !== 1) {
            throw new ServiceError("INTERNAL_SERVER_ERROR", {
                message: "request claim is missing or already complete",
            });
        }
    }

    /** Remove a bounded batch after its immutable retry deadlines. */
    async prune(database: DatabaseConnection, limit: number): Promise<number> {
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) {
            throw new ServiceError("BAD_REQUEST", {
                message: "request cleanup limit must be between 1 and 1000",
            });
        }

        // select expired records in bounded batches
        const rows = await database
            .select({
                caller: this.table.caller,
                scope: this.table.scope,
                procedure: this.table.procedure,
                requestId: this.table.requestId,
            })
            .from(this.table)
            .where(lte(this.table.expiresAt, Date.now()))
            .orderBy(asc(this.table.expiresAt))
            .limit(limit);
        let removed = 0;
        for (const row of rows) {
            const deleted = await database
                .delete(this.table)
                .where(this.key(row))
                .returning({ requestId: this.table.requestId });
            removed += deleted.length;
        }

        return removed;
    }

    /** Select one mutation within its caller, authority and procedure. */
    private key(request: RequestIdentity) {
        return and(
            eq(this.table.caller, request.caller),
            eq(this.table.scope, request.scope),
            eq(this.table.procedure, request.procedure),
            eq(this.table.requestId, request.requestId),
        );
    }
}

/** The public result of claiming a transaction-local request. */
export type RequestClaim =
    | {
          /** This transaction must apply the mutation. */
          kind: "new";
      }
    | {
          /** A committed request supplied its original result. */
          kind: "replay";
          /** The original public response, including a valid JSON null. */
          value: schema.Infer<ReturnType<typeof schema.json>>;
      };

/** A canonical fingerprint, optionally protected by a versioned encryption key. */
export interface RequestFingerprint {
    /** Serialized fingerprint; request secrets must be protected against offline guessing. */
    readonly digest: schema.Infer<ReturnType<typeof schema.json>>;
    /** Encryption key version indexed for rewrapping. */
    readonly keyId?: string;
}

/** Declare a service-local request journal with the shared retry and retention protocol. */
export function defineRequestTable(name: string) {
    return table(
        name,
        {
            /** Authenticated caller identity. */
            caller: text("caller").notNull(),
            /** Account or space containing the mutation. */
            scope: text("scope").notNull(),
            /** Stable procedure name. */
            procedure: text("procedure").notNull(),
            /** Timestamped idempotency key. */
            requestId: text("request_id").notNull(),
            /** Canonical request fingerprint or its encrypted representation. */
            digest: json("digest", schema.json()).notNull(),
            /** Key protecting sensitive fingerprints. */
            keyId: text("key_id"),
            /** Completed public response; wrapping distinguishes JSON null from an unfinished request. */
            response: json("response", schema.object({ value: schema.json() })),
            /** Persisted request time. */
            createdAt: integer("created_at").notNull(),
            /** Immutable retry deadline derived from the request identifier. */
            expiresAt: integer("expires_at").notNull(),
        },
        (request) => [
            primaryKey({
                columns: [request.caller, request.scope, request.procedure, request.requestId],
            }),
            index(`${name}_expiry`).on(request.expiresAt),
            index(`${name}_key`).on(request.keyId, request.scope),
        ],
    );
}

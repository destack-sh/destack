import {
    and,
    asc,
    eq,
    gt,
    lte,
    isNotNull,
    isNull,
    sql,
    type DatabaseConnection,
} from "@destack/db";
import { secret, secretVersion, vault } from "@destack/model/space";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { EnvelopeEncryption, type EncryptionContext, SecretEnvelope } from "../encryption/index.ts";
import { Secret } from "../secret/index.ts";
import { vaultRequest, vaultValue } from "../stack/index.ts";
import { secretAction } from "../audit/index.ts";
import type { VaultContext, VaultOperation } from "./context.ts";
import { Page } from "@destack/service/page";
import { IdempotencyStore } from "@destack/service/database";
import { fingerprintRequest, type RequestIdentity } from "@destack/service/request";
import { authorizeVault } from "./access.ts";
import { timingSafeEqual } from "node:crypto";

/** Minimum recoverable deletion interval. */
const DAY_MILLISECONDS = 86400000;

/** Managed secret persistence with transaction-bound access checks and audit. */
export class Vault {
    /** Prepared regional or local database. */
    readonly database: DatabaseConnection;
    /** Envelope encryption and root-key protection. */
    readonly encryption: EnvelopeEncryption;
    /** Immutable storage location authenticated into ciphertext. */
    readonly location: string;
    /** Recovery interval before scheduled deletion becomes irreversible. */
    readonly recoveryMilliseconds: number;

    /** Bind prepared storage and separately provisioned root keys. */
    constructor(
        database: DatabaseConnection,
        encryption: EnvelopeEncryption,
        location: string,
        recoveryMilliseconds = 30 * DAY_MILLISECONDS,
    ) {
        // reject an unusable hosting location or recovery interval
        if (
            !location ||
            !Number.isSafeInteger(recoveryMilliseconds) ||
            recoveryMilliseconds < DAY_MILLISECONDS
        ) {
            throw new ServiceError("BAD_REQUEST", {
                message: "vault location and recovery interval are required",
            });
        }

        // retain the host's prepared database and key protection
        this.database = database;
        this.encryption = encryption;
        this.location = location;
        this.recoveryMilliseconds = recoveryMilliseconds;
    }

    /** Read a vault after checking the selected space. */
    async getVault(input: VaultSelection, context: VaultContext) {
        return this.database.transaction(async (transaction) => {
            // authorize the selected vault before reading its metadata
            await authorizeVault(context, { ...input, operation: "vault.get" }, transaction);
            const row = await transaction
                .select()
                .from(vault)
                .where(and(eq(vault.spaceId, input.spaceId), eq(vault.resourceId, input.vaultId)))
                .get();
            if (!row) {
                throw new ServiceError("NOT_FOUND");
            }

            return { spaceId: row.spaceId, resourceId: row.resourceId };
        });
    }

    /** List only vaults whose collection the caller may inspect. */
    async listVaults(
        input: { spaceId: Secret["spaceId"]; limit?: number; cursor?: string },
        context: VaultContext,
    ) {
        return this.database.transaction(async (transaction) => {
            // collection authority covers every returned row, not a filtered partial response
            await authorizeVault(
                context,
                { spaceId: input.spaceId, operation: "vault.list" },
                transaction,
            );
            const page = new Page(input, ["vault", input.spaceId], identifier("resource"));
            const rows = await transaction
                .select({ spaceId: vault.spaceId, resourceId: vault.resourceId })
                .from(vault)
                .where(
                    and(
                        eq(vault.spaceId, input.spaceId),
                        page.after ? gt(vault.resourceId, page.after) : undefined,
                    ),
                )
                .orderBy(asc(vault.resourceId))
                .limit(page.limit + 1);

            return page.result(rows, (row) => row.resourceId);
        });
    }

    /** Create secret metadata without assigning a value. */
    async create(
        input: VaultSelection & { requestId: string; name: string; tags?: Secret["tags"] },
        context: VaultContext,
    ): Promise<Secret> {
        return this.database.transaction(
            async (transaction) => {
                // verify creation authority and the provisioned parent vault
                await authorizeVault(
                    context,
                    { ...input, operation: "secret.create" },
                    transaction,
                );
                const existing = await transaction
                    .select()
                    .from(vault)
                    .where(
                        and(eq(vault.spaceId, input.spaceId), eq(vault.resourceId, input.vaultId)),
                    )
                    .get();
                if (!existing) {
                    throw new ServiceError("NOT_FOUND");
                }

                return this.retry("secret.create", input, context, transaction, async () => {
                    // reserve the name while holding the parent vault's write lock
                    await this.lockVault(input.vaultId, transaction);
                    const now = Date.now();
                    const id = identifier("secret").parse(`secret-${v7()}`);
                    const [row] = await transaction
                        .insert(secret)
                        .values({
                            id,
                            spaceId: input.spaceId,
                            vaultId: input.vaultId,
                            name: input.name,
                            tags: input.tags ?? {},
                            createdAt: now,
                            updatedAt: now,
                        })
                        .onConflictDoNothing()
                        .returning();
                    if (!row) {
                        throw new ServiceError("CONFLICT", {
                            message: "secret name already exists",
                        });
                    }

                    // commit creation and its audit record together
                    await this.record("secret.create", row, context, transaction);

                    return describeSecret(row);
                });
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Read metadata without retrieving encrypted value storage. */
    async get(input: SecretSelection, context: VaultContext): Promise<Secret> {
        return this.database.transaction(async (transaction) =>
            describeSecret(await this.load("secret.get", input, context, transaction)),
        );
    }

    /** List vault metadata in stable identifier order. */
    async list(input: VaultSelection & { limit?: number; cursor?: string }, context: VaultContext) {
        return this.database.transaction(async (transaction) => {
            // authorize the collection before selecting a bounded metadata page
            await authorizeVault(context, { ...input, operation: "secret.list" }, transaction);
            const page = new Page(
                input,
                ["secret", input.spaceId, input.vaultId],
                identifier("secret"),
            );
            const rows = await transaction
                .select()
                .from(secret)
                .where(
                    and(
                        eq(secret.spaceId, input.spaceId),
                        eq(secret.vaultId, input.vaultId),
                        page.after ? gt(secret.id, page.after) : undefined,
                    ),
                )
                .orderBy(asc(secret.id))
                .limit(page.limit + 1);

            return page.result(rows.map(describeSecret), (row) => row.id);
        });
    }

    /** Change metadata, availability, or the recoverable deletion state. */
    async change(
        operation: "update" | "disable" | "enable" | "delete" | "restore",
        input: SecretMutation & { name?: string; tags?: Secret["tags"] },
        context: VaultContext,
    ): Promise<Secret> {
        return this.mutate(`secret.${operation}`, input, context, async (row, transaction) => {
            // leave source-managed names and lifetimes to their declaring configuration
            if (
                (operation === "update" || operation === "delete") &&
                row.provenance !== null &&
                row.detachedAt === null
            ) {
                throw new ServiceError("SOURCE_MANAGED");
            }
            const now = Date.now();
            const update: {
                name?: string;
                tags?: Secret["tags"];
                disabledAt?: number | null;
                deleteAt?: number | null;
            } = {};

            // apply each state transition without silently restoring disabled access
            if (operation === "update") {
                if (input.name === undefined || input.tags === undefined) {
                    throw new ServiceError("BAD_REQUEST");
                }
                update.name = input.name;
                update.tags = input.tags;
                const duplicate = await transaction
                    .select({ id: secret.id })
                    .from(secret)
                    .where(and(eq(secret.vaultId, row.vaultId), eq(secret.name, input.name)))
                    .get();
                if (duplicate && duplicate.id !== row.id) {
                    throw new ServiceError("CONFLICT", { message: "secret name already exists" });
                }
            } else if (operation === "disable") {
                update.disabledAt = now;
            } else if (operation === "enable") {
                update.disabledAt = null;
            } else if (operation === "delete") {
                if (row.deleteAt !== null) {
                    throw new ServiceError("CONFLICT", {
                        message: "secret deletion is already scheduled",
                    });
                }
                update.deleteAt = now + this.recoveryMilliseconds;
            } else if (operation === "restore") {
                if (row.deleteAt === null || row.deleteAt <= now) {
                    throw new ServiceError("CONFLICT", { message: "secret cannot be restored" });
                }
                update.deleteAt = null;
            } else {
                operation satisfies never;
                throw new ServiceError("BAD_REQUEST", { message: "unknown secret operation" });
            }

            // preserve the revision increment performed while locking the secret
            const [changed] = await transaction
                .update(secret)
                .set(update)
                .where(eq(secret.id, row.id))
                .returning();

            return describeSecret(changed!);
        });
    }

    /** Destroy due values in bounded transactions under maintenance authority. */
    async purge(
        input: { spaceId: Secret["spaceId"]; limit: number },
        context: VaultContext,
    ): Promise<number> {
        const { limit, spaceId } = input;
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) {
            throw new ServiceError("BAD_REQUEST");
        }

        // authorize the selected space even when no records are due
        await this.database.transaction(async (transaction) => {
            await authorizeVault(context, { operation: "secret.purge", spaceId }, transaction);
        });

        // select bounded work within the caller's administering space
        const rows = await this.database
            .select()
            .from(secret)
            .where(
                and(
                    eq(secret.spaceId, spaceId),
                    isNotNull(secret.deleteAt),
                    isNull(secret.destroyedAt),
                    lte(secret.deleteAt, Date.now()),
                ),
            )
            .orderBy(asc(secret.id))
            .limit(limit);
        let removed = 0;

        // recheck each deadline and permission inside the destructive transaction
        for (const selected of rows) {
            await this.database.transaction(
                async (transaction) => {
                    const row = await this.load(
                        "secret.purge",
                        { spaceId: selected.spaceId, secretId: selected.id },
                        context,
                        transaction,
                    );
                    const [locked] = await transaction
                        .update(secret)
                        .set({ revision: sql`${secret.revision} + 1` })
                        .where(
                            and(
                                eq(secret.id, row.id),
                                eq(secret.revision, row.revision),
                                isNull(secret.destroyedAt),
                                lte(secret.deleteAt, Date.now()),
                            ),
                        )
                        .returning();
                    if (!locked) {
                        return;
                    }

                    // erase ciphertext and retain metadata tombstones with the audit event
                    await transaction.delete(vaultValue).where(eq(vaultValue.secretId, row.id));
                    await transaction
                        .update(secretVersion)
                        .set({ destroyedAt: Date.now() })
                        .where(
                            and(
                                eq(secretVersion.secretId, row.id),
                                sql`${secretVersion.destroyedAt} IS NULL`,
                            ),
                        );
                    await transaction
                        .update(secret)
                        .set({ destroyedAt: Date.now(), updatedAt: Date.now() })
                        .where(eq(secret.id, row.id));
                    await this.record("secret.purge", row, context, transaction);
                    removed++;
                },
                { isolationLevel: "read committed" },
            );
        }

        return removed;
    }

    /** Serialize a mutation, checking authority again before replaying any saved response. */
    async mutate<Result>(
        operation: keyof typeof secretAction,
        input: SecretMutation & { version?: number },
        context: VaultContext,
        change: (row: SecretRow, transaction: DatabaseConnection) => Promise<Result>,
    ): Promise<Result> {
        return this.database.transaction(
            async (transaction) => {
                const row = await this.load(operation, input, context, transaction);

                return this.retry(operation, input, context, transaction, async () => {
                    // serialize name changes before locking individual secret records
                    if (operation === "secret.update") {
                        await this.lockVault(row.vaultId, transaction);
                    }
                    if (
                        row.destroyedAt !== null ||
                        (row.deleteAt !== null && row.deleteAt <= Date.now())
                    ) {
                        throw new ServiceError("CONFLICT", {
                            message: "secret recovery period has ended",
                        });
                    }

                    // lock the observed revision before changing metadata or values
                    const [locked] = await transaction
                        .update(secret)
                        .set({ revision: sql`${secret.revision} + 1`, updatedAt: Date.now() })
                        .where(and(eq(secret.id, row.id), eq(secret.revision, input.revision)))
                        .returning();
                    if (!locked) {
                        throw new ServiceError("PRECONDITION_FAILED", {
                            message: "secret revision has changed",
                        });
                    }

                    // commit the change and audit event under the same transaction
                    const result = await change(locked, transaction);
                    await this.record(operation, locked, context, transaction, input.version);

                    return result;
                });
            },
            { isolationLevel: "read committed" },
        );
    }

    /** Serialize vault-local name allocation across creates and renames. */
    private async lockVault(
        vaultId: Secret["vaultId"],
        transaction: DatabaseConnection,
    ): Promise<void> {
        const rows = await transaction
            .update(vault)
            .set({ kind: "vault" })
            .where(eq(vault.resourceId, vaultId))
            .returning();
        if (rows.length !== 1) {
            throw new ServiceError("NOT_FOUND");
        }
    }

    /** Resolve the parent vault exclusively from persisted secret metadata. */
    async load(
        operation: VaultOperation,
        input: SecretSelection & { version?: number },
        context: VaultContext,
        transaction: DatabaseConnection,
    ): Promise<SecretRow> {
        const row = await transaction
            .select()
            .from(secret)
            .where(and(eq(secret.spaceId, input.spaceId), eq(secret.id, input.secretId)))
            .get();
        if (!row) {
            throw new ServiceError("NOT_FOUND");
        }
        await authorizeVault(
            context,
            {
                operation,
                spaceId: row.spaceId,
                vaultId: row.vaultId,
                secretId: row.id,
                version:
                    input.version ??
                    (operation === "version.read" ? (row.currentVersion ?? undefined) : undefined),
            },
            transaction,
        );

        return row;
    }

    /** Authenticate storage identity independently of caller-supplied routing. */
    encryptionContext(row: SecretRow, version: number): EncryptionContext {
        return {
            kind: "secret",
            location: this.location,
            spaceId: row.spaceId,
            vaultId: row.vaultId,
            secretId: row.id,
            version,
        };
    }

    /** Authenticate each request fingerprint within its caller, scope and procedure. */
    requestContext(request: RequestIdentity): EncryptionContext {
        return {
            kind: "request",
            location: this.location,
            spaceId: request.scope,
            caller: request.caller,
            procedure: request.procedure,
            requestId: request.requestId,
        };
    }

    /** Commit a domain audit event with its metadata change or plaintext read. */
    async record(
        operation: keyof typeof secretAction,
        row: SecretRow,
        context: VaultContext,
        transaction: DatabaseConnection,
        version?: number,
    ) {
        await context.audit.record(transaction, secretAction[operation], {
            targets: {
                space: { type: "space", id: row.spaceId },
                secret: { type: "secret", id: row.id },
            },
            details: version === undefined ? {} : { version },
            outcome: "success",
        });
    }

    /** Retain public responses and an encrypted digest, never the request's plaintext value. */
    private async retry<Result>(
        operation: VaultOperation,
        input: { requestId: string; spaceId: Secret["spaceId"] },
        context: VaultContext,
        transaction: DatabaseConnection,
        change: () => Promise<Result>,
    ): Promise<Result> {
        // protect fingerprints of secret values before writing the shared request journal
        const request = {
            caller: context.caller.id,
            scope: input.spaceId,
            procedure: operation,
            requestId: input.requestId,
        };
        const digest = await fingerprintRequest(input);
        const identity = this.requestContext(request);
        try {
            const encrypted = await this.encryption.encrypt(digest, identity);
            const requests = new IdempotencyStore(vaultRequest);
            const claim = await requests.begin(
                transaction,
                request,
                { digest: encrypted, keyId: encrypted.keyId },
                async (stored) => {
                    const original = await this.encryption.decrypt(
                        SecretEnvelope.parse(stored),
                        identity,
                    );
                    try {
                        return (
                            original.length === digest.length && timingSafeEqual(original, digest)
                        );
                    } finally {
                        original.fill(0);
                    }
                },
            );
            if (claim.kind === "replay") {
                return claim.value as Result;
            }

            // commit the public result alongside the protected mutation and domain audit
            const result = await change();
            await requests.complete(transaction, request, result);

            return result;
        } finally {
            digest.fill(0);
        }
    }
}

/** Persisted secret including source provenance. */
type SecretRow = typeof secret.$inferSelect;
/** A vault scoped to its administering space. */
export interface VaultSelection {
    /** Administering space. */
    spaceId: Secret["spaceId"];
    /** Provisioned vault resource. */
    vaultId: Secret["vaultId"];
}
/** A secret scoped to its administering space. */
export interface SecretSelection {
    /** Administering space. */
    spaceId: Secret["spaceId"];
    /** Immutable secret identity. */
    secretId: Secret["id"];
}
/** An immutable value selection. */
export interface VersionSelection extends SecretSelection {
    /** Exact immutable value version. */
    version: number;
}
/** Optimistic mutation with a caller retry identifier. */
export interface SecretMutation extends SecretSelection {
    /** Required current secret revision. */
    revision: number;
    /** Caller-scoped retry identity. */
    requestId: string;
}

/** Exclude provenance and implementation references from metadata responses. */
export function describeSecret(row: SecretRow): Secret {
    return Secret.strip().parse(row);
}

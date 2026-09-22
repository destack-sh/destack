import {
    and,
    asc,
    desc,
    eq,
    gt,
    lte,
    isNotNull,
    isNull,
    sql,
    type DatabaseConnection,
} from "@destack/db";
import { secret, secretVersion, vault } from "@destack/model/regional";
import { identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { v7 } from "uuid";
import { EnvelopeEncryption, type EncryptionContext, SecretEnvelope } from "../encryption/index.ts";
import { Secret, SecretVersion, SecretValue } from "../secret/index.ts";
import { vaultRequest, vaultValue } from "../stack/index.ts";
import { secretAction, requestRewrap } from "../audit/index.ts";
import type { VaultContext, VaultOperation } from "./context.ts";
import { Page } from "@destack/service/page";
import { IdempotencyStore } from "@destack/service/database";
import { fingerprintRequest, type RequestIdentity } from "@destack/service/request";
import { authorizeVault } from "./access.ts";
import { timingSafeEqual } from "node:crypto";

/** Minimum recoverable deletion interval. */
const DAY_MILLISECONDS = 86400000;
/** Maximum decoded secret value size. */
const MAX_VALUE_BYTES = 65536;

/** Managed secret persistence with transaction-bound access checks and audit. */
export class VaultStore {
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

    /** Append an immutable encrypted value and optionally select it as current. */
    async write(
        input: SecretMutation & { value: SecretValue; expiresAt?: number; promote?: boolean },
        context: VaultContext,
    ) {
        return this.mutate("version.write", input, context, async (row, transaction) => {
            // use one timestamp for expiry validation and persisted creation
            const now = Date.now();
            if (row.deleteAt !== null) {
                throw new ServiceError("CONFLICT", { message: "secret deletion is scheduled" });
            }
            if (input.expiresAt !== undefined && input.expiresAt <= now) {
                throw new ServiceError("BAD_REQUEST", {
                    message: "secret expiry must be in the future",
                });
            }

            // assign the next version while holding the secret's write lock
            const previous = await transaction
                .select()
                .from(secretVersion)
                .where(eq(secretVersion.secretId, row.id))
                .orderBy(desc(secretVersion.version))
                .get();
            const version = (previous?.version ?? 0) + 1;

            // bound decoded bytes before encrypting the transport representation
            const value = SecretValue.parse(input.value);
            const bytes =
                value.encoding === "text"
                    ? new TextEncoder().encode(value.value)
                    : Uint8Array.fromBase64(value.value);
            const size = bytes.byteLength;
            bytes.fill(0);
            if (size > MAX_VALUE_BYTES) {
                throw new ServiceError("PAYLOAD_TOO_LARGE");
            }
            const plaintext = new TextEncoder().encode(JSON.stringify(value));
            let envelope: SecretEnvelope;
            try {
                envelope = await this.encryption.encrypt(
                    plaintext,
                    this.encryptionContext(row, version),
                );
            } finally {
                plaintext.fill(0);
            }

            // persist ciphertext and public metadata in the same transaction
            const [created] = await transaction
                .insert(secretVersion)
                .values({
                    secretId: row.id,
                    version,
                    createdAt: now,
                    expiresAt: input.expiresAt ?? null,
                })
                .returning();
            await transaction.insert(vaultValue).values({ secretId: row.id, version, ...envelope });
            if (input.promote !== false) {
                await transaction
                    .update(secret)
                    .set({ currentVersion: version })
                    .where(eq(secret.id, row.id));
                row.currentVersion = version;
            }

            return { secret: describeSecret(row), version: describeVersion(created!) };
        });
    }

    /** Read exact version metadata. */
    async getVersion(input: VersionSelection, context: VaultContext): Promise<SecretVersion> {
        return this.database.transaction(async (transaction) => {
            await this.load("version.get", input, context, transaction);

            return describeVersion(
                await this.loadVersion(input.secretId, input.version, transaction),
            );
        });
    }

    /** List immutable version metadata without decrypting values. */
    async listVersions(
        input: SecretSelection & { limit?: number; cursor?: string },
        context: VaultContext,
    ) {
        return this.database.transaction(async (transaction) => {
            // authorize the parent secret before enumerating its immutable versions
            await this.load("version.list", input, context, transaction);
            const page = new Page(
                input,
                ["version", input.spaceId, input.secretId],
                schema.number().int().positive(),
            );
            const rows = await transaction
                .select()
                .from(secretVersion)
                .where(
                    and(
                        eq(secretVersion.secretId, input.secretId),
                        page.after ? gt(secretVersion.version, page.after) : undefined,
                    ),
                )
                .orderBy(asc(secretVersion.version))
                .limit(page.limit + 1);

            return page.result(rows.map(describeVersion), (row) => row.version);
        });
    }

    /** Authorize and durably audit plaintext access before returning its result. */
    async read(input: SecretSelection & { version?: number }, context: VaultContext) {
        return this.database.transaction(
            async (transaction) => {
                // check authority before obtaining or decrypting a value
                const row = await this.load("version.read", input, context, transaction);
                if (row.disabledAt !== null || row.deleteAt !== null) {
                    throw new ServiceError("FORBIDDEN", { message: "secret is unavailable" });
                }
                const number = input.version ?? row.currentVersion;
                if (number === null) {
                    throw new ServiceError("NOT_FOUND", {
                        message: "secret has no current version",
                    });
                }
                const version = await this.loadVersion(row.id, number, transaction);
                requireReadable(version);

                // authenticate ciphertext before recording successful disclosure
                const stored = await transaction
                    .select()
                    .from(vaultValue)
                    .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, number)))
                    .get();
                if (!stored) {
                    throw new ServiceError("INTERNAL_SERVER_ERROR", {
                        message: "secret ciphertext is missing",
                    });
                }
                const plaintext = await this.encryption.decrypt(
                    envelopeFrom(stored),
                    this.encryptionContext(row, number),
                );
                try {
                    const value = SecretValue.parse(
                        JSON.parse(new TextDecoder().decode(plaintext)),
                    );
                    await this.record("version.read", row, context, transaction, number);

                    return { version: describeVersion(version), value };
                } finally {
                    plaintext.fill(0);
                }
            },
            { isolationLevel: "serializable" },
        );
    }

    /** Select a usable version as current without changing its value. */
    async promote(
        input: SecretMutation & { version: number },
        context: VaultContext,
    ): Promise<Secret> {
        return this.mutate("version.promote", input, context, async (row, transaction) => {
            // require a retained secret and a readable destination version
            if (row.deleteAt !== null) {
                throw new ServiceError("CONFLICT");
            }
            requireReadable(await this.loadVersion(row.id, input.version, transaction));

            // change only the current selection, preserving immutable value history
            await transaction
                .update(secret)
                .set({ currentVersion: input.version })
                .where(eq(secret.id, row.id));

            return describeSecret({ ...row, currentVersion: input.version });
        });
    }

    /** Disable, enable, or irreversibly destroy one value version. */
    async changeVersion(
        operation: "disable" | "enable" | "destroy",
        input: SecretMutation & { version: number },
        context: VaultContext,
    ): Promise<SecretVersion> {
        return this.mutate(`version.${operation}`, input, context, async (row, transaction) => {
            // retain irreversible destruction when changing version availability
            const version = await this.loadVersion(row.id, input.version, transaction);
            if (version.destroyedAt !== null) {
                throw new ServiceError("CONFLICT", { message: "secret version is destroyed" });
            }
            const now = Date.now();

            // erase ciphertext while retaining its immutable history entry
            if (operation === "destroy") {
                await transaction
                    .delete(vaultValue)
                    .where(
                        and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)),
                    );
            }
            const [changed] = await transaction
                .update(secretVersion)
                .set(
                    operation === "destroy"
                        ? { destroyedAt: now }
                        : { disabledAt: operation === "disable" ? now : null },
                )
                .where(
                    and(
                        eq(secretVersion.secretId, row.id),
                        eq(secretVersion.version, input.version),
                    ),
                )
                .returning();

            return describeVersion(changed!);
        });
    }

    /** Rewrap one value under the active root key, preserving its version. */
    async rewrap(
        input: SecretMutation & { version: number },
        context: VaultContext,
    ): Promise<SecretVersion> {
        return this.mutate("version.rewrap", input, context, async (row, transaction) => {
            // load the exact envelope under the secret's optimistic write lock
            const version = await this.loadVersion(row.id, input.version, transaction);
            const stored = await transaction
                .select()
                .from(vaultValue)
                .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)))
                .get();
            if (!stored) {
                throw new ServiceError("NOT_FOUND");
            }

            // replace key protection while preserving the value and version
            const envelope = await this.encryption.rewrap(
                envelopeFrom(stored),
                this.encryptionContext(row, input.version),
            );
            await transaction
                .update(vaultValue)
                .set(envelope)
                .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)));

            return describeVersion(version);
        });
    }

    /** Rewrap a bounded batch of retry digests before retiring an old root key. */
    async rewrapRequests(
        input: { spaceId: Secret["spaceId"]; keyId: string; limit: number },
        context: VaultContext,
    ): Promise<number> {
        // bound each rewrapping transaction
        const { spaceId, keyId, limit } = input;
        if (!Number.isInteger(limit) || limit < 1 || limit > 1000) {
            throw new ServiceError("BAD_REQUEST");
        }

        return this.database.transaction(
            async (transaction) => {
                // authorize the entire space before inspecting protected retry records
                await authorizeVault(
                    context,
                    { operation: "request.rewrap", spaceId },
                    transaction,
                );

                // select only records still protected by the retiring key
                const rows = await transaction
                    .select()
                    .from(vaultRequest)
                    .where(and(eq(vaultRequest.scope, spaceId), eq(vaultRequest.keyId, keyId)))
                    .limit(limit);

                // preserve request identity while replacing root-key protection
                for (const row of rows) {
                    const identity = this.requestContext(row);
                    const envelope = await this.encryption.rewrap(
                        SecretEnvelope.parse(row.digest),
                        identity,
                    );
                    if (envelope.keyId === keyId) {
                        throw new ServiceError("CONFLICT", {
                            message: "replacement root key must differ",
                        });
                    }

                    // commit each replacement with its audit event
                    await transaction
                        .update(vaultRequest)
                        .set({ keyId: envelope.keyId, digest: envelope })
                        .where(
                            and(
                                eq(vaultRequest.caller, row.caller),
                                eq(vaultRequest.scope, row.scope),
                                eq(vaultRequest.procedure, row.procedure),
                                eq(vaultRequest.requestId, row.requestId),
                                eq(vaultRequest.keyId, keyId),
                            ),
                        );
                    await context.audit.record(transaction, requestRewrap, {
                        targets: { space: { type: "space", id: spaceId } },
                        details: { previousKeyId: keyId, keyId: envelope.keyId, count: 1 },
                        outcome: "success",
                    });
                }

                return rows.length;
            },
            { isolationLevel: "serializable" },
        );
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
    private async mutate<Result>(
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
    private async load(
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

    /** Read an exact immutable version under an already authorized secret. */
    private async loadVersion(
        secretId: Secret["id"],
        version: number,
        transaction: DatabaseConnection,
    ) {
        const row = await transaction
            .select()
            .from(secretVersion)
            .where(and(eq(secretVersion.secretId, secretId), eq(secretVersion.version, version)))
            .get();
        if (!row) {
            throw new ServiceError("NOT_FOUND");
        }

        return row;
    }

    /** Authenticate storage identity independently of caller-supplied routing. */
    private encryptionContext(row: SecretRow, version: number): EncryptionContext {
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
    private requestContext(request: RequestIdentity): EncryptionContext {
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
    private async record(
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
function describeSecret(row: SecretRow): Secret {
    return Secret.strip().parse(row);
}
/** Exclude backend references from version responses. */
function describeVersion(row: typeof secretVersion.$inferSelect): SecretVersion {
    return SecretVersion.strip().parse(row);
}
/** Reject disabled, destroyed and expired values before decryption. */
function requireReadable(row: typeof secretVersion.$inferSelect): void {
    if (
        row.destroyedAt !== null ||
        row.disabledAt !== null ||
        (row.expiresAt !== null && row.expiresAt <= Date.now())
    ) {
        throw new ServiceError("FORBIDDEN", { message: "secret version is unavailable" });
    }
}
/** Reject unknown encrypted formats instead of interpreting them as current. */
function envelopeFrom(row: typeof vaultValue.$inferSelect): SecretEnvelope {
    if (row.format !== 1) {
        throw new ServiceError("INTERNAL_SERVER_ERROR", {
            message: "unsupported secret encryption format",
        });
    }

    return { ...row, format: 1, keyNonce: row.keyNonce ?? undefined };
}

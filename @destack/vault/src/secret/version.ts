import { and, asc, desc, eq, gt, type DatabaseConnection } from "@destack/db";
import { secret, secretVersion } from "@destack/model/space";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Page } from "@destack/service/page";
import { SecretEnvelope } from "../encryption/index.ts";
import { vaultValue } from "../stack/index.ts";
import { Secret, SecretVersion, SecretValue } from "./secret.ts";
import {
    Vault,
    describeSecret,
    type SecretSelection,
    type SecretMutation,
    type VersionSelection,
} from "../vault/vault.ts";
import type { VaultContext } from "../vault/context.ts";

/** Maximum decoded secret value size. */
const MAX_VALUE_BYTES = 65536;

/** Append an immutable encrypted value and optionally select it as current. */
export async function writeVersion(
    vault: Vault,
    input: SecretMutation & { value: SecretValue; expiresAt?: number; promote?: boolean },
    context: VaultContext,
) {
    return vault.mutate("version.write", input, context, async (row, transaction) => {
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
            envelope = await vault.encryption.encrypt(
                plaintext,
                vault.encryptionContext(row, version),
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
export async function getVersion(
    vault: Vault,
    input: VersionSelection,
    context: VaultContext,
): Promise<SecretVersion> {
    return vault.database.transaction(async (transaction) => {
        await vault.load("version.get", input, context, transaction);

        return describeVersion(await loadVersion(input.secretId, input.version, transaction));
    });
}

/** List immutable version metadata without decrypting values. */
export async function listVersions(
    vault: Vault,
    input: SecretSelection & { limit?: number; cursor?: string },
    context: VaultContext,
) {
    return vault.database.transaction(async (transaction) => {
        // authorize the parent secret before enumerating its immutable versions
        await vault.load("version.list", input, context, transaction);
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
export async function readVersion(
    vault: Vault,
    input: SecretSelection & { version?: number },
    context: VaultContext,
) {
    return vault.database.transaction(
        async (transaction) => {
            // check authority before obtaining or decrypting a value
            const row = await vault.load("version.read", input, context, transaction);
            if (row.disabledAt !== null || row.deleteAt !== null) {
                throw new ServiceError("FORBIDDEN", { message: "secret is unavailable" });
            }
            const number = input.version ?? row.currentVersion;
            if (number === null) {
                throw new ServiceError("NOT_FOUND", {
                    message: "secret has no current version",
                });
            }
            const version = await loadVersion(row.id, number, transaction);
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
            const plaintext = await vault.encryption.decrypt(
                envelopeFrom(stored),
                vault.encryptionContext(row, number),
            );
            try {
                const value = SecretValue.parse(JSON.parse(new TextDecoder().decode(plaintext)));
                await vault.record("version.read", row, context, transaction, number);

                return { version: describeVersion(version), value };
            } finally {
                plaintext.fill(0);
            }
        },
        { isolationLevel: "serializable" },
    );
}

/** Select a usable version as current without changing its value. */
export async function promoteVersion(
    vault: Vault,
    input: SecretMutation & { version: number },
    context: VaultContext,
): Promise<Secret> {
    return vault.mutate("version.promote", input, context, async (row, transaction) => {
        // require a retained secret and a readable destination version
        if (row.deleteAt !== null) {
            throw new ServiceError("CONFLICT");
        }
        requireReadable(await loadVersion(row.id, input.version, transaction));

        // change only the current selection, preserving immutable value history
        await transaction
            .update(secret)
            .set({ currentVersion: input.version })
            .where(eq(secret.id, row.id));

        return describeSecret({ ...row, currentVersion: input.version });
    });
}

/** Disable, enable, or irreversibly destroy one value version. */
export async function changeVersion(
    vault: Vault,
    operation: "disable" | "enable" | "destroy",
    input: SecretMutation & { version: number },
    context: VaultContext,
): Promise<SecretVersion> {
    return vault.mutate(`version.${operation}`, input, context, async (row, transaction) => {
        // retain irreversible destruction when changing version availability
        const version = await loadVersion(row.id, input.version, transaction);
        if (version.destroyedAt !== null) {
            throw new ServiceError("CONFLICT", { message: "secret version is destroyed" });
        }
        const now = Date.now();

        // erase ciphertext while retaining its immutable history entry
        if (operation === "destroy") {
            await transaction
                .delete(vaultValue)
                .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)));
        }
        const [changed] = await transaction
            .update(secretVersion)
            .set(
                operation === "destroy"
                    ? { destroyedAt: now }
                    : { disabledAt: operation === "disable" ? now : null },
            )
            .where(
                and(eq(secretVersion.secretId, row.id), eq(secretVersion.version, input.version)),
            )
            .returning();

        return describeVersion(changed!);
    });
}

/** Rewrap one value under the active root key, preserving its version. */
export async function rewrapVersion(
    vault: Vault,
    input: SecretMutation & { version: number },
    context: VaultContext,
): Promise<SecretVersion> {
    return vault.mutate("version.rewrap", input, context, async (row, transaction) => {
        // load the exact envelope under the secret's optimistic write lock
        const version = await loadVersion(row.id, input.version, transaction);
        const stored = await transaction
            .select()
            .from(vaultValue)
            .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)))
            .get();
        if (!stored) {
            throw new ServiceError("NOT_FOUND");
        }

        // replace key protection while preserving the value and version
        const envelope = await vault.encryption.rewrap(
            envelopeFrom(stored),
            vault.encryptionContext(row, input.version),
        );
        await transaction
            .update(vaultValue)
            .set(envelope)
            .where(and(eq(vaultValue.secretId, row.id), eq(vaultValue.version, input.version)));

        return describeVersion(version);
    });
}

/** Read an exact immutable version under an already authorized secret. */
async function loadVersion(
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

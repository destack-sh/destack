import { identifier, schema } from "@destack/schema";

/** Mutable metadata of an independently versioned secret. */
export const Secret = schema.object({
    /** Immutable identity. */
    id: identifier("secret"),
    /** Administering space. */
    spaceId: identifier("space"),
    /** Vault resource holding the value. */
    vaultId: identifier("resource"),
    /** Vault-local display name. */
    name: schema.string().min(1).max(256),
    /** Creation time in UTC epoch milliseconds. */
    createdAt: schema.number().int(),
    /** Last metadata change in UTC epoch milliseconds. */
    updatedAt: schema.number().int(),
    /** Optimistic concurrency revision. */
    revision: schema.number().int().positive(),
    /** User-defined labels, excluding secret values. */
    tags: schema.record(schema.string().max(128), schema.string().max(256)),
    /** Version selected by unpinned reads. */
    currentVersion: schema.number().int().positive().nullable(),
    /** Explicit disabling time. */
    disabledAt: schema.number().int().nullable(),
    /** Scheduled destruction time. */
    deleteAt: schema.number().int().nullable(),
    /** Completion time of scheduled value destruction. */
    destroyedAt: schema.number().int().nullable(),
});
/** Public secret metadata. */
export type Secret = schema.Infer<typeof Secret>;

/** Metadata of an immutable value version. */
export const SecretVersion = schema.object({
    /** Immutable secret identity. */
    secretId: identifier("secret"),
    /** Monotonically increasing version. */
    version: schema.number().int().positive(),
    /** Creation time in UTC epoch milliseconds. */
    createdAt: schema.number().int(),
    /** Time after which reads are rejected. */
    expiresAt: schema.number().int().nullable(),
    /** Explicit disabling time. */
    disabledAt: schema.number().int().nullable(),
    /** Destruction time retained after removing ciphertext. */
    destroyedAt: schema.number().int().nullable(),
});
/** Public version metadata. */
export type SecretVersion = schema.Infer<typeof SecretVersion>;

/** A bounded text or binary value; excluded from logs and metadata. */
export const SecretValue = schema.discriminatedUnion("encoding", [
    schema.object({ encoding: schema.literal("text"), value: schema.string().max(65536) }),
    schema.object({ encoding: schema.literal("base64"), value: schema.base64().max(87384) }),
]);
/** A secret value crossing the authenticated service transport. */
export type SecretValue = schema.Infer<typeof SecretValue>;

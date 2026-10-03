import { schema } from "@destack/schema";

/** The largest decoded secret value: 64 KiB, the limit of common managed secret stores. */
export const MAX_VALUE_BYTES = 64 * 1024;

/** The longest base64 text of the largest value: four characters per three bytes. */
const MAX_BASE64_LENGTH = Math.ceil(MAX_VALUE_BYTES / 3) * 4;

/** A bounded text or binary value; excluded from logs and metadata. */
export const SecretValue = schema.discriminatedUnion("encoding", [
    schema.object({
        encoding: schema.literal("text"),
        value: schema.string().max(MAX_VALUE_BYTES),
    }),
    schema.object({
        encoding: schema.literal("base64"),
        value: schema.base64().max(MAX_BASE64_LENGTH),
    }),
]);
/** A secret value crossing the authenticated service transport. */
export type SecretValue = schema.Infer<typeof SecretValue>;

/** A value read from a secret, with the number of its version. */
export const SecretReading = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive(),
    /** The decrypted value. */
    value: SecretValue,
});
/** A value read from a secret, with the number of its version. */
export type SecretReading = schema.Infer<typeof SecretReading>;

/** What the audit of a read records: the number of the version disclosed. */
export const SecretDisclosure = SecretReading.pick({ version: true });

/** The value of a new version, and whether it becomes its secret's current version. */
export const VersionWrite = schema.object({
    /** The value to encrypt. */
    value: schema.sensitive(SecretValue),
    /** Whether the version becomes current, true when absent. */
    promote: schema.boolean().exactOptional(),
});
/** The value of a new version, and whether it becomes its secret's current version. */
export type VersionWrite = schema.Infer<typeof VersionWrite>;

/** The version a promotion selects as its secret's current one. */
export const SecretPromotion = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive(),
});
/** The version a promotion selects as its secret's current one. */
export type SecretPromotion = schema.Infer<typeof SecretPromotion>;

/** The version a read selects: an exact number, or the secret's current version when absent. */
export const SecretSelection = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive().exactOptional(),
});
/** The version a read selects: an exact number, or the secret's current version when absent. */
export type SecretSelection = schema.Infer<typeof SecretSelection>;

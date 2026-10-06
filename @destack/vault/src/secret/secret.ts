import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";

/** The largest decoded secret value: 64 KiB, the limit of common managed secret stores. */
export const MAX_VALUE_BYTES = 64 * 1024;

/** The longest base64 text of the largest value: four characters per three bytes. */
const MAX_BASE64_LENGTH = Math.ceil(MAX_VALUE_BYTES / 3) * 4;

/** The algorithm of generated ES256 keys: ECDSA over P-256 (RFC 7518 3.4). */
const ES256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** The algorithms a space generates a secret's first version with: ES256 writes an ECDSA P-256 private key as a JWK (RFC 7518 6.2). */
export const SECRET_ALGORITHMS = ["ES256"] as const;

/** An algorithm a space generates a secret's first version with. */
export type SecretAlgorithm = (typeof SECRET_ALGORITHMS)[number];

/** How a space generates a secret's first version when it installs the package: in one of the package's vaults, with an algorithm. */
export const SecretGeneration = defineSchema(
    schema.object({
        /** The package's vault declaration keeping the secret. */
        vault: DeclarationName,
        /** The algorithm writing the first version. */
        algorithm: schema.enum(SECRET_ALGORITHMS),
    }),
);
/** How a space generates a secret's first version when it installs the package. */
export type SecretGeneration = schema.Infer<typeof SecretGeneration>;

/** A bounded text or binary value; excluded from logs and metadata. */
const secretValueSchema = schema.discriminatedUnion("encoding", [
    schema.object({
        encoding: schema.literal("text"),
        value: schema.string().max(MAX_VALUE_BYTES),
    }),
    schema.object({
        encoding: schema.literal("base64"),
        value: schema.base64().max(MAX_BASE64_LENGTH),
    }),
]);
/** A bounded text or binary value; excluded from logs and metadata. */
export const SecretValue = Object.assign(secretValueSchema, {
    /** Generate a value with an algorithm: ES256 writes an ECDSA P-256 private key as a JWK (RFC 7518 6.2). */
    async generate(algorithm: SecretAlgorithm): Promise<SecretValue> {
        const keys = await crypto.subtle.generateKey(ES256, true, ["sign", "verify"]);
        const jwk = await crypto.subtle.exportKey("jwk", keys.privateKey);

        return { encoding: "text", value: JSON.stringify({ ...jwk, alg: algorithm }) };
    },
});
/** A secret value crossing the authenticated service transport. */
export type SecretValue = schema.Infer<typeof secretValueSchema>;

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

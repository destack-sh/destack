import { DeclarationName } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";

/** The largest decoded secret value: 64 KiB, the limit of common managed secret stores. */
export const MAX_VALUE_BYTES = 64 * 1024;

/** The longest base64 text of the largest value: four characters per three bytes. */
const MAX_BASE64_LENGTH = Math.ceil(MAX_VALUE_BYTES / 3) * 4;

/** The algorithms a space generates a secret's first version with. */
export const SECRET_ALGORITHMS = ["ES256"] as const;

/** An algorithm a space generates a secret's first version with. */
export type SecretAlgorithm = (typeof SECRET_ALGORITHMS)[number];

/** The algorithm of generated ES256 keys: ECDSA over P-256 (RFC 7518 3.4). */
const ES256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** How a space generates a secret's first version when it installs the package. */
export const SecretGeneration = Object.assign(
    defineSchema(
        schema.object({
            /** The package's vault declaration keeping the secret. */
            vault: DeclarationName,
            /** The algorithm writing the first version. */
            algorithm: schema.enum(SECRET_ALGORITHMS),
        }),
    ),
    {
        /** Generate a secret's first value with an algorithm: a private JSON Web Key. */
        async generate(algorithm: SecretAlgorithm): Promise<SecretValue> {
            const keys = await crypto.subtle.generateKey(ES256, true, ["sign", "verify"]);
            const jwk = await crypto.subtle.exportKey("jwk", keys.privateKey);

            return { encoding: "text", value: JSON.stringify({ ...jwk, alg: algorithm }) };
        },
    },
);
/** How a space generates a secret's first version when it installs the package. */
export type SecretGeneration = schema.Infer<typeof SecretGeneration>;

/** A bounded text or binary secret value. */
export const SecretValue = defineSchema(
    schema.discriminatedUnion("encoding", [
        schema.object({
            encoding: schema.literal("text"),
            value: schema.string().max(MAX_VALUE_BYTES),
        }),
        schema.object({
            encoding: schema.literal("base64"),
            value: schema.base64().max(MAX_BASE64_LENGTH),
        }),
    ]),
);
/** A bounded text or binary secret value. */
export type SecretValue = schema.Infer<typeof SecretValue>;

/** A value read from a secret, with the number of its version. */
export const SecretReading = defineSchema(
    schema.object({
        /** The version's number within its secret. */
        version: schema.number().int().positive(),
        /** The decrypted value. */
        value: SecretValue,
    }),
);
/** A value read from a secret, with the number of its version. */
export type SecretReading = schema.Infer<typeof SecretReading>;

/** The version a read selects: an exact number, or the secret's current version when absent. */
export const SecretSelection = defineSchema(
    schema.object({
        /** The version's number within its secret. */
        version: schema.number().int().positive().exactOptional(),
    }),
);
/** The version a read selects: an exact number, or the secret's current version when absent. */
export type SecretSelection = schema.Infer<typeof SecretSelection>;

/** The audit details of a read: the number of the version it disclosed. */
export const SecretDisclosure = SecretReading.pick({ version: true });

/** A secret a read selects by name within its vault, at an exact version or the current one. */
export const VaultSelection = defineSchema(
    SecretSelection.extend({
        /** The secret's name within the vault. */
        name: schema.string().min(1),
    }),
);
/** A secret a read selects by name within its vault, at an exact version or the current one. */
export type VaultSelection = schema.Infer<typeof VaultSelection>;

/** A secret version a workload's deployment captured, as its code reads it. */
export interface Secret {
    /** Read the captured version's value. */
    readonly read: () => Promise<SecretReading>;
}

/** Secrets as code reads them. */
export const Secret = {
    /** Read a text value, such as an operator's environment variable, as a secret's one version. */
    of(value: string): Secret {
        return { read: async () => ({ version: 1, value: { encoding: "text", value } }) };
    },

    /** Read a secret's value as text, refusing a binary one. */
    async text(secret: Secret): Promise<string> {
        const { value } = await secret.read();
        if (value.encoding !== "text") {
            throw new TypeError(`the secret is ${value.encoding}, not text`);
        }

        return value.value;
    },
};

/** A vault a workload binds, as its code reads its secrets. */
export interface Vault {
    /** Read a version of the vault's secret of a name, refusing a secret of any other vault. */
    readonly read: (name: string, selection?: SecretSelection) => Promise<SecretReading>;
}

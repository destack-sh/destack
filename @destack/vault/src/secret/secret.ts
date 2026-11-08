import type { Crypto } from "@destack/environment";
import { DeclarationName } from "@destack/package";
import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { SecretReading, type SecretValue } from "@destack/resource";

/** The algorithms generating a secret's values: a signing key. */
export const SECRET_ALGORITHMS = ["ES256"] as const;

/** An algorithm a space generates a secret's first version with. */
export type SecretAlgorithm = (typeof SECRET_ALGORITHMS)[number];

/** The algorithm of generated ES256 keys: ECDSA over P-256 (RFC 7518 3.4). */
const ES256 = { name: "ECDSA", namedCurve: "P-256" } as const;

/** How a space generates a secret's first version when it installs the package. */
export const SecretGeneration = Object.assign(
    schema.object({
        /** The package's vault declaration keeping the secret. */
        vault: DeclarationName,
        /** The algorithm writing the first version. */
        algorithm: schema.enum(SECRET_ALGORITHMS),
    }),
    {
        /** Generate a value with an algorithm: a private JSON Web Key. */
        async generate(algorithm: SecretAlgorithm, crypto: Crypto): Promise<SecretValue> {
            const keys = await crypto.generateKey(ES256, true, ["sign", "verify"]);
            const jwk = await crypto.exportKey("jwk", keys.privateKey);

            return { encoding: "text", value: JSON.stringify({ ...jwk, alg: algorithm }) };
        },
    },
);
/** How a space generates a secret's first version when it installs the package. */
export type SecretGeneration = schema.Infer<typeof SecretGeneration>;

/** The version a read selects: an exact number, or the secret's current version when absent. */
export const SecretSelection = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive().exactOptional(),
});
/** The version a read selects: an exact number, or the secret's current version when absent. */
export type SecretSelection = schema.Infer<typeof SecretSelection>;

/** The audit details of a read: the number of the version it disclosed. */
export const SecretDisclosure = SecretReading.pick({ version: true });

/** A secret a read selects by name within its vault, at an exact version or the current one. */
export const VaultSelection = SecretSelection.extend({
    /** The secret's name within the vault. */
    name: schema.string().min(1),
});
/** A secret a read selects by name within its vault, at an exact version or the current one. */
export type VaultSelection = schema.Infer<typeof VaultSelection>;

/** A vault a workload binds, as its code reads its secrets. */
export interface Vault {
    /** The vault object the binding names, which its package's code puts secrets into. */
    readonly reference: ObjectReference;
    /** Read a version of the vault's secret of a name, refusing a secret of any other vault. */
    readonly read: (name: string, selection?: SecretSelection) => Promise<SecretReading>;
}

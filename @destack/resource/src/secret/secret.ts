import { schema } from "@destack/schema";
import { ResourceError } from "../error/error.ts";

/** The largest decoded secret value: 64 KiB, the limit of common managed secret stores. */
export const MAX_VALUE_BYTES = 64 * 1024;

/** The longest base64 text of the largest value: four characters per three bytes. */
const MAX_BASE64_LENGTH = Math.ceil(MAX_VALUE_BYTES / 3) * 4;

/** A bounded text or binary secret value. */
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
/** A bounded text or binary secret value. */
export type SecretValue = schema.Infer<typeof SecretValue>;

/** A value read from a secret, with the number of its version. */
const secretReading = schema.object({
    /** The version's number within its secret. */
    version: schema.number().int().positive(),
    /** The decrypted value. */
    value: SecretValue,
});

/** A value read from a secret, with the number of its version. */
export const SecretReading = Object.assign(secretReading, {
    /** Read a reading's value as text, refusing a binary one. */
    text(reading: SecretReading): string {
        if (reading.value.encoding !== "text") {
            throw ResourceError.of(
                "TEXT_REQUIRED",
                `text secret is required: the secret is '${reading.value.encoding}'`,
            );
        }

        return reading.value.value;
    },
});
/** A value read from a secret, with the number of its version. */
export type SecretReading = schema.Infer<typeof secretReading>;

/** A secret version a workload's deployment captured, as its code reads it. */
export interface Secret {
    /** Read the captured version's value. */
    readonly read: () => Promise<SecretReading>;
    /** Read the captured version and the earlier versions still readable, newest first. */
    readonly versions: () => Promise<readonly [SecretReading, ...SecretReading[]]>;
}

/** Secrets as code reads them. */
export const Secret = {
    /** Read a text value, such as an operator's environment variable, as a secret's one version. */
    of(value: string): Secret {
        const reading: SecretReading = { version: 1, value: { encoding: "text", value } };

        return { read: async () => reading, versions: async () => [reading] };
    },
};

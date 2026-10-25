import { gcmsiv } from "@noble/ciphers/aes.js";
import { schema } from "@destack/schema";
import type { Random } from "@destack/environment";
import type { Sealing } from "../table/column.ts";
import type { Dialect } from "../dialect/dialect.ts";

/** The label the column key derives under from a database's identity root. */
const COLUMN_LABEL = { label: "@destack/db/column", context: "" } as const;

/** The version prefix of sealed values, so a later format reads beside this one. */
const SEALED_PREFIX = "v1.";

/** The bytes of a random nonce, which AES-GCM-SIV also survives repeating (RFC 8452). */
const NONCE_BYTES = 12;

/** The dialect of a sealed value's text form, as sealed columns store text. */
export const SEALED_DIALECT: Dialect = "sqlite";

/** The values a driver takes for a sealed column, stored as their JSON text. */
const SealedValue = schema.union([schema.string(), schema.number(), schema.boolean()]);

/** Encrypts a database's sensitive columns at rest with AES-GCM-SIV (RFC 8452). */
export class ColumnCipher {
    /** The 256-bit column key. */
    readonly #key: Uint8Array<ArrayBuffer>;
    /** The randomness the random nonces draw from. */
    readonly #random: Random;

    /** Encrypt under a 256-bit key, drawing nonces from some randomness. */
    constructor(key: Uint8Array<ArrayBuffer>, random: Random) {
        this.#key = key;
        this.#random = random;
    }

    /** Derive the column key from the identity root of the database's scope. */
    static async derive(
        root: {
            readonly derive: (label: {
                readonly label: string;
                readonly context: string;
            }) => Promise<Uint8Array<ArrayBuffer>>;
        },
        random: Random,
    ): Promise<ColumnCipher> {
        return new ColumnCipher(await root.derive(COLUMN_LABEL), random);
    }

    /** Seal a column's driver value as text bound to the column. */
    seal(value: unknown, column: string, sealing: Sealing): string {
        // encrypt the value's JSON under a fresh or fixed nonce and the column's context
        const plaintext = new TextEncoder().encode(JSON.stringify(SealedValue.parse(value)));
        const nonce =
            sealing === "random" ? this.#random.bytes(NONCE_BYTES) : new Uint8Array(NONCE_BYTES);
        const ciphertext = ColumnCipher.#aead(this.#key, nonce, column).encrypt(plaintext);
        const sealed = new Uint8Array([...nonce, ...ciphertext]);

        return `${SEALED_PREFIX}${sealed.toBase64({ alphabet: "base64url", omitPadding: true })}`;
    }

    /** Open a sealed column value back into its driver value. */
    open(sealed: unknown, column: string): unknown {
        // require the sealed format
        const text = schema.string().parse(sealed);
        if (!text.startsWith(SEALED_PREFIX)) {
            throw new TypeError(`${column} contains a value in an unknown encrypted format`);
        }

        // decrypt under the column's context and read the JSON
        const bytes = Uint8Array.fromBase64(text.slice(SEALED_PREFIX.length), {
            alphabet: "base64url",
        });
        const nonce = bytes.subarray(0, NONCE_BYTES);
        const plaintext = ColumnCipher.#decrypt(
            this.#key,
            nonce,
            column,
            bytes.subarray(NONCE_BYTES),
        );

        return SealedValue.parse(JSON.parse(new TextDecoder().decode(plaintext)));
    }

    /** Require the cipher of a sensitive column, which a database declaring one opens with. */
    static require(
        cipher: ColumnCipher | undefined,
        column: { readonly definition: { readonly name: string } },
    ): ColumnCipher {
        if (cipher === undefined) {
            throw new TypeError(
                `cannot encrypt sensitive column ${column.definition.name}: the database has no key`,
            );
        }

        return cipher;
    }

    /** Decrypt a sealed value, refusing one sealed under another key or for another column. */
    static #decrypt(
        key: Uint8Array<ArrayBuffer>,
        nonce: Uint8Array,
        column: string,
        ciphertext: Uint8Array,
    ): Uint8Array {
        try {
            return ColumnCipher.#aead(key, nonce, column).decrypt(ciphertext);
        } catch (error) {
            throw new TypeError(`cannot decrypt ${column} with the database key`, {
                cause: error,
            });
        }
    }

    /** Bind the cipher to a nonce and a column's context, its name. */
    static #aead(key: Uint8Array<ArrayBuffer>, nonce: Uint8Array, column: string) {
        return gcmsiv(key, nonce, new TextEncoder().encode(column));
    }
}

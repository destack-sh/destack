import { createHash } from "node:crypto";
import { StorageError } from "../error/index.ts";

/** The bytes of an AES-256 customer key. */
const KEY_BYTES = 32;

/** The bytes of an AES block, which the counter advances by. */
const BLOCK_BYTES = 16;

/** The bytes of a write's counter nonce, the counter block's first half. */
const NONCE_BYTES = 8;

/** Encrypt or decrypt bytes at an offset of one write's content. */
export type ContentCipher = (bytes: Uint8Array, offset: number) => Promise<Uint8Array<ArrayBuffer>>;

/** A customer key encrypting file content with AES-256 in counter mode, so reads start at any byte. */
export class CustomerKey {
    /** The base64 MD5 digest of the key, which files record. */
    readonly md5: string;
    /** The imported key. */
    readonly #key: CryptoKey;

    /** Keep an imported key and its digest. */
    private constructor(md5: string, key: CryptoKey) {
        this.md5 = md5;
        this.#key = key;
    }

    /** Read a key given as 32 raw bytes or 64 hexadecimal digits, absent when none is given. */
    static async read(value: ArrayBuffer | string | undefined): Promise<CustomerKey | undefined> {
        // decode the key, refusing another length
        if (value === undefined) {
            return undefined;
        }
        const bytes =
            typeof value === "string"
                ? /^[0-9a-fA-F]{64}$/u.test(value)
                    ? Uint8Array.fromHex(value)
                    : undefined
                : new Uint8Array(value);
        if (bytes === undefined || bytes.byteLength !== KEY_BYTES) {
            throw new StorageError(
                "INVALID_CUSTOMER_KEY",
                "a customer key is 32 bytes or 64 hexadecimal digits",
            );
        }

        // import it with its digest
        const md5 = createHash("md5").update(bytes).digest("base64");
        const key = await crypto.subtle.importKey("raw", bytes, "AES-CTR", false, ["encrypt"]);

        return new CustomerKey(md5, key);
    }

    /** Require the key a file or upload was encrypted with, or none for plain content. */
    static require(key: CustomerKey | undefined, md5: string | null): void {
        if (md5 === null && key !== undefined) {
            throw new StorageError(
                "INVALID_CUSTOMER_KEY",
                "the file is not encrypted with a customer key",
            );
        } else if (md5 !== null && key?.md5 !== md5) {
            throw new StorageError(
                "INVALID_CUSTOMER_KEY",
                "the file is encrypted with another customer key",
            );
        }
    }

    /** Draw a random counter nonce for one write, as hexadecimal. */
    static nonce(): string {
        return crypto.getRandomValues(new Uint8Array(NONCE_BYTES)).toHex();
    }

    /** Bind the key to one write's content under its counter nonce. */
    cipher(nonce: string): ContentCipher {
        const bytes = Uint8Array.fromHex(nonce);

        return (content, offset) => this.#apply(bytes, content, offset);
    }

    /** Encrypt or decrypt bytes at an offset of the content under a nonce. */
    async #apply(
        nonce: Uint8Array,
        bytes: Uint8Array,
        offset: number,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // start the counter at the block with the offset, under the write's nonce
        const counter = new Uint8Array(BLOCK_BYTES);
        counter.set(nonce);
        new DataView(counter.buffer).setBigUint64(
            NONCE_BYTES,
            BigInt(Math.floor(offset / BLOCK_BYTES)),
        );

        // apply the key stream from the block's start, dropping the bytes before the offset
        const skipped = offset % BLOCK_BYTES;
        const padded = new Uint8Array(skipped + bytes.byteLength);
        padded.set(bytes, skipped);
        const applied = await crypto.subtle.encrypt(
            { name: "AES-CTR", counter, length: 64 },
            this.#key,
            padded,
        );

        return new Uint8Array(applied, skipped);
    }
}

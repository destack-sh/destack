import { gcmsiv } from "@noble/ciphers/aes.js";

/** Encrypt bytes with a key and a nonce, as the database's cipher does. */
export function encrypt(
    key: Uint8Array,
    nonce: Uint8Array,
    bytes: Uint8Array,
): Uint8Array<ArrayBuffer> {
    return Uint8Array.from(gcmsiv(key, nonce).encrypt(bytes));
}

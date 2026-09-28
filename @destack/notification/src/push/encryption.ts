import type { PushKeys } from "../object/endpoint.ts";

/** The record size, RFC 8188's default. */
const RECORD_SIZE = 4096;

/** The salt length, as RFC 8188 fixes it. */
const SALT_BYTES = 16;

/** The bytes of an uncompressed P-256 public key. */
const PUBLIC_KEY_BYTES = 65;

/** The delimiter ending the last record, as RFC 8188 fixes it. */
const LAST_RECORD = 2;

/** The curve RFC 8291 fixes. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/** Encrypt push messages as RFC 8291 defines. */
export const PushEncryption = {
    /** Encrypt a message to a subscription's keys. */
    async encrypt(
        keys: PushKeys,
        message: Uint8Array<ArrayBuffer>,
        options: { readonly sender?: CryptoKeyPair; readonly salt?: Uint8Array<ArrayBuffer> } = {},
    ): Promise<Uint8Array<ArrayBuffer>> {
        // agree on a shared secret
        const receiver = decode(keys.p256dh);
        const secret = decode(keys.auth);
        const sender =
            options.sender ??
            ((await crypto.subtle.generateKey(CURVE, true, ["deriveBits"])) as CryptoKeyPair);
        const sent = new Uint8Array(await crypto.subtle.exportKey("raw", sender.publicKey));
        const browser = await crypto.subtle.importKey("raw", receiver, CURVE, false, []);
        const shared = new Uint8Array(
            await crypto.subtle.deriveBits(
                { name: "ECDH", public: browser },
                sender.privateKey,
                256,
            ),
        );

        // derive the content key and nonce
        const info = concat(text("WebPush: info\0"), receiver, sent);
        const input = await derive(secret, shared, info, 32);
        const salt = options.salt ?? crypto.getRandomValues(new Uint8Array(SALT_BYTES));
        const content = await derive(salt, input, text("Content-Encoding: aes128gcm\0"), 16);
        const nonce = await derive(salt, input, text("Content-Encoding: nonce\0"), 12);

        // seal the message behind the header
        const key = await crypto.subtle.importKey("raw", content, "AES-GCM", false, ["encrypt"]);
        const sealed = new Uint8Array(
            await crypto.subtle.encrypt(
                { name: "AES-GCM", iv: nonce },
                key,
                concat(message, Uint8Array.of(LAST_RECORD)),
            ),
        );
        const header = new Uint8Array(SALT_BYTES + 4 + 1 + PUBLIC_KEY_BYTES);
        header.set(salt, 0);
        new DataView(header.buffer).setUint32(SALT_BYTES, RECORD_SIZE);
        header[SALT_BYTES + 4] = PUBLIC_KEY_BYTES;
        header.set(sent, SALT_BYTES + 5);

        return concat(header, sealed);
    },
};

/** Derive key material with HKDF-SHA-256. */
async function derive(
    salt: Uint8Array<ArrayBuffer>,
    input: Uint8Array<ArrayBuffer>,
    info: Uint8Array<ArrayBuffer>,
    length: number,
): Promise<Uint8Array<ArrayBuffer>> {
    const key = await crypto.subtle.importKey("raw", input, "HKDF", false, ["deriveBits"]);
    const bits = await crypto.subtle.deriveBits(
        { name: "HKDF", hash: "SHA-256", salt, info },
        key,
        length * 8,
    );

    return new Uint8Array(bits);
}

/** Decode base64url text. */
function decode(value: string): Uint8Array<ArrayBuffer> {
    return new Uint8Array(Uint8Array.fromBase64(value, { alphabet: "base64url" }));
}

/** Encode text as UTF-8. */
function text(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** Join byte arrays. */
function concat(...parts: readonly Uint8Array<ArrayBuffer>[]): Uint8Array<ArrayBuffer> {
    // copy each part after the ones before it
    const joined = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
    let offset = 0;
    for (const part of parts) {
        joined.set(part, offset);
        offset += part.length;
    }

    return joined;
}

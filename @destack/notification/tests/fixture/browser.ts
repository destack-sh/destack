import type { PushKeys } from "../../src/index.ts";

/** The JWK member with an elliptic-curve private key (RFC 7518 6.2.2.1). */
const PRIVATE_KEY_MEMBER = "d";

/** The elliptic curve of push subscriptions. */
const CURVE = { name: "ECDH", namedCurve: "P-256" } as const;

/** A browser's push subscription: the keys it hands the sender, and the private key only it has. */
export interface Browser {
    /** The public key and authentication secret, base64url encoded. */
    readonly keys: PushKeys;
    /** The key pair decrypting messages. */
    readonly pair: CryptoKeyPair;
}

/** Subscribe a browser to push: a fresh key pair and authentication secret, as PushManager.subscribe makes them. */
export async function subscribeBrowser(): Promise<Browser> {
    const pair = (await crypto.subtle.generateKey(CURVE, true, ["deriveBits"])) as CryptoKeyPair;
    const raw = new Uint8Array(await crypto.subtle.exportKey("raw", pair.publicKey));
    const auth = crypto.getRandomValues(new Uint8Array(16));

    return { keys: { p256dh: encode(raw), auth: encode(auth) }, pair };
}

/** Import a browser from RFC 8291's example keys, its private key as a JWK. */
export async function importBrowser(keys: PushKeys, privateKey: string): Promise<Browser> {
    const raw = Uint8Array.fromBase64(keys.p256dh, { alphabet: "base64url" });
    const x = encode(raw.slice(1, 33));
    const y = encode(raw.slice(33, 65));
    const jwk = { kty: "EC", crv: "P-256", x, y };
    const publicKey = await crypto.subtle.importKey("jwk", jwk, CURVE, true, []);
    const secret = await crypto.subtle.importKey(
        "jwk",
        { ...jwk, [PRIVATE_KEY_MEMBER]: privateKey },
        CURVE,
        true,
        ["deriveBits"],
    );

    return { keys, pair: { publicKey, privateKey: secret } };
}

/** Decrypt a push message as the browser's service worker does, returning its text. */
export async function decrypt(browser: Browser, body: Uint8Array<ArrayBuffer>): Promise<string> {
    // read the header: the salt, the record size and the sender's public key
    const salt = body.slice(0, 16);
    const length = body[20]!;
    const sender = body.slice(21, 21 + length);
    const sealed = body.slice(21 + length);

    // agree on the secret, then derive the content key and nonce as the sender did
    const receiver = Uint8Array.fromBase64(browser.keys.p256dh, { alphabet: "base64url" });
    const auth = Uint8Array.fromBase64(browser.keys.auth, { alphabet: "base64url" });
    const origin = await crypto.subtle.importKey("raw", sender, CURVE, false, []);
    const shared = new Uint8Array(
        await crypto.subtle.deriveBits(
            { name: "ECDH", public: origin },
            browser.pair.privateKey,
            256,
        ),
    );
    const info = join(text("WebPush: info\0"), receiver, sender);
    const input = await derive(auth, shared, info, 32);
    const content = await derive(salt, input, text("Content-Encoding: aes128gcm\0"), 16);
    const nonce = await derive(salt, input, text("Content-Encoding: nonce\0"), 12);

    // open the record, dropping the delimiter ending the last one
    const key = await crypto.subtle.importKey("raw", content, "AES-GCM", false, ["decrypt"]);
    const opened = new Uint8Array(
        await crypto.subtle.decrypt({ name: "AES-GCM", iv: nonce }, key, sealed),
    );

    return new TextDecoder().decode(opened.slice(0, opened.lastIndexOf(2)));
}

/** Derive key material with HKDF-SHA-256. */
async function derive(
    salt: Uint8Array<ArrayBuffer>,
    input: Uint8Array<ArrayBuffer>,
    info: Uint8Array<ArrayBuffer>,
    length: number,
): Promise<Uint8Array<ArrayBuffer>> {
    const key = await crypto.subtle.importKey("raw", input, "HKDF", false, ["deriveBits"]);

    return new Uint8Array(
        await crypto.subtle.deriveBits(
            { name: "HKDF", hash: "SHA-256", salt, info },
            key,
            length * 8,
        ),
    );
}

/** Encode bytes as base64url text. */
export function encode(bytes: Uint8Array): string {
    return bytes.toBase64({ alphabet: "base64url", omitPadding: true });
}

/** Encode text as UTF-8. */
function text(value: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(value);
}

/** Join byte arrays. */
function join(...parts: readonly Uint8Array[]): Uint8Array<ArrayBuffer> {
    const joined = new Uint8Array(parts.reduce((length, part) => length + part.length, 0));
    let offset = 0;
    for (const part of parts) {
        joined.set(part, offset);
        offset += part.length;
    }

    return joined;
}

import { createHmac } from "node:crypto";
import { present } from "@destack/schema";

/** Generate the RFC 6238 code an authenticator would display for an enrollment URI. */
export function authenticatorCode(uri: string): string {
    const secret = present(new URL(uri).searchParams.get("secret"), "the enrollment secret");
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    const bits = Array.from(secret)
        .map((character) => alphabet.indexOf(character).toString(2).padStart(5, "0"))
        .join("");
    const bytes = Array.from({ length: Math.floor(bits.length / 8) }, (_, index) =>
        Number.parseInt(bits.slice(index * 8, index * 8 + 8), 2),
    );

    // sign the current thirty-second counter with the enrolled secret
    const counter = Buffer.alloc(8);
    counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
    const signature = createHmac("sha1", Buffer.from(bytes)).update(counter).digest();
    const offset = signature.readUInt8(signature.length - 1) & 15;
    const code = (signature.readUInt32BE(offset) & 0x7fffffff) % 1_000_000;

    return String(code).padStart(6, "0");
}

/** A platform authenticator with one P-256 passkey, answering WebAuthn ceremonies. */
export class PasskeyAuthenticator {
    /** The key pair of the passkey. */
    readonly keys: CryptoKeyPair;
    /** The credential identifier. */
    readonly id: Uint8Array<ArrayBuffer>;
    /** The origin the browser reports. */
    readonly origin: string;
    /** The signature counter, advanced by every assertion. */
    #counter = 0;

    /** Keep a generated passkey for an origin. */
    private constructor(keys: CryptoKeyPair, origin: string) {
        this.keys = keys;
        this.id = crypto.getRandomValues(new Uint8Array(16));
        this.origin = origin;
    }

    /** Generate a passkey for an origin. */
    static async generate(origin: string): Promise<PasskeyAuthenticator> {
        const keys = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, [
            "sign",
            "verify",
        ]);

        return new PasskeyAuthenticator(keys, origin);
    }

    /** Answer registration options with a none attestation of the passkey, verified by the user. */
    async register(options: { challenge: string }): Promise<object> {
        // encode the public key as a COSE EC2 ES256 key
        const jwk = await crypto.subtle.exportKey("jwk", this.keys.publicKey);
        const key = cbor(
            new Map<number, unknown>([
                [1, 2],
                [3, -7],
                [-1, 1],
                [
                    -2,
                    Uint8Array.fromBase64(present(jwk.x, "the x coordinate"), {
                        alphabet: "base64url",
                    }),
                ],
                [
                    -3,
                    Uint8Array.fromBase64(present(jwk.y, "the y coordinate"), {
                        alphabet: "base64url",
                    }),
                ],
            ]),
        );

        // attest the credential in authenticator data with user presence, verification and attested data
        const length = new Uint8Array([this.id.length >> 8, this.id.length & 255]);
        const data = concat(
            await this.#rpHash(),
            new Uint8Array([0x45, 0, 0, 0, 0]),
            new Uint8Array(16),
            length,
            this.id,
            key,
        );
        const attestation = cbor(
            new Map<string, unknown>([
                ["fmt", "none"],
                ["attStmt", new Map()],
                ["authData", data],
            ]),
        );

        return {
            id: encode(this.id),
            rawId: encode(this.id),
            type: "public-key",
            response: {
                clientDataJSON: this.#clientData("webauthn.create", options.challenge),
                attestationObject: encode(attestation),
                transports: ["internal"],
            },
            clientExtensionResults: {},
            authenticatorAttachment: "platform",
        };
    }

    /** Answer authentication options with an assertion by the passkey, verified by the user. */
    async authenticate(options: { challenge: string }): Promise<object> {
        // sign the authenticator data and the client data hash
        this.#counter += 1;
        const counter = new Uint8Array(4);
        new DataView(counter.buffer).setUint32(0, this.#counter);
        const data = concat(await this.#rpHash(), new Uint8Array([0x05]), counter);
        const client = this.#clientData("webauthn.get", options.challenge);
        const hash = await crypto.subtle.digest(
            "SHA-256",
            Uint8Array.fromBase64(client, { alphabet: "base64url" }),
        );
        const signature = await crypto.subtle.sign(
            { name: "ECDSA", hash: "SHA-256" },
            this.keys.privateKey,
            concat(data, new Uint8Array(hash)),
        );

        return {
            id: encode(this.id),
            rawId: encode(this.id),
            type: "public-key",
            response: {
                clientDataJSON: client,
                authenticatorData: encode(data),
                signature: encode(der(new Uint8Array(signature))),
            },
            clientExtensionResults: {},
            authenticatorAttachment: "platform",
        };
    }

    /** Hash the relying party identifier, the origin's host. */
    async #rpHash(): Promise<Uint8Array<ArrayBuffer>> {
        const host = new TextEncoder().encode(new URL(this.origin).hostname);

        return new Uint8Array(await crypto.subtle.digest("SHA-256", host));
    }

    /** Encode the client data a browser collects for a ceremony. */
    #clientData(type: string, challenge: string): string {
        const data = { type, challenge, origin: this.origin, crossOrigin: false };

        return encode(new TextEncoder().encode(JSON.stringify(data)));
    }
}

/** Encode bytes as unpadded base64url. */
function encode(bytes: Uint8Array): string {
    return bytes.toBase64({ alphabet: "base64url", omitPadding: true });
}

/** Join byte arrays. */
function concat(...parts: Uint8Array[]): Uint8Array<ArrayBuffer> {
    const joined = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
    let offset = 0;
    for (const part of parts) {
        joined.set(part, offset);
        offset += part.length;
    }

    return joined;
}

/** Encode integers, strings, bytes and maps as CBOR, the subset attestations use. */
function cbor(value: unknown): Uint8Array<ArrayBuffer> {
    // encode an unsigned or negative integer
    if (typeof value === "number") {
        return value >= 0 ? head(0, value) : head(1, -1 - value);
    }
    // encode a text string
    else if (typeof value === "string") {
        const text = new TextEncoder().encode(value);

        return concat(head(3, text.length), text);
    }
    // encode a byte string
    else if (value instanceof Uint8Array) {
        return concat(head(2, value.length), value);
    }
    // encode a map of its encoded keys and values
    else if (value instanceof Map) {
        const entries = [...value].flatMap(([key, entry]) => [cbor(key), cbor(entry)]);

        return concat(head(5, value.size), ...entries);
    }
    // refuse any other value
    else {
        throw new TypeError("unsupported cbor value");
    }
}

/** Write a CBOR major type with its argument. */
function head(major: number, argument: number): Uint8Array<ArrayBuffer> {
    return argument < 24
        ? new Uint8Array([(major << 5) | argument])
        : argument < 256
          ? new Uint8Array([(major << 5) | 24, argument])
          : new Uint8Array([(major << 5) | 25, argument >> 8, argument & 255]);
}

/** Encode a raw P-256 signature, r then s, as the DER sequence WebAuthn assertions use. */
function der(raw: Uint8Array): Uint8Array<ArrayBuffer> {
    // encode each half as a minimal positive integer
    const integer = (half: Uint8Array) => {
        let start = 0;
        while (start < half.length - 1 && half[start] === 0) {
            start++;
        }
        const trimmed = half.slice(start);
        const leading = present(trimmed[0], "the integer's leading byte");
        const padded = leading & 0x80 ? concat(new Uint8Array([0]), trimmed) : trimmed;

        return concat(new Uint8Array([0x02, padded.length]), padded);
    };
    const body = concat(integer(raw.slice(0, 32)), integer(raw.slice(32)));

    return concat(new Uint8Array([0x30, body.length]), body);
}

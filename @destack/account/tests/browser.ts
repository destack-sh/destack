import { createHmac } from "node:crypto";

/** Retain browser cookies across real authentication HTTP requests. */
export class Browser {
    /** The origin the browser's pages and requests belong to. */
    readonly origin: string;
    /** Send a request to the served origin. */
    readonly send: (request: Request) => Promise<Response>;
    /** Cookies received from the origin. */
    readonly cookies = new Map<string, string>();

    /** Connect a browser to a served origin. */
    constructor(origin: string, send: (request: Request) => Promise<Response>) {
        this.origin = origin;
        this.send = send;
    }

    /** Send a same-origin request, posting a JSON body or a form with the origin, and apply response cookies. */
    async fetch(path: string, body?: object | URLSearchParams): Promise<Response> {
        const isForm = body instanceof URLSearchParams;
        const request = new Request(new URL(path, this.origin), {
            method: body ? "POST" : "GET",
            headers: {
                ...(body ? { origin: this.origin } : {}),
                "content-type": isForm ? "application/x-www-form-urlencoded" : "application/json",
                cookie: [...this.cookies].map(([name, value]) => `${name}=${value}`).join("; "),
            },
            body: isForm ? body : body ? JSON.stringify(body) : undefined,
        });
        const sent = await this.send(request);

        // read the whole body like a browser to end the request
        const bytes = await sent.arrayBuffer();
        const response = new Response(bytes.byteLength === 0 ? null : bytes, sent);

        // replace or expire cookies as a browser would
        for (const cookie of response.headers.getSetCookie()) {
            const assignment = cookie.split(";")[0];
            const separator = assignment.indexOf("=");
            const name = assignment.slice(0, separator);
            const value = assignment.slice(separator + 1);
            if (/max-age=0(?:;|$)/i.test(cookie)) {
                this.cookies.delete(name);
            } else {
                this.cookies.set(name, value);
            }
        }

        return response;
    }
}

/** Generate the RFC 6238 code an authenticator would display for an enrollment URI. */
export function authenticatorCode(uri: string): string {
    const secret = new URL(uri).searchParams.get("secret")!;
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    const bits = [...secret]
        .map((character) => alphabet.indexOf(character).toString(2).padStart(5, "0"))
        .join("");
    const bytes = Array.from({ length: Math.floor(bits.length / 8) }, (_, index) =>
        Number.parseInt(bits.slice(index * 8, index * 8 + 8), 2),
    );

    // sign the current thirty-second counter with the enrolled secret
    const counter = Buffer.alloc(8);
    counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30_000)));
    const signature = createHmac("sha1", Buffer.from(bytes)).update(counter).digest();
    const offset = signature[signature.length - 1] & 15;
    const code = (signature.readUInt32BE(offset) & 0x7fffffff) % 1_000_000;

    return String(code).padStart(6, "0");
}

/** A platform authenticator holding one P-256 passkey, answering WebAuthn ceremonies as a browser relays them. */
export class PasskeyAuthenticator {
    /** The key pair of the passkey. */
    readonly keys: CryptoKeyPair;
    /** The credential identifier. */
    readonly id: Uint8Array<ArrayBuffer>;
    /** The origin the browser reports. */
    readonly origin: string;
    /** The signature counter, advanced by every assertion. */
    #counter = 0;

    /** Hold a generated passkey for an origin. */
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
                [-2, Uint8Array.fromBase64(jwk.x!, { alphabet: "base64url" })],
                [-3, Uint8Array.fromBase64(jwk.y!, { alphabet: "base64url" })],
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
    // write a major type with its argument
    const head = (major: number, argument: number) =>
        argument < 24
            ? new Uint8Array([(major << 5) | argument])
            : argument < 256
              ? new Uint8Array([(major << 5) | 24, argument])
              : new Uint8Array([(major << 5) | 25, argument >> 8, argument & 255]);

    // encode each supported value
    if (typeof value === "number") {
        return value >= 0 ? head(0, value) : head(1, -1 - value);
    } else if (typeof value === "string") {
        const text = new TextEncoder().encode(value);

        return concat(head(3, text.length), text);
    } else if (value instanceof Uint8Array) {
        return concat(head(2, value.length), value);
    } else if (value instanceof Map) {
        const entries = [...value].flatMap(([key, entry]) => [cbor(key), cbor(entry)]);

        return concat(head(5, value.size), ...entries);
    }
    throw new TypeError("unsupported cbor value");
}

/** Encode a raw P-256 signature, r then s, as the DER sequence WebAuthn assertions carry. */
function der(raw: Uint8Array): Uint8Array<ArrayBuffer> {
    // encode each half as a minimal positive integer
    const integer = (half: Uint8Array) => {
        let start = 0;
        while (start < half.length - 1 && half[start] === 0) {
            start++;
        }
        const trimmed = half.slice(start);
        const padded = trimmed[0]! & 0x80 ? concat(new Uint8Array([0]), trimmed) : trimmed;

        return concat(new Uint8Array([0x02, padded.length]), padded);
    };
    const body = concat(integer(raw.slice(0, 32)), integer(raw.slice(32)));

    return concat(new Uint8Array([0x30, body.length]), body);
}

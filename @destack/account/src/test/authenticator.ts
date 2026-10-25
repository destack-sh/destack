import type { Environment } from "@destack/environment";
import { Totp } from "../authentication/totp.ts";
import { present, schema } from "@destack/schema";

/** The RFC 4648 base32 alphabet otpauth URIs encode secrets in. */
const BASE32 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/** The bytes of a passkey's credential identifier, WebAuthn's least of 16. */
const CREDENTIAL_BYTES = 16;

/** An authenticator enrollment: its TOTP URI and backup codes. */
export const Enrollment = schema.looseObject({
    /** The URI an authenticator app enrolls from. */
    totpURI: schema.string(),
    /** The single-use recovery codes. */
    backupCodes: schema.array(schema.string()),
});

/** Generate the RFC 6238 code an authenticator shows some steps from now on a clock. */
export async function authenticatorCode(
    uri: string,
    environment: Pick<Environment, "clock" | "crypto">,
    steps = 0,
): Promise<string> {
    // decode the enrolled base32 secret into bytes
    const secret = present(new URL(uri).searchParams.get("secret"), "the enrollment secret");
    const bits = Array.from(secret)
        .map((character) => BASE32.indexOf(character).toString(2).padStart(5, "0"))
        .join("");
    const bytes = Uint8Array.from({ length: Math.floor(bits.length / 8) }, (_, index) =>
        Number.parseInt(bits.slice(index * 8, index * 8 + 8), 2),
    );

    // compute the code of the step
    const { clock, crypto } = environment;
    const totp = await Totp.of(bytes, crypto);

    return totp.code(Totp.step(clock.now()) + steps, crypto);
}

/** A platform authenticator with one P-256 passkey, answering WebAuthn ceremonies. */
export class PasskeyAuthenticator {
    /** The key pair of the passkey. */
    readonly keys: CryptoKeyPair;
    /** The credential identifier. */
    readonly id: Uint8Array<ArrayBuffer>;
    /** The origin the browser reports. */
    readonly origin: string;
    /** The crypto the passkey signs and hashes with. */
    readonly crypto: Environment["crypto"];
    /** The signature counter, advanced by every assertion. */
    #counter = 0;

    /** Store a generated passkey for an origin. */
    private constructor(
        keys: CryptoKeyPair,
        origin: string,
        environment: Pick<Environment, "crypto" | "random">,
    ) {
        // store the keys, a fresh credential identifier, the origin and the WebCrypto
        this.keys = keys;
        this.id = environment.random.bytes(CREDENTIAL_BYTES);
        this.origin = origin;
        this.crypto = environment.crypto;
    }

    /** Generate a passkey for an origin. */
    static async generate(
        origin: string,
        environment: Pick<Environment, "crypto" | "random">,
    ): Promise<PasskeyAuthenticator> {
        const keys = await environment.crypto.generateKey(
            { name: "ECDSA", namedCurve: "P-256" },
            true,
            ["sign", "verify"],
        );

        return new PasskeyAuthenticator(keys, origin, environment);
    }

    /** Answer registration options with a none attestation of the passkey, verified by the user. */
    async register(options: { challenge: string }): Promise<object> {
        // encode the public key as a COSE EC2 ES256 key
        const jwk = await this.crypto.exportKey("jwk", this.keys.publicKey);
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
        const hash = await this.crypto.digest(
            "SHA-256",
            Uint8Array.fromBase64(client, { alphabet: "base64url" }),
        );
        const signature = await this.crypto.sign(
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

        return new Uint8Array(await this.crypto.digest("SHA-256", host));
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
function concat(...arrays: Uint8Array[]): Uint8Array<ArrayBuffer> {
    // copy each array after the ones before it
    const joined = new Uint8Array(arrays.reduce((total, array) => total + array.length, 0));
    let offset = 0;
    for (const array of arrays) {
        joined.set(array, offset);
        offset += array.length;
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
        // skip the leading zero bytes
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

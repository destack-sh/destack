import type { Environment, Network } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/index.ts";
import { type KeyringEnvironment, LocalKeyring } from "../keyring/index.ts";
import { type AwsCredentials, AwsSigner } from "./signature.ts";

/** The service name KMS signs requests under. */
const SERVICE = "kms";

/** The JSON protocol version of KMS's API. */
const CONTENT_TYPE = "application/x-amz-json-1.1";

/** How long a KMS call may take before it refuses, in milliseconds. */
const TIMEOUT_MILLISECONDS = 10_000;

/** The data key spec of a root key: 256 bits, as `LocalKeyring` imports them. */
const KEY_SPEC = "AES_256";

/** The encryption context key binding a root key to its owner, which IAM conditions read. */
const OWNER = "owner";

/** A root key version as text: a positive integer without leading zeros. */
const VERSION = /^[1-9]\d*$/u;

/** The AWS error codes of throttling and of an unavailable service, which pass with time. */
const UNAVAILABLE = new Set([
    "ThrottlingException",
    "KMSInternalException",
    "DependencyTimeoutException",
]);

/** The AWS error codes of a ciphertext that its key, its context or its bytes do not match. */
const MISMATCHED = new Set(["InvalidCiphertextException", "IncorrectKeyException"]);

/** A keyset whose root key versions KMS encrypts under one key, as a process's secret stores it. */
const SealedKeyset = schema.object({
    /** The ARN of the KMS key encrypting every version. */
    keyId: schema.string().min(1),
    /** The version new ciphertexts are encrypted under. */
    active: schema.number().int().positive(),
    /** The base64 KMS ciphertext of each version, by its decimal version. */
    keys: schema.record(schema.string().regex(VERSION), schema.base64()),
});
/** A keyset whose root key versions KMS encrypts under one key. */
type SealedKeyset = schema.Infer<typeof SealedKeyset>;

/** KMS's answer to `GenerateDataKeyWithoutPlaintext`. */
const Generated = schema.looseObject({ CiphertextBlob: schema.base64() });

/** KMS's answer to `Encrypt`. */
const Encrypted = schema.looseObject({ CiphertextBlob: schema.base64() });

/** KMS's answer to `Decrypt`. */
const Decrypted = schema.looseObject({ Plaintext: schema.base64() });

/** KMS's error body, with its type and message. */
const ErrorBody = schema.looseObject({
    __type: schema.string().exactOptional(),
    message: schema.string().exactOptional(),
    Message: schema.string().exactOptional(),
});

/** Where and as whom a client calls KMS. */
export interface AwsKmsOptions {
    /** The region the keys live in, such as eu-central-1. */
    readonly region: string;
    /** The KMS origin, the region's (`AwsKms.endpoint`) outside tests. */
    readonly endpoint: string;
    /** The signing credentials. */
    readonly credentials: AwsCredentials;
    /** The network calls go through, the clock their timeouts run on, and the signing WebCrypto. */
    readonly environment: Pick<Environment, "clock" | "crypto"> & {
        readonly network: Pick<Network, "fetch">;
    };
}

/** A client of AWS KMS sealing and unsealing root keys and secrets for their owners. */
export class AwsKms {
    /** The options. */
    readonly options: AwsKmsOptions;
    /** The signer of KMS requests. */
    readonly #signer: AwsSigner;

    /** Build the origin of a region's KMS API. */
    static endpoint(region: string): string {
        return `https://kms.${region}.amazonaws.com`;
    }

    /** Create a client signing for its region. */
    constructor(options: AwsKmsOptions) {
        this.options = options;
        this.#signer = new AwsSigner({
            service: SERVICE,
            region: options.region,
            credentials: options.credentials,
            crypto: options.environment.crypto,
        });
    }

    /** Write a sealed keyset with a new active root key KMS generates for an owner. */
    async seal(keyId: string, owner: string, sealed?: string): Promise<string> {
        // continue the earlier keyset under the same key, refusing another
        const keys = sealed === undefined ? undefined : parse(sealed);
        if (keys !== undefined && keys.keyId !== keyId) {
            throw IdentityError.of(
                "ROOT_KEY_BROKEN",
                `the keyset is sealed under '${keys.keyId}', not '${keyId}'`,
            );
        }

        // generate the next version's ciphertext inside KMS, its plaintext never leaving it
        const version = Math.max(0, ...Object.keys(keys?.keys ?? {}).map(Number)) + 1;
        const generated = Generated.parse(
            await this.#call("GenerateDataKeyWithoutPlaintext", {
                KeyId: keyId,
                KeySpec: KEY_SPEC,
                EncryptionContext: { [OWNER]: owner },
            }),
        );

        return JSON.stringify(
            SealedKeyset.parse({
                keyId,
                active: version,
                keys: { ...keys?.keys, [String(version)]: generated.CiphertextBlob },
            }),
        );
    }

    /** Open the keyring a sealed keyset holds for an owner, decrypting each version through KMS. */
    async unseal(
        sealed: string,
        owner: string,
        environment: KeyringEnvironment,
    ): Promise<LocalKeyring> {
        // decrypt every version under the keyset's key and the owner's context
        const keys = parse(sealed);
        const versions = await Promise.all(
            Object.entries(keys.keys).map(
                async ([version, ciphertext]) =>
                    [Number(version), await this.decrypt(keys.keyId, ciphertext, owner)] as const,
            ),
        );

        return LocalKeyring.import(keys.active, new Map(versions), environment);
    }

    /** Encrypt bytes under a key for an owner, answering the base64 ciphertext. */
    async encrypt(
        keyId: string,
        plaintext: Uint8Array<ArrayBuffer>,
        owner: string,
    ): Promise<string> {
        const encrypted = Encrypted.parse(
            await this.#call("Encrypt", {
                KeyId: keyId,
                Plaintext: plaintext.toBase64(),
                EncryptionContext: { [OWNER]: owner },
            }),
        );

        return encrypted.CiphertextBlob;
    }

    /** Decrypt a base64 ciphertext under a key for the owner it was encrypted for. */
    async decrypt(
        keyId: string,
        ciphertext: string,
        owner: string,
    ): Promise<Uint8Array<ArrayBuffer>> {
        const decrypted = Decrypted.parse(
            await this.#call("Decrypt", {
                KeyId: keyId,
                CiphertextBlob: ciphertext,
                EncryptionContext: { [OWNER]: owner },
            }),
        );

        return Uint8Array.fromBase64(decrypted.Plaintext);
    }

    /** Call a KMS operation with a JSON body, answering its JSON or refusing as its error reads. */
    async #call(operation: string, input: Readonly<Record<string, unknown>>): Promise<unknown> {
        // sign the call
        const { endpoint, environment } = this.options;
        const url = new URL("/", endpoint);
        const body = JSON.stringify(input);
        const headers = await this.#signer.authorize(
            {
                method: "POST",
                path: "/",
                query: [],
                headers: {
                    "content-type": CONTENT_TYPE,
                    "x-amz-target": `TrentService.${operation}`,
                    host: url.host,
                },
                payloadHash: await AwsSigner.payloadHash(body, environment.crypto),
            },
            new Date(environment.clock.now()),
        );

        // post it within the timeout, reading a lapsed or lost call as unavailable
        const signal = environment.clock.timeout(TIMEOUT_MILLISECONDS);
        const request = new Request(url.href, { method: "POST", headers, body, signal });
        let response: Response;
        try {
            response = await environment.network.fetch(request);
        } catch (error) {
            throw IdentityError.of(
                "KMS_UNAVAILABLE",
                `KMS at '${endpoint}' is unavailable: it did not respond`,
                {
                    cause: error,
                },
            );
        }

        // answer the body, or the refusal
        if (!response.ok) {
            throw await refusal(operation, response);
        }

        return await response.json();
    }
}

/** Parse a sealed keyset, refusing a malformed one. */
function parse(sealed: string): SealedKeyset {
    const read = SealedKeyset.safeParse(JSON.parse(sealed));
    if (!read.success) {
        throw IdentityError.of("ROOT_KEY_BROKEN", "the sealed keyset is malformed");
    }

    return read.data;
}

/** Read KMS's refusal of an operation as an identity failure by its AWS error type. */
async function refusal(operation: string, response: Response) {
    // read the type after any hash and the message
    const text = await response.text();
    let parsed: unknown;
    try {
        parsed = JSON.parse(text);
    } catch {
        parsed = undefined;
    }
    const read = ErrorBody.safeParse(parsed);
    const fields: schema.Infer<typeof ErrorBody> = read.success ? read.data : {};
    const type = fields.__type?.split("#").pop();
    const detail = fields.message ?? fields.Message ?? text;
    const message = `KMS refused ${operation} with HTTP ${response.status}${type === undefined ? "" : ` ${type}`}: ${detail}`;

    // classify it: passing with time, a mismatched ciphertext, or a refused key or caller
    if (response.status >= 500 || (type !== undefined && UNAVAILABLE.has(type))) {
        return IdentityError.of("KMS_UNAVAILABLE", message);
    } else if (type !== undefined && MISMATCHED.has(type)) {
        return IdentityError.of("DECRYPTION_BROKEN", message);
    } else {
        return IdentityError.of("KMS_BROKEN", message);
    }
}

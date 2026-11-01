import type { Environment, Network } from "@destack/environment";
import { schema } from "@destack/schema";
import { AwsKms, AwsSigner } from "../aws/index.ts";

/** The region a test KMS and its clients sign for. */
export const TEST_KMS_REGION = "eu-central-1";

/** The key a test KMS encrypts under. */
export const TEST_KMS_KEY_ID = "arn:aws:kms:eu-central-1:111122223333:key/root";

/** A key a test KMS refuses its callers. */
export const TEST_KMS_DENIED_KEY_ID = "arn:aws:kms:eu-central-1:111122223333:key/denied";

/** The credentials a test KMS accepts signatures of. */
export const TEST_KMS_CREDENTIALS = {
    accessKeyId: "AKIDEXAMPLE",
    secretAccessKey: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
};

/** The input of a test KMS's operations. */
const KmsInput = schema.looseObject({
    /** The key the operation uses. */
    KeyId: schema.string(),
    /** The bytes to encrypt. */
    Plaintext: schema.string().exactOptional(),
    /** The ciphertext to decrypt. */
    CiphertextBlob: schema.string().exactOptional(),
    /** The context the ciphertext binds. */
    EncryptionContext: schema.record(schema.string(), schema.string()),
});
/** The input of a test KMS's operations. */
type KmsInput = schema.Infer<typeof KmsInput>;

/** The AES-GCM nonce length a test KMS prefixes its ciphertexts with. */
const NONCE_BYTES = 12;

/** A KMS in memory: one AES key, its ciphertexts bound to their key id and encryption context. */
export class TestKms {
    /** The clock, WebCrypto and randomness it answers with. */
    readonly environment: Pick<Environment, "clock" | "crypto" | "random">;
    /** The operations called, in order. */
    readonly operations: string[] = [];
    /** The key encrypting every data key. */
    readonly #key: Promise<CryptoKey>;
    /** The signer recomputing each request's signature. */
    readonly #signer: AwsSigner;

    /** Start a KMS with a fresh key in an environment. */
    constructor(environment: Pick<Environment, "clock" | "crypto" | "random">) {
        // keep the environment, a fresh key and the signer of the accepted credentials
        this.environment = environment;
        this.#key = environment.crypto.importKey(
            "raw",
            environment.random.bytes(32),
            "AES-GCM",
            false,
            ["encrypt", "decrypt"],
        );
        this.#signer = new AwsSigner({
            service: "kms",
            region: TEST_KMS_REGION,
            credentials: TEST_KMS_CREDENTIALS,
            crypto: environment.crypto,
        });
    }

    /** Answer every request of a network as this KMS. */
    network(network: Network): Network {
        return { ...network, fetch: (request) => this.fetch(new Request(request)) };
    }

    /** Open a client of this KMS. */
    client(): AwsKms {
        return new AwsKms({
            region: TEST_KMS_REGION,
            endpoint: AwsKms.endpoint(TEST_KMS_REGION),
            credentials: TEST_KMS_CREDENTIALS,
            environment: {
                ...this.environment,
                network: { fetch: (request) => this.fetch(new Request(request)) },
            },
        });
    }

    /** Answer a KMS request, refusing a bad signature as KMS does. */
    async fetch(request: Request): Promise<Response> {
        // refuse a request whose signature differs from the one its signed headers give
        const body = await request.text();
        const target = request.headers.get("x-amz-target") ?? "";
        const expected = await this.#signer.authorize(
            {
                method: "POST",
                path: "/",
                query: [],
                headers: {
                    "content-type": request.headers.get("content-type") ?? "",
                    "x-amz-target": target,
                    host: new URL(request.url).host,
                },
                payloadHash: await AwsSigner.payloadHash(body, this.environment.crypto),
            },
            new Date(this.environment.clock.now()),
        );
        if (request.headers.get("authorization") !== expected["authorization"]) {
            return failure(400, "InvalidSignatureException", "The request signature is wrong.");
        }

        // answer the operation, refusing the denied key
        const input = KmsInput.parse(JSON.parse(body));
        const operation = target.replace("TrentService.", "");
        this.operations.push(operation);
        if (input.KeyId === TEST_KMS_DENIED_KEY_ID) {
            return failure(400, "AccessDeniedException", "User is not authorized to use the key.");
        } else if (operation === "GenerateDataKeyWithoutPlaintext") {
            const generated = await this.#encrypt(input, this.environment.random.bytes(32));

            return Response.json({ CiphertextBlob: generated, KeyId: TEST_KMS_KEY_ID });
        } else if (operation === "Encrypt") {
            const plaintext = Uint8Array.fromBase64(input.Plaintext ?? "");

            return Response.json({
                CiphertextBlob: await this.#encrypt(input, plaintext),
                KeyId: TEST_KMS_KEY_ID,
            });
        } else {
            return await this.#decrypt(input);
        }
    }

    /** Encrypt bytes under the key, bound to its key id and context. */
    async #encrypt(input: KmsInput, plaintext: Uint8Array<ArrayBuffer>) {
        const nonce = this.environment.random.bytes(NONCE_BYTES);
        const sealed = await this.environment.crypto.encrypt(
            { name: "AES-GCM", iv: nonce, additionalData: bound(input) },
            await this.#key,
            plaintext,
        );

        return new Uint8Array([...nonce, ...new Uint8Array(sealed)]).toBase64();
    }

    /** Decrypt a data key, refusing one bound to another key or context. */
    async #decrypt(input: KmsInput): Promise<Response> {
        const bytes = Uint8Array.fromBase64(input.CiphertextBlob ?? "");
        try {
            const plaintext = await this.environment.crypto.decrypt(
                { name: "AES-GCM", iv: bytes.slice(0, NONCE_BYTES), additionalData: bound(input) },
                await this.#key,
                bytes.slice(NONCE_BYTES),
            );

            return Response.json({
                Plaintext: new Uint8Array(plaintext).toBase64(),
                KeyId: TEST_KMS_KEY_ID,
            });
        } catch {
            return failure(400, "InvalidCiphertextException", "The ciphertext is invalid.");
        }
    }
}

/** Write the additional data a test ciphertext binds: its key id and encryption context. */
function bound(input: KmsInput) {
    return new TextEncoder().encode(JSON.stringify([input.KeyId, input.EncryptionContext]));
}

/** Answer a KMS error as its JSON protocol writes it. */
function failure(status: number, type: string, message: string): Response {
    return Response.json({ __type: type, message }, { status });
}

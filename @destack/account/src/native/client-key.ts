import { type Keychain, KeyPair, Proof } from "@destack/identity";
import { type Identifier, schema } from "@destack/schema";
import type { Environment } from "@destack/environment";
import type {} from "@destack/package/import-meta";
import { AccountError } from "../error/error.ts";

/** The keychain service storing the keys of the clients registered on a computer, by client. */
export const CLIENT_KEYS = "app.destack.client";

/** The keys of the clients registered on a computer, proving them to its machine. */
export class ClientKey {
    /** The keychain storing the private keys, by client. */
    readonly #keys: Keychain;
    /** The clock proofs are signed and verified at, and the WebCrypto and randomness of keys. */
    readonly #environment: Pick<Environment, "clock" | "crypto" | "random">;

    /** Store client keys in a keychain. */
    constructor(keys: Keychain, environment: Pick<Environment, "clock" | "crypto" | "random">) {
        this.#keys = keys;
        this.#environment = environment;
    }

    /** Generate and store a key for a client a sign-in registered. */
    async generate(client: Identifier<"client">): Promise<void> {
        const { secret } = await KeyPair.generate(this.#environment.crypto);
        await this.#keys.save(client, secret);
    }

    /** Sign a proof of a client now, which the machine on this computer verifies. */
    async prove(client: Identifier<"client">): Promise<string> {
        const { privateKey, publicKey } = await this.#load(client);

        return Proof.sign(
            privateKey,
            publicKey,
            client,
            this.#environment.clock.now(),
            this.#environment,
        );
    }

    /** Verify a client's proof with the key this keychain stores, answering the client. */
    async verify(proof: string): Promise<Identifier<"client">> {
        // read the client the proof speaks for, refusing one this computer stores no key of
        const read = Proof.read(proof);
        const client = schema.identifier("client").safeParse(read.subject);
        if (!client.success || (await this.#keys.load(client.data)) === undefined) {
            throw AccountError.of("CLIENT_KEY_UNKNOWN", "no client of this computer signed it");
        }

        // verify it with the client's key now
        const { publicKey } = await this.#load(client.data);
        await read.verify(
            publicKey,
            client.data,
            this.#environment.clock.now(),
            this.#environment.crypto,
        );

        return client.data;
    }

    /** Forget a client's key. */
    async forget(client: Identifier<"client">): Promise<void> {
        await this.#keys.remove(client);
    }

    /** Read a client's stored key pair, refusing a client without one. */
    async #load(client: Identifier<"client">): Promise<KeyPair> {
        const secret = await this.#keys.load(client);
        if (secret === undefined) {
            throw AccountError.of(
                "CLIENT_KEY_MISSING",
                `client ${client} has no key on this computer`,
            );
        }

        return KeyPair.read(secret, this.#environment.crypto);
    }
}

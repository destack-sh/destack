import { type Keychain, KeyPair, Proof } from "@destack/identity";
import { CallError } from "@destack/object";
import { type Identifier, schema } from "@destack/schema";
import type { Clock } from "@destack/environment";

/** The keychain service storing the keys of the clients registered on a computer, by client. */
export const CLIENT_KEYS = "app.destack.client";

/** The keys of the clients registered on a computer, proving them to its machine. */
export class ClientKey {
    /** The keychain storing the private keys, by client. */
    readonly #keys: Keychain;
    /** The clock proofs are signed and verified at. */
    readonly #clock: Clock;

    /** Store client keys in a keychain. */
    constructor(keys: Keychain, clock: Clock) {
        this.#keys = keys;
        this.#clock = clock;
    }

    /** Generate and store a key for a client a sign-in registered. */
    async generate(client: Identifier<"client">): Promise<void> {
        const { secret } = await KeyPair.generate();
        await this.#keys.save(client, secret);
    }

    /** Sign a proof of a client now, which the machine on this computer verifies. */
    async prove(client: Identifier<"client">): Promise<string> {
        const { privateKey, publicKey } = await this.#load(client);

        return Proof.sign(privateKey, publicKey, client, this.#clock.now());
    }

    /** Verify a client's proof with the key this keychain stores, answering the client. */
    async verify(proof: string): Promise<Identifier<"client">> {
        // read the client the proof speaks for, refusing one this computer stores no key of
        const read = Proof.read(proof);
        const client = schema.identifier("client").safeParse(read.subject);
        if (!client.success || (await this.#keys.load(client.data)) === undefined) {
            throw new CallError("UNAUTHORIZED", {
                message: "no client of this computer signed it",
            });
        }

        // verify it with the client's key now
        const { publicKey } = await this.#load(client.data);
        await read.verify(publicKey, client.data, this.#clock.now());

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
            throw new TypeError(`client ${client} has no key on this computer`);
        }

        return KeyPair.read(secret);
    }
}

import type { Fetch, OmitEach } from "@destack/object";
import { PackageId } from "@destack/package";
import type { Environment } from "@destack/environment";
import { RequestId, Scope } from "@destack/sync";
import { type Identifier, schema } from "@destack/schema";
import type { ClientOf } from "@destack/object/client";
import { Machine, machine } from "../object/index.ts";
import accountPackage from "../package.ts";
import {
    Keychain,
    KeyPair,
    type Keyring,
    MemoryKeychain,
    Proof,
    PublicKey,
} from "@destack/identity";
import { Assertion } from "@destack/access/token";
import { principal } from "@destack/access";
import { AccountError } from "../error/error.ts";
import type { TokenBinding } from "../native/issuer.ts";
import { TokenSource } from "./bearer.ts";

/** The label a cloud machine's key derives from its process's keyring under. */
const MACHINE_KEY_LABEL = { label: "@destack/account/machine-key", context: "" } as const;

/** What a machine enrolls with beside its key: the shared fields and those of its kind. */
export type Enrollment = OmitEach<
    Parameters<ClientOf<typeof accountPackage>["machine"]["enroll"]>[0],
    "id" | "publicKey" | "proof"
>;

/** The identity a machine proves with its key. */
export class MachineIdentity {
    /** The machine's identifier. */
    readonly machineId: Identifier<"machine">;
    /** The keychain storing the private key. */
    readonly #keys: Keychain;
    /** The key pair once loaded or generated. */
    #pair: KeyPair | undefined;
    /** The access tokens granted to this machine by its assertions. */
    readonly #tokens: TokenSource;
    /** The environment proofs are signed and tokens refreshed at, and requests drawn from. */
    readonly #environment: Environment;

    /** Prove a machine's identity with the key in its keychain, in an environment. */
    constructor(machineId: Identifier<"machine">, keys: Keychain, environment: Environment) {
        // keep the keychain, and get tokens by the assertions the machine's key signs
        this.machineId = machineId;
        this.#keys = keys;
        this.#environment = environment;
        this.#tokens = new TokenSource((endpoint) => this.assert(endpoint), environment);
    }

    /** Generate and store a key pair, and return its public half with a proof of possession. */
    async generate(): Promise<{ publicKey: PublicKey; proof: string }> {
        // replace the stored pair and prove possession of the pair stored
        const stored = await this.#keys.load(this.machineId);
        const pair = await this.#replace(stored, await KeyPair.generate(this.#environment.crypto));

        return { publicKey: pair.publicKey, proof: await this.prove() };
    }

    /** Prove a machine's identity with a key derived from a keyring. */
    static async derive(
        machineId: Identifier<"machine">,
        keyring: Pick<Keyring, "derivePrivateKey">,
        environment: Environment,
    ): Promise<MachineIdentity> {
        const keychain = new MemoryKeychain();
        await MachineIdentity.save(machineId, keyring, keychain);

        return new MachineIdentity(machineId, keychain, environment);
    }

    /** Save the key a keyring derives for a machine in a keychain. */
    static async save(
        machineId: Identifier<"machine">,
        keyring: Pick<Keyring, "derivePrivateKey">,
        keys: Keychain,
    ): Promise<void> {
        await keys.save(
            machineId,
            JSON.stringify(await keyring.derivePrivateKey(MACHINE_KEY_LABEL)),
        );
    }

    /** Enroll this machine with a new key through a client the approving user signed in. */
    async enroll(
        client: ClientOf<typeof accountPackage>,
        enrollment: Enrollment,
    ): Promise<Machine> {
        await this.generate();

        return this.register(client, enrollment);
    }

    /** Enroll this machine with the key it stores through a client the approving user signed in. */
    async register(
        client: ClientOf<typeof accountPackage>,
        enrollment: Enrollment,
    ): Promise<Machine> {
        const row = await client.machine.enroll({
            ...enrollment,
            id: this.machineId,
            publicKey: await this.key(),
            proof: await this.prove(),
        });

        return Machine.of(row);
    }

    /** Register a new key through a client signed with the current key. */
    async rotate(client: ClientOf<typeof accountPackage>, accountId: string): Promise<void> {
        // rotate under the key's lock while the stored key is still the one seen
        const seen = await this.#keys.load(this.machineId);
        {
            await using _held = await this.#keys.lock(this.machineId);
            if ((await this.#keys.load(this.machineId)) === seen) {
                await this.#rotate(client, accountId);
            }
        }

        // reload the stored key and forget the tokens the replaced key granted
        this.#pair = undefined;
        this.#tokens.clear();
    }

    /** Read the public half of this machine's key. */
    async key(): Promise<PublicKey> {
        return (await this.#load()).publicKey;
    }

    /** Sign a proof for this machine now that it has its key. */
    async prove(): Promise<string> {
        const pair = await this.#load();

        return Proof.sign(
            pair.privateKey,
            pair.publicKey,
            this.machineId,
            this.#environment.clock.now(),
            this.#environment,
        );
    }

    /** Wrap a fetch to call a package with this machine's tokens, in a partition when given. */
    fetch(
        send: Fetch,
        audience: PackageId,
        issuer: string,
        binding: Pick<TokenBinding, "partition"> = {},
    ): Fetch {
        return this.#tokens.fetch(send, audience, issuer, binding);
    }

    /** Read this machine's access token for a package, in a partition when given (RFC 7523). */
    token(
        audience: PackageId,
        issuer: string,
        fetch: Fetch,
        binding: Pick<TokenBinding, "partition"> = {},
    ): Promise<string> {
        return this.#tokens.token(audience, issuer, fetch, binding);
    }

    /** Sign an assertion of this machine for a token endpoint now (RFC 7523). */
    async assert(endpoint: string): Promise<string> {
        const pair = await this.#load();
        const key = {
            id: await PublicKey.thumbprint(pair.publicKey, this.#environment.crypto),
            subject: principal.machine.reference(Scope.universe.id, this.machineId),
            publicKey: pair.publicKey,
        };

        return Assertion.sign(
            pair.privateKey,
            key,
            endpoint,
            this.#environment.clock.now(),
            this.#environment,
        );
    }

    /** Forget the machine's private key. */
    async forget(): Promise<void> {
        await this.#keys.remove(this.machineId);
        this.#pair = undefined;
        this.#tokens.clear();
    }

    /** Store a generated pair in place of the key read before. */
    async #replace(
        stored: string | undefined,
        generated: { readonly pair: KeyPair; readonly secret: string },
    ): Promise<KeyPair> {
        // store the generated pair or the key another writer stored meanwhile
        const current = await Keychain.update(this.#keys, this.machineId, (key) =>
            key === stored || key === undefined ? generated.secret : key,
        );
        if (current === generated.secret) {
            this.#pair = generated.pair;

            return generated.pair;
        }

        // adopt the key the other writer stored
        this.#pair = await KeyPair.read(current, this.#environment.crypto);

        return this.#pair;
    }

    /** Register a new key through a client signed with the current one, then store it. */
    async #rotate(client: ClientOf<typeof accountPackage>, accountId: string): Promise<void> {
        // generate a new pair beside the stored one
        const stored = await this.#keys.load(this.machineId);
        const generated = await KeyPair.generate(this.#environment.crypto);
        const { pair } = generated;

        // register the new pair with a proof by its private half
        await client.key.create({
            scope: schema.identifier("account").parse(accountId),
            requestId: RequestId.create(this.#environment),
            parent: { ...machine.typeReference, id: this.machineId },
            publicKey: pair.publicKey,
            proof: await Proof.sign(
                pair.privateKey,
                pair.publicKey,
                this.machineId,
                this.#environment.clock.now(),
                this.#environment,
            ),
        });

        // store the new pair or the one a racing writer stored
        await this.#replace(stored, generated);
    }

    /** Load the stored key pair and refuse a machine without one. */
    async #load(): Promise<KeyPair> {
        // reuse the pair once loaded
        if (this.#pair !== undefined) {
            return this.#pair;
        }

        // read the private half from the store
        const stored = await this.#keys.load(this.machineId);
        if (stored === undefined) {
            throw AccountError.of("MACHINE_KEY_MISSING", "this machine has no key");
        }
        this.#pair = await KeyPair.read(stored, this.#environment.crypto);

        return this.#pair;
    }
}

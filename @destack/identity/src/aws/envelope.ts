import type { Environment } from "@destack/environment";
import { schema } from "@destack/schema";
import { IdentityError } from "../error/index.ts";
import { type KeyringEnvironment, LocalKeyring } from "../keyring/index.ts";
import { AwsKms } from "./kms.ts";
import type { AwsCredentials } from "./signature.ts";

/** How a process's root keys and secrets are sealed, as AWS's envelope encryption. */
export const EnvelopeConfiguration = schema.discriminatedUnion("kind", [
    schema.object({
        /** Kept in the clear in the process's secrets, for local runs. */
        kind: schema.literal("local"),
    }),
    schema.object({
        /** Sealed by an AWS KMS key. */
        kind: schema.literal("aws-kms"),
        /** The region the key lives in, such as eu-central-1. */
        region: schema.string().min(1),
        /** The ARN of the key. */
        keyId: schema.string().min(1),
    }),
]);
/** How a process's root keys and secrets are sealed. */
export type EnvelopeConfiguration = schema.Infer<typeof EnvelopeConfiguration>;

/** The owner of the universe's sealed keys, which its KMS key policy matches. */
export const UNIVERSE_OWNER = "universe";

/** The envelope a process opens its root keys and secrets through, for one owner. */
export class Envelope {
    /** How the secrets are sealed. */
    readonly configuration: EnvelopeConfiguration;
    /** The owner the secrets are sealed for, such as `machine eu-central`. */
    readonly owner: string;
    /** The KMS client and key of an AWS envelope. */
    readonly #kms: { readonly client: AwsKms; readonly keyId: string } | undefined;

    /** Open a envelope for an owner, refusing an AWS envelope without credentials. */
    constructor(
        configuration: EnvelopeConfiguration,
        owner: string,
        credentials: AwsCredentials | undefined,
        environment: Pick<Environment, "clock" | "crypto" | "network">,
    ) {
        // keep the configuration and the owner
        this.configuration = configuration;
        this.owner = owner;

        // open the KMS client of an AWS envelope
        if (configuration.kind === "local") {
            this.#kms = undefined;
        } else if (credentials === undefined) {
            throw IdentityError.of("KMS_REFUSED", "the aws-kms envelope needs KMS credentials");
        } else {
            const client = new AwsKms({
                region: configuration.region,
                endpoint: AwsKms.endpoint(configuration.region),
                credentials,
                environment,
            });
            this.#kms = { client, keyId: configuration.keyId };
        }
    }

    /** Write the owner of a cloud machine's sealed keys, which its KMS key policy matches. */
    static machine(id: string): string {
        return `machine ${id}`;
    }

    /** Generate a keyset with a new root key, inside KMS for a KMS envelope, else in the clear. */
    async generate(environment: KeyringEnvironment): Promise<string> {
        const kms = this.#kms;

        return kms === undefined
            ? LocalKeyring.generate(environment)
            : kms.client.seal(kms.keyId, this.owner);
    }

    /** Seal a secret: kept in the clear locally, a KMS ciphertext for the owner else. */
    async seal(secret: string): Promise<string> {
        const kms = this.#kms;

        return kms === undefined
            ? secret
            : kms.client.encrypt(kms.keyId, new TextEncoder().encode(secret), this.owner);
    }

    /** Open the root keyring of a keyset: plain under a local envelope, sealed by KMS else. */
    async keyring(keyset: string, environment: KeyringEnvironment): Promise<LocalKeyring> {
        return this.#kms === undefined
            ? LocalKeyring.read(keyset, environment)
            : this.#kms.client.unseal(keyset, this.owner, environment);
    }

    /** Open a sealed secret as text: kept in the clear locally, a KMS ciphertext else. */
    async open(secret: string): Promise<string> {
        // answer a local secret as it is
        const kms = this.#kms;
        if (kms === undefined) {
            return secret;
        }

        // decrypt a sealed one for the owner
        const plaintext = await kms.client.decrypt(kms.keyId, secret, this.owner);

        return new TextDecoder().decode(plaintext);
    }
}

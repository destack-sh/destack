import type { Environment } from "@destack/environment";
import { type Connector, ResourceBinding } from "@destack/resource";
import { schema } from "@destack/schema";
import { VAULT_PROVIDER } from "../declare/kind.ts";
import { type Secret, type Vault, SecretReading } from "./secret.ts";
import { createClient, type ClientOf } from "@destack/object/client";
import vaultPackage from "../package.ts";
import { vault } from "../object/vault.ts";

/** A secret's or vault's binding to the vault package, as its connectors read it. */
const VaultBinding = {
    /** Connect to the vault at a binding's address with its credential, in an environment. */
    connect(
        binding: ResourceBinding,
        environment: Environment,
    ): {
        readonly client: ClientOf<typeof vaultPackage>;
        readonly scope: string;
    } {
        // call the vault with the credential the host lends
        const { credential, scope } = ResourceBinding.lent(binding);
        const client = createClient(
            vaultPackage,
            {
                url: binding.reference,
                headers: () => ({ authorization: `Bearer ${credential}` }),
            },
            environment,
        );

        return { client, scope };
    },

    /** Require the version a secret's binding captured. */
    requireVersion(binding: ResourceBinding): number {
        if (binding.version === undefined) {
            throw new TypeError(`secret binding ${binding.resource} has no version`);
        }

        return binding.version;
    },
};

/** Connect a bound secret through the vault package at the bound address. */
export const secretConnector: Connector<Secret> = {
    code: VAULT_PROVIDER,
    connect: async (binding, _declaration, environment) => {
        // read the captured version of the bound secret
        const { client, scope } = VaultBinding.connect(binding, environment);
        const id = schema.identifier("secret").parse(binding.resource);
        const version = VaultBinding.requireVersion(binding);

        return {
            read: async () => SecretReading.parse(await client.secret.read({ scope, id, version })),
            [Symbol.asyncDispose]: async () => {},
        };
    },
};

/** Connect a bound vault through the vault package at the bound address. */
export const vaultConnector: Connector<Vault> = {
    code: VAULT_PROVIDER,
    connect: async (binding, _declaration, environment) => {
        // read a version of the bound vault's secret of a name, the current one by default
        const { client, scope } = VaultBinding.connect(binding, environment);
        const id = schema.identifier("vault").parse(binding.resource);

        return {
            reference: vault.reference(scope, id),
            read: async (name, selection = {}) =>
                SecretReading.parse(await client.vault.read({ scope, id, name, ...selection })),
            [Symbol.asyncDispose]: async () => {},
        };
    },
};

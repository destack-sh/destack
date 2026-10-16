import { type Connector, type ResourceBinding } from "@destack/resource";
import { schema } from "@destack/schema";
import { VAULT_PROVIDER } from "../declare/kind.ts";
import { type Secret, type Vault, SecretReading } from "./secret.ts";
import { createClient, type ClientOf } from "@destack/runtime/client";
import vaultPackage from "../package.ts";
import { vault } from "../object/vault.ts";

/** A secret's or vault's binding to the vault package, as its connectors read it. */
const VaultBinding = {
    /** Connect to the vault at a binding's address with the binding's credential. */
    connect(binding: ResourceBinding): {
        readonly client: ClientOf<typeof vaultPackage>;
        readonly scope: string;
    } {
        // refuse a binding without its credential
        const { credential, scope } = binding;
        if (credential === undefined) {
            throw new TypeError(`${binding.resource} is bound without its credential`);
        }
        // refuse a binding without its scope
        else if (scope === undefined) {
            throw new TypeError(`${binding.resource} is bound without its scope`);
        }

        // call the vault with the credential
        const client = createClient(vaultPackage, {
            url: binding.reference,
            headers: () => ({ authorization: `Bearer ${credential}` }),
        });

        return { client, scope };
    },

    /** Require the version a secret's binding captured. */
    requireVersion(binding: ResourceBinding): number {
        if (binding.version === undefined) {
            throw new TypeError(`secret ${binding.resource} is bound without its version`);
        }

        return binding.version;
    },
};

/** Connect a bound secret through the vault package at the bound address. */
export const secretConnector: Connector<Secret> = {
    code: VAULT_PROVIDER,
    connect: async (binding) => {
        // read the captured version of the bound secret
        const { client, scope } = VaultBinding.connect(binding);
        const id = schema.identifier("secret").parse(binding.resource);
        const version = VaultBinding.requireVersion(binding);
        const captured: Secret = {
            read: async () => SecretReading.parse(await client.secret.read({ scope, id, version })),
        };

        return Object.assign(captured, { [Symbol.asyncDispose]: () => Promise.resolve() });
    },
};

/** Connect a bound vault through the vault package at the bound address. */
export const vaultConnector: Connector<Vault> = {
    code: VAULT_PROVIDER,
    connect: async (binding) => {
        // read a version of the bound vault's secret of a name, the current one by default
        const { client, scope } = VaultBinding.connect(binding);
        const id = schema.identifier("vault").parse(binding.resource);
        const bound: Vault = {
            reference: vault.reference(scope, id),
            read: async (name, selection = {}) =>
                SecretReading.parse(await client.vault.read({ scope, id, name, ...selection })),
        };

        return Object.assign(bound, { [Symbol.asyncDispose]: () => Promise.resolve() });
    },
};

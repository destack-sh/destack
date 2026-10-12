import { type Connector, type ResourceBinding } from "@destack/resource";
import { schema } from "@destack/schema";
import { VAULT_PROVIDER } from "../declare/kind.ts";
import { type Secret, type Vault, SecretReading } from "../secret/secret.ts";
import { vaultService } from "../service/index.ts";
import { VaultClient } from "./client.ts";

/** Connect a bound secret through the vault service at the bound address. */
export const secretConnector: Connector<Secret> = {
    code: VAULT_PROVIDER,
    connect: async (binding) => {
        // read the captured version of the bound secret
        const { client, scope } = connect(binding);
        const id = schema.identifier("secret").parse(binding.resource);
        const version = requireVersion(binding);
        const captured: Secret = {
            read: async () => SecretReading.parse(await client.secret.read({ scope, id, version })),
        };

        return Object.assign(captured, { [Symbol.asyncDispose]: () => Promise.resolve() });
    },
};

/** Connect a bound vault through the vault service at the bound address. */
export const vaultConnector: Connector<Vault> = {
    code: VAULT_PROVIDER,
    connect: async (binding) => {
        // read a version of the bound vault's secret of a name, the current one by default
        const { client, scope } = connect(binding);
        const id = schema.identifier("vault").parse(binding.resource);
        const vault: Vault = {
            read: async (name, selection = {}) =>
                SecretReading.parse(await client.vault.read({ scope, id, name, ...selection })),
        };

        return Object.assign(vault, { [Symbol.asyncDispose]: () => Promise.resolve() });
    },
};

/** Connect to the vault service at a binding's address with the binding's credential. */
function connect(binding: ResourceBinding): {
    readonly client: VaultClient;
    readonly scope: string;
} {
    // require the credential and the scope the binding has
    const { credential, scope } = binding;
    if (credential === undefined) {
        throw new TypeError(`${binding.resource} is bound without its credential`);
    } else if (scope === undefined) {
        throw new TypeError(`${binding.resource} is bound without its scope`);
    }

    // call the service with the credential
    const client = new VaultClient(vaultService, {
        url: binding.reference,
        headers: () => ({ authorization: `Bearer ${credential}` }),
    });

    return { client, scope };
}

/** Require the version a secret's binding captured. */
function requireVersion(binding: ResourceBinding): number {
    if (binding.version === undefined) {
        throw new TypeError(`secret ${binding.resource} is bound without its version`);
    }

    return binding.version;
}

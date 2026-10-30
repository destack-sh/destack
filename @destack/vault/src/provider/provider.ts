import { AccessContext, Caller, principal } from "@destack/access";
import type { Provider, Provisioner } from "@destack/resource";
import type { StoreProvider } from "@destack/stack/server";
import { SecretReading, type Vault } from "../secret/secret.ts";
import { VAULT_PROVIDER, VaultKind } from "../declare/kind.ts";
import { vault } from "../object/index.ts";
import vaultPackage from "../package.ts";

/** The address workloads call the vault package at: its name, which their runtime routes. */
export const VAULT_REFERENCE = vaultPackage.name;

/** The provider of a space's vaults, which keeps them as objects in the space's database. */
export type VaultProvider = Provider<typeof VaultKind, typeof vault> &
    StoreProvider<Vault & AsyncDisposable> & {
        readonly provision: Provisioner<typeof VaultKind>;
    };

/** Provide a space's vaults as objects of its database, under the vault provider code. */
export const vaultProvider: VaultProvider = {
    kind: VaultKind,
    code: VAULT_PROVIDER,
    object: vault,
    provision: {
        // answer the vault package's address for a new vault
        provision: async () => ({ reference: VAULT_REFERENCE }),
        // destroy nothing outside the space's database, which stores the vault's secrets
        destroy: async () => {},
    },
    connect: async (serving, row, bound) => {
        const id = vault.identifier(bound.target);
        const reader = principal.installation.reference(row.scope, row.id);

        return {
            reference: vault.reference(row.scope, id),
            read: async (name, selection = {}) => {
                // read the secret's version as the installation once the store serves
                const store = await serving;
                const read = await store.query(
                    AccessContext.of(Caller.of(reader), store.clock.now()),
                    vault.call("read", {
                        ...vault.target({ scope: row.scope, id }),
                        name,
                        ...selection,
                    }),
                );

                return SecretReading.parse(read);
            },
            [Symbol.asyncDispose]: async () => {},
        };
    },
};

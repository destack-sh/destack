import type { Provider, Provisioner } from "@destack/resource";
import { VAULT_PROVIDER, VaultKind } from "../declare/kind.ts";
import { vault } from "../object/index.ts";
import vaultPackage from "../package.ts";

/** The provider of a space's vaults, which keeps them as objects in the space's database. */
export type VaultProvider = Provider<typeof VaultKind, typeof vault> & {
    readonly provision: Provisioner<typeof VaultKind>;
};

/** Provide a space's vaults as objects of its database, under the vault provider code. */
export const vaultProvider: VaultProvider = {
    kind: VaultKind,
    code: VAULT_PROVIDER,
    object: vault,
    provision: {
        // answer the vault's address for a new vault: the package's name
        provision: async () => ({ reference: vaultPackage.name }),
        // leave a vault's secrets to their own restriction, which refuses deleting a vault keeping any
        destroy: async () => {},
    },
};

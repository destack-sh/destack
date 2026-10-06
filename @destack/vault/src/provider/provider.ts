import type { Provider, Provisioner, Rewrapper } from "@destack/resource";
import { VaultKind } from "../declare/kind.ts";
import type { KeyringVaultHost } from "../key/index.ts";
import { vault } from "../server/secret.ts";
import { vaultKey } from "../stack/db.ts";

/** A vault provider: provisioning one host's vaults and sealing their keys to the host a space moves to. */
export type VaultProvider = Provider<
    typeof VaultKind,
    typeof vault,
    never,
    typeof vaultKey,
    typeof vaultKey.$inferSelect
> & {
    readonly provision: Provisioner<typeof VaultKind>;
    readonly rewrap: Rewrapper<typeof vaultKey, typeof vaultKey.$inferSelect>;
};

/** Provide a host's vaults through their keystore under its provider code, their secrets served by the vault service. */
export function vaultProvider(keystore: KeyringVaultHost): VaultProvider {
    return {
        kind: VaultKind,
        code: keystore.provider,
        object: vault,
        provision: keystore,
        rewrap: keystore.rewrap,
    };
}

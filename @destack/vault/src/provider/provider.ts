import type { Select } from "@destack/db";
import type { Provider, Provisioner, Rewrapper } from "@destack/resource";
import { VaultKind } from "../declare/kind.ts";
import type { VaultKeystore } from "../key/index.ts";
import { vault, vaultKey } from "../object/index.ts";

/** The provider of one machine's vaults. */
export type VaultProvider = Provider<
    typeof VaultKind,
    typeof vault,
    never,
    typeof vaultKey.table,
    Select<typeof vaultKey.table>
> & {
    readonly provision: Provisioner<typeof VaultKind>;
    readonly rewrap: Rewrapper<typeof vaultKey.table, Select<typeof vaultKey.table>>;
};

/** Provide a machine's vaults through their keystore under its provider code. */
export function vaultProvider(keystore: VaultKeystore): VaultProvider {
    return {
        kind: VaultKind,
        code: keystore.provider,
        object: vault,
        provision: keystore,
        rewrap: keystore.rewrap,
    };
}

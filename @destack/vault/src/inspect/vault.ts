import { type VaultDescription, VaultKind } from "../declare/kind.ts";
import type { VaultDeclaration } from "../declare/vault.ts";

/** Describe a declared vault for the package manifest. */
export function describeVault(vault: VaultDeclaration): VaultDescription {
    return VaultKind.description.parse({ name: vault.name, kind: vault.kind, spec: vault.spec });
}

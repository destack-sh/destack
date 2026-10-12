import type { ResourceDeclaration } from "@destack/resource";
import { type VaultDescription, VaultKind } from "../declare/kind.ts";

/** Describe a declared vault for the package manifest. */
export function describeVault(
    vault: ResourceDeclaration<unknown, VaultDescription>,
): VaultDescription {
    return VaultKind.description.parse({ name: vault.name, kind: vault.kind, spec: vault.spec });
}

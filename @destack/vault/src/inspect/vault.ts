import type { ResourceDeclaration } from "@destack/resource";
import { type VaultDescription, VaultKind, type VaultSpec } from "../declare/kind.ts";

/** Describe a declared vault for the package manifest. */
export function describeVault(
    vault: Pick<ResourceDeclaration<unknown, typeof VaultSpec>, "name" | "spec">,
): VaultDescription {
    return VaultKind.description.parse({ name: vault.name, spec: vault.spec });
}

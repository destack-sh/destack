import { ModuleMetadata } from "@destack/package";
import { ResourceDeclaration } from "@destack/resource";
import { vaultConnector } from "../secret/connector.ts";
import type { Vault } from "../secret/secret.ts";
import { VaultKind, type VaultSpec } from "./kind.ts";

/** Declare a vault resource, whose secrets its workloads read through the vault package. */
export function defineVault(
    options: { readonly name: string },
    module?: ModuleMetadata,
): ResourceDeclaration<Vault, typeof VaultSpec> {
    const owner = ModuleMetadata.require(module, "defineVault").package;

    return new ResourceDeclaration<Vault, typeof VaultSpec>(owner, VaultKind, {
        name: options.name,
        spec: {},
        connectors: { [vaultConnector.code]: vaultConnector },
    });
}

import { ModuleMetadata } from "@destack/package";
import { type Connector, ResourceDeclaration } from "@destack/resource";
import { vaultConnector } from "../connector/connector.ts";
import type { Vault } from "../secret/secret.ts";
import { type VaultDescription, VaultKind } from "./kind.ts";

/** A package's vault, whose secrets its workloads read through the vault service. */
export class VaultDeclaration extends ResourceDeclaration<Vault, VaultDescription> {
    /** The connector reading the vault's secrets at the bound address, by provider code. */
    override get connectors(): Readonly<Record<string, Connector<Vault>>> {
        return { [vaultConnector.code]: vaultConnector };
    }
}

/** Declare a vault resource, whose secrets its workloads read through the vault service. */
export function defineVault(
    declaration: Omit<VaultDescription, "kind">,
    module?: ModuleMetadata,
): VaultDeclaration {
    const owner = ModuleMetadata.require(module, "defineVault").package;

    return new VaultDeclaration(
        owner,
        VaultKind.description.parse({ ...declaration, kind: VaultKind.name }),
    );
}

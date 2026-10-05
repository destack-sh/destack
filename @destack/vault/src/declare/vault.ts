import { ModuleMetadata } from "@destack/package";
import { type Connector, ResourceDeclaration, type ResourceBinding } from "@destack/resource";
import { spaceService } from "@destack/space/service";
import { SecretClient } from "../object/index.ts";
import { VaultKind, type VaultDescription } from "./kind.ts";

/** The provider code of vaults, whose secrets the space's own service serves. */
const VAULT_PROVIDER = "vault";

/** A package's vault, whose secrets its workloads reach through their space's service. */
class VaultDeclaration extends ResourceDeclaration<SecretClient, VaultDescription> {
    /** The connector reaching the vault's secrets at the bound address as the installation. */
    override get connectors(): { readonly vault: Connector<SecretClient> } {
        return {
            vault: {
                code: VAULT_PROVIDER,
                connect: async (binding: ResourceBinding) => {
                    // require the credential the host lends the binding
                    const { credential } = binding;
                    if (credential === undefined) {
                        throw new TypeError(`vault ${this.name} is bound without its credential`);
                    }

                    // reach the space's service as the installation, disposing nothing
                    const client = new SecretClient(spaceService, {
                        url: binding.reference,
                        headers: () => ({ authorization: `Bearer ${credential}` }),
                    });

                    return Object.assign(client, {
                        [Symbol.asyncDispose]: () => Promise.resolve(),
                    });
                },
            },
        };
    }
}

/** Declare a vault resource. */
export function defineVault(
    declaration: Omit<VaultDescription, "kind">,
    module?: ModuleMetadata,
): ResourceDeclaration<SecretClient, VaultDescription> {
    const owner = ModuleMetadata.require(module, "defineVault").package;

    return new VaultDeclaration(
        owner,
        VaultKind.description.parse({ ...declaration, kind: "vault" }),
    );
}

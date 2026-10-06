import { ModuleMetadata } from "@destack/package";
import { type Connector, ResourceDeclaration, type ResourceBinding } from "@destack/resource";
import { vaultService } from "../service/index.ts";
import { SecretClient } from "../object/index.ts";
import { VAULT_PROVIDER, VaultKind, type VaultDescription } from "./kind.ts";

/** A package's vault, whose secrets its workloads reach through the vault service. */
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

                    // reach the vault service as the installation
                    return Object.assign(installationClient(binding.reference, credential), {
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

/** Reach the vault service at a bound address as the installation a credential names. */
export function installationClient(reference: string, credential: string): SecretClient {
    return new SecretClient(vaultService, {
        url: reference,
        headers: () => ({ authorization: `Bearer ${credential}` }),
    });
}

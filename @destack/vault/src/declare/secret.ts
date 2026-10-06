import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import {
    type Connector,
    ResourceDeclaration,
    type ResourceBinding,
    SECRET_KIND,
} from "@destack/resource";
import { Secret } from "../secret/client.ts";
import { SecretGeneration } from "../secret/secret.ts";
import { VAULT_PROVIDER } from "./kind.ts";
import { installationClient } from "./vault.ts";

/** A secret a package declares, which a space binds to one of its secrets or generates at install. */
export const SecretDescription = defineSchema(
    schema.object({
        /** The package-local secret name. */
        name: DeclarationName,
        /** How the space generates the first version at install, absent for a secret a stack binds. */
        generated: SecretGeneration.exactOptional(),
    }),
);
/** A secret a package declares, which a space binds to one of its secrets or generates at install. */
export type SecretDescription = schema.Infer<typeof SecretDescription>;

/** A package's declaration of a secret it reads, bound to the version its deployment captured. */
class SecretDeclaration extends ResourceDeclaration<
    Secret,
    { readonly name: string; readonly kind: string; readonly spec: Record<string, never> }
> {
    /** How the space generates the first version at install, absent for a secret a stack binds. */
    readonly generated?: SecretGeneration;

    /** Retain validated metadata without acquiring credentials. */
    constructor(owner: Package, declaration: SecretDescription) {
        super(owner, { name: declaration.name, kind: SECRET_KIND, spec: {} });
        if (declaration.generated !== undefined) {
            this.generated = declaration.generated;
        }
    }

    /** The connector reading the captured version through the vault service at the bound address. */
    override get connectors(): { readonly vault: Connector<Secret> } {
        return {
            vault: {
                code: VAULT_PROVIDER,
                connect: async (binding: ResourceBinding) => {
                    // require the credential, space and version the host binds
                    const { credential, scope, version } = binding;
                    if (credential === undefined || scope === undefined || version === undefined) {
                        throw new TypeError(`secret ${this.name} is bound without its version`);
                    }

                    // read through the vault service as the installation
                    const captured = { scope, target: binding.resource, version };
                    const client = installationClient(binding.reference, credential);

                    return Object.assign(new Secret(client, captured), {
                        [Symbol.asyncDispose]: () => Promise.resolve(),
                    });
                },
            },
        };
    }
}

/** Declare a secret without embedding its value, generated at install when it names how. */
export function defineSecret(
    declaration: SecretDescription,
    module?: ModuleMetadata,
): SecretDeclaration {
    const owner = ModuleMetadata.require(module, "defineSecret").package;

    return new SecretDeclaration(owner, SecretDescription.parse(declaration));
}

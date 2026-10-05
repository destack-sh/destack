import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import {
    type Connector,
    ResourceDeclaration,
    type ResourceBinding,
    SECRET_KIND,
    SECRET_PROVIDER,
} from "@destack/resource";
import { spaceService } from "@destack/space/service";
import { Secret } from "../secret/client.ts";
import { SecretClient } from "../object/index.ts";

/** The algorithms a space generates a secret's first version with: ES256 writes an ECDSA P-256 private key as a JWK (RFC 7518 6.2). */
export const SECRET_ALGORITHMS = ["ES256"] as const;

/** An algorithm a space generates a secret's first version with. */
export type SecretAlgorithm = (typeof SECRET_ALGORITHMS)[number];

/** How a space generates a secret's first version when it installs the package: in one of the package's vaults, with an algorithm. */
export const SecretGeneration = defineSchema(
    schema.object({
        /** The package's vault declaration keeping the secret. */
        vault: DeclarationName,
        /** The algorithm writing the first version. */
        algorithm: schema.enum(SECRET_ALGORITHMS),
    }),
);
/** How a space generates a secret's first version when it installs the package. */
export type SecretGeneration = schema.Infer<typeof SecretGeneration>;

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

    /** The connector reading the captured version through the space's service at the bound address. */
    override get connectors(): { readonly space: Connector<Secret> } {
        return {
            space: {
                code: SECRET_PROVIDER,
                connect: async (binding: ResourceBinding) => {
                    // require the credential, the space and the version the host binds
                    const { credential, scope, version } = binding;
                    if (credential === undefined || scope === undefined || version === undefined) {
                        throw new TypeError(`secret ${this.name} is bound without its version`);
                    }

                    // read through the space's service as the installation, disposing nothing
                    const client = new SecretClient(spaceService, {
                        url: binding.reference,
                        headers: () => ({ authorization: `Bearer ${credential}` }),
                    });
                    const captured = { scope, target: binding.resource, version };

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

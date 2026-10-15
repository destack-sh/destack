import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import { type Connector, ResourceDeclaration } from "@destack/resource";
import { defineSchema, schema } from "@destack/schema";
import { secretConnector } from "../secret/connector.ts";
import { type Secret, SecretGeneration } from "../secret/secret.ts";
import { SECRET_KIND } from "./kind.ts";

/** A secret a package declares. */
export const SecretDescription = Object.assign(
    defineSchema(
        schema.object({
            /** The package-local secret name. */
            name: DeclarationName,
            /** How the space generates the first version at install. */
            generated: SecretGeneration.exactOptional(),
        }),
    ),
    {
        /** Read how a declared secret's spec generates it, absent for a secret a stack binds. */
        generation(declared: {
            readonly name: string;
            readonly spec: Readonly<Record<string, unknown>>;
        }): SecretGeneration | undefined {
            return SecretDescription.parse({ ...declared.spec, name: declared.name }).generated;
        },
    },
);
/** A secret a package declares. */
export type SecretDescription = schema.Infer<typeof SecretDescription>;

/** A package's declaration of a secret it reads, bound to the version its deployment captured. */
export class SecretDeclaration extends ResourceDeclaration<
    Secret,
    { readonly name: string; readonly kind: string; readonly spec: Record<string, never> }
> {
    /** How the space generates the first version at install, absent for a secret a stack binds. */
    readonly generated?: SecretGeneration;

    /** Keep a validated secret declaration of a package. */
    constructor(owner: Package, declaration: SecretDescription) {
        super(owner, { name: declaration.name, kind: SECRET_KIND, spec: {} });
        if (declaration.generated !== undefined) {
            this.generated = declaration.generated;
        }
    }

    /** The connector reading the captured version at the bound address, by provider code. */
    override get connectors(): Readonly<Record<string, Connector<Secret>>> {
        return { [secretConnector.code]: secretConnector };
    }
}

/** Declare a secret a package reads, generated at install when it says how. */
export function defineSecret(
    declaration: SecretDescription,
    module?: ModuleMetadata,
): SecretDeclaration {
    const owner = ModuleMetadata.require(module, "defineSecret").package;

    return new SecretDeclaration(owner, SecretDescription.parse(declaration));
}

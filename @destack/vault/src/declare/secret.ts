import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import { ResourceHandle } from "@destack/resource";
import type { Secret } from "../secret/client.ts";

/** A secret a package declares, which a space binds to one of its secrets. */
export const SecretDescription = defineSchema(
    schema.object({
        /** The package-local secret name. */
        name: DeclarationName,
    }),
);
/** A secret a package declares, which a space binds to one of its secrets. */
export type SecretDescription = schema.Infer<typeof SecretDescription>;

/** A package's declaration of a secret it reads. */
class SecretDeclaration extends ResourceHandle<Secret> {
    /** Retain validated metadata without acquiring credentials. */
    constructor(owner: Package, declaration: SecretDescription) {
        super(owner, declaration.name);
    }
}

/** Declare a secret without embedding its value. */
export function defineSecret(
    declaration: SecretDescription,
    module?: ModuleMetadata,
): SecretDeclaration {
    const owner = ModuleMetadata.require(module, "defineSecret").package;

    return new SecretDeclaration(owner, SecretDescription.parse(declaration));
}

import { defineSchema, identifier, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { ResourceHandle } from "@destack/resource";
import { DeclarationName } from "@destack/package";
import type { BoundSecret } from "../secret/client.ts";

/** A secret selected when installing a package. */
export const SecretDescription = defineSchema(
    schema.object({
        /** The package-local secret name. */
        name: DeclarationName,
        /** The declaration format version. */
        version: schema.literal(1),
    }),
);
/** A secret selected when installing a package. */
export type SecretDescription = schema.Infer<typeof SecretDescription>;

/** A stored secret and version selection. */
export const SecretReference = defineSchema(
    schema.object({
        /** The space administering the vault. */
        space: identifier("space"),
        /** The secret identifier. */
        secret: identifier("secret"),
        /** An exact version; omit to select the current version at access time. */
        version: schema.number().int().positive().optional(),
    }),
);
/** A stored secret and version selection. */
export type SecretReference = schema.Infer<typeof SecretReference>;

/** An inert secret declaration with host-authorized value access. */
class Secret extends ResourceHandle<BoundSecret> {
    /** Declaration format version. */
    readonly version: SecretDescription["version"];

    /** Retain validated metadata without acquiring credentials. */
    constructor(owner: Package, declaration: SecretDescription) {
        super(owner, declaration.name);
        this.version = declaration.version;
    }
}

/** Declare a secret without embedding its value. */
export function defineSecret(
    declaration: Omit<SecretDescription, "version">,
    module?: ModuleMetadata,
): Secret {
    const owner = declaringModule(module, "defineSecret").package;

    return new Secret(owner, SecretDescription.parse({ ...declaration, version: 1 }));
}

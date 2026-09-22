import { defineSchema, identifier, schema } from "@destack/schema";
import { ResourceName, ResourceHandle } from "@destack/resource";
import type { BoundSecret } from "../secret/client.ts";

/** A secret selected when installing a package. */
export const SecretDeclaration = defineSchema(
    schema.object({
        /** The package-local secret name. */
        name: ResourceName,
        /** The declaration format version. */
        version: schema.literal(1),
    }),
);
/** A secret selected when installing a package. */
export type SecretDeclaration = schema.Infer<typeof SecretDeclaration>;

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
    readonly version: SecretDeclaration["version"];

    /** Retain validated metadata without acquiring credentials. */
    constructor(declaration: SecretDeclaration) {
        super(declaration.name);
        this.version = declaration.version;
    }
}

/** Declare a secret without embedding its value. */
export function defineSecret(declaration: Omit<SecretDeclaration, "version">): Secret {
    return new Secret(SecretDeclaration.parse({ ...declaration, version: 1 }));
}

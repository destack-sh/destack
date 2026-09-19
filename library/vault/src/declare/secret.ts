import { defineSchema, identifier, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";

/** A secret selected when installing a package. */
export const SecretDeclaration = defineSchema(schema.object({
    /** The package-local secret name. */
    name: ResourceName,
    /** The declaration format version. */
    version: schema.literal(1),
}));
/** A secret selected when installing a package. */
export type SecretDeclaration = schema.Infer<typeof SecretDeclaration>;

/** A stored secret and version selection. */
export const SecretReference = defineSchema(schema.object({
    /** The space administering the vault. */
    space: identifier("space"),
    /** The secret identifier. */
    secret: identifier("secret"),
    /** An exact version; omit to select the current version at access time. */
    version: schema.number().int().positive().optional(),
}));
/** A stored secret and version selection. */
export type SecretReference = schema.Infer<typeof SecretReference>;

/** Declare a secret without embedding its value. */
export function defineSecret(
    declaration: Omit<SecretDeclaration, "version">,
): SecretDeclaration {
    return SecretDeclaration.parse({ ...declaration, version: 1 });
}

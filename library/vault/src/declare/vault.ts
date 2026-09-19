import { defineSchema, schema } from "@destack/schema";
import { defineResourceSchema } from "@destack/resource";

/** A managed collection of encrypted secrets. */
export const VaultSpec = defineSchema(schema.object({}));
/** A vault resource dependency. */
export const VaultDeclaration = defineResourceSchema("vault", 1, VaultSpec);
/** A vault resource dependency. */
export type VaultDeclaration = schema.Infer<typeof VaultDeclaration>;

/** Declare a vault resource. */
export function defineVault(
    declaration: Omit<VaultDeclaration, "kind" | "version">,
): VaultDeclaration {
    return VaultDeclaration.parse({ ...declaration, kind: "vault", version: 1 });
}

import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { connect } from "../secret/client.ts";

/** A managed collection of encrypted secrets. */
export const VaultSpec = defineSchema(schema.object({}));
/** A vault resource dependency. */
export const VaultDescription = defineResourceSchema("vault", 1, VaultSpec);
/** A vault resource dependency. */
export type VaultDescription = schema.Infer<typeof VaultDescription>;

/** Declare a vault resource. */
export function defineVault(
    declaration: Omit<VaultDescription, "kind" | "version">,
    module?: ModuleMetadata,
): Resource<ReturnType<typeof connect>, VaultDescription> {
    const owner = declaringModule(module, "defineVault").package;

    return new Resource(
        owner,
        VaultDescription.parse({ ...declaration, kind: "vault", version: 1 }),
    );
}

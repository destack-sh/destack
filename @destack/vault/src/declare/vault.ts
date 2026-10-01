import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata } from "@destack/package";
import { defineResourceKind, Resource } from "@destack/resource";
import type { connect } from "../secret/client.ts";

/** A managed collection of encrypted secrets. */
export const VaultSpec = defineSchema(schema.object({}));
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** A vault resource dependency. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

/** Declare a vault resource. */
export function defineVault(
    declaration: Omit<VaultDescription, "kind">,
    module?: ModuleMetadata,
): Resource<ReturnType<typeof connect>, VaultDescription> {
    const owner = declaringModule(module, "defineVault").package;

    return new Resource(owner, VaultKind.description.parse({ ...declaration, kind: "vault" }));
}

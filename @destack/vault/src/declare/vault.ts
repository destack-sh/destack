import { defineSchema, schema } from "@destack/schema";
import { ModuleMetadata } from "@destack/package";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import type { SecretClient } from "../object/index.ts";

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
): ResourceDeclaration<SecretClient, VaultDescription> {
    const owner = ModuleMetadata.require(module, "defineVault").package;

    return new ResourceDeclaration(
        owner,
        VaultKind.description.parse({ ...declaration, kind: "vault" }),
    );
}

import { schema } from "@destack/schema";
import { defineResourceKind } from "@destack/resource";

/** The provider code of vaults and secrets. */
export const VAULT_PROVIDER = "vault";

/** The spec of a vault resource, which has no settings. */
export const VaultSpec = schema.object({});
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** A vault resource a package declares. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

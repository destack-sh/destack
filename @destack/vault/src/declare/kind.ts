import { schema } from "@destack/schema";
import { defineKind } from "@destack/package";

/** The provider code of vaults and secrets. */
export const VAULT_PROVIDER = "vault";

/** The spec of a vault resource, which has no settings. */
export const VaultSpec = schema.object({});
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineKind({ name: "vault", provisioned: { spec: VaultSpec } });
/** A vault resource a package declares. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

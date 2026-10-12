import { defineSchema, schema } from "@destack/schema";
import { defineResourceKind } from "@destack/resource";

/** The declaration kind of secrets, which workloads bind beside resources. */
export const SECRET_KIND = "secret";

/** The provider code of vaults and secrets. */
export const VAULT_PROVIDER = "vault";

/** The spec of a vault resource, which has no settings. */
export const VaultSpec = defineSchema(schema.object({}));
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** A vault resource a package declares. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

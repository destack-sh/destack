import { defineSchema, schema } from "@destack/schema";
import { defineResourceKind, SECRET_PROVIDER } from "@destack/resource";

/** The provider code of vaults and their secrets, which the vault service serves. */
export const VAULT_PROVIDER = SECRET_PROVIDER;

/** A managed collection of encrypted secrets. */
export const VaultSpec = defineSchema(schema.object({}));
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** A vault resource dependency. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

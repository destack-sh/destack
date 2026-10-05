import { defineSchema, schema } from "@destack/schema";
import { defineResourceKind } from "@destack/resource";

/** A managed collection of encrypted secrets. */
export const VaultSpec = defineSchema(schema.object({}));
/** The vault resource kind: encrypted secrets. */
export const VaultKind = defineResourceKind("vault", { spec: VaultSpec });
/** A vault resource dependency. */
export type VaultDescription = schema.Infer<typeof VaultKind.description>;

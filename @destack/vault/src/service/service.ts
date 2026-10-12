import { defineService } from "@destack/service";
import { secret, secretVersion, vault, vaultKey } from "../object/index.ts";

/** The vaults of the spaces a machine serves, with their key versions, secrets and versions. */
export const vaultService = defineService("vault", {
    objects: { vault, vaultKey, secret, version: secretVersion },
});

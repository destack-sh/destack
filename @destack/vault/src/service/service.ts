import { defineService } from "@destack/service";
import { secret, secretVersion, vault } from "../object/index.ts";

/** The vaults of the spaces a cell serves, with their secrets and versions, served as objects. */
export const vaultService = defineService("vault", {
    objects: { vault, secret, version: secretVersion },
});

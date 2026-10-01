import { defineService } from "@destack/service";
import { secret, secretVersion, vault } from "../object/index.ts";

/** Vaults, secrets and versions served as objects. */
export const vaultService = defineService("vault", {
    objects: { vault, secret, version: secretVersion },
});

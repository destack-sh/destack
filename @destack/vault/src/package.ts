import { definePackage } from "@destack/object";
import { VaultError } from "./error/error.ts";
import { secret, secretVersion, vault } from "./object/index.ts";

/** The vault package, which machines serve and clients call. */
export default definePackage({
    objects: { vault, secret, version: secretVersion },
    errors: VaultError,
    permissions: {
        installation: {
            // provision the space's vaults as their provider, observing each one ready
            space: [vault.permission("control")],
        },
    },
});

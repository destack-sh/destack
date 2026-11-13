import { t } from "@destack/locale";
import { definePackage } from "@destack/object";
import { VaultError } from "./error/error.ts";
import { secret, secretVersion, vault } from "./object/index.ts";

/** The vault package, which machines serve and clients call. */
export default definePackage({
    title: t`Vault`,
    description: t`Keep secrets as encrypted versions and rotate them without redeploying.`,
    icon: "vault",
    color: "blue",
    objects: [vault, secret, secretVersion],
    errors: VaultError,
    access: {
        permissions: {
            space: {
                // provision the space's vaults as their provider, observing each one ready
                space: [vault.permission("control")],
            },
        },
    },
});

import { implementPackage } from "@destack/object/server";
import { vaultProvider } from "./provider/index.ts";
import vaultPackage from "./package.ts";
import { implementSecret, implementSecretVersion, secretBindable } from "./server/secret.ts";
import { implementVault } from "./server/vault.ts";

export * from "./server/index.ts";

/** The vault package's server: vaults, secrets, their provider and their bindable. */
export default implementPackage(vaultPackage, {
    objects: [implementVault(), implementSecret(), implementSecretVersion()],
    providers: [vaultProvider],
    bindables: [secretBindable],
});

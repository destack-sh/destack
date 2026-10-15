import { defineServer } from "@destack/object/server";
import { vaultProvider } from "./provider/index.ts";
import vaultPackage from "./package.ts";
import { implementSecret, implementSecretVersion } from "./server/secret.ts";
import { implementVault } from "./server/vault.ts";

export * from "./server/index.ts";

/** The vault package's server: vaults, their secrets and versions, and the vault provider. */
export default defineServer(vaultPackage, {
    objects: [implementVault(), implementSecret(), implementSecretVersion()],
    providers: [vaultProvider],
});

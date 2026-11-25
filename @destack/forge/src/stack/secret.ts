import { defineVault } from "@destack/vault/declare";

/** The forge's own vault, storing the credentials of Git remotes that only the forge reads. */
export const credentialVault = defineVault({ name: "credentials" });

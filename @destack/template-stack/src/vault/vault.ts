import { defineVault } from "@destack/vault/declare";

/** The vault containing application secrets. */
export const credentials = defineVault({ name: "credentials", spec: {} });

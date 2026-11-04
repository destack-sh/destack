import { defineVault } from "@destack/vault/declare";

/** The vault of an account's home that its connections keep their credentials in. */
export const connectionVault = defineVault({ name: "credentials", spec: {} });

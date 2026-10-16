import { defineSecret, defineVault } from "@destack/vault/declare";

/** The forge's own vault, storing the credentials of Git remotes that only the forge reads. */
export const credentialVault = defineVault({ name: "credentials", spec: {} });

/** The secret GitHub signs the forge's GitHub App's webhook deliveries with. */
export const githubWebhookSecret = defineSecret({ name: "github-webhook" });

/** The private key of the forge's GitHub App, a PKCS #1 or PKCS #8 PEM. */
export const githubPrivateKey = defineSecret({ name: "github-private-key" });

import { defineSecret } from "@destack/vault/declare";

/** The secret GitHub signs the repository GitHub App's webhook deliveries with. */
export const githubWebhookSecret = defineSecret({ name: "github-webhook" });

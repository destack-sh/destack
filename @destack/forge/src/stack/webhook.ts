import { defineWebhook } from "@destack/workflow";
import type {} from "@destack/package/import-meta";
import { GitHubSignature } from "../github/signature.ts";
import { githubWebhookSecret } from "./secret.ts";

/** The deliveries of the forge's GitHub App, signed with the secret its stack binds. */
export const githubWebhook = defineWebhook("github", {
    signature: new GitHubSignature(),
    secret: githubWebhookSecret,
});

import { definePackage } from "@destack/package/declare";
import { githubWebhookSecret } from "./stack/secret.ts";

/** The handle stacks import to install this package. */
export default definePackage({ secrets: { "github-webhook": githubWebhookSecret } });

import { BUILD_STORE_TABLES } from "@destack/build/table";
import { defineAdapter, definePackage } from "@destack/object";
import { githubWebhook } from "./stack/webhook.ts";
import { appInstallation, issuer, unit } from "./object/index.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhookSecret,
    repositoryStorage,
} from "./stack/index.ts";
import { ForgeError } from "./error/error.ts";

export { FORGE_INSTALLATION } from "./client/installation.ts";

/** The handle stacks install the forge by, and clients call its packages and repositories by. */
export default definePackage({
    objects: {
        unit,
        issuer,
        appInstallation,
    },
    // the store of pushed builds, indexed beside the forge's objects
    tables: BUILD_STORE_TABLES,
    adapters: {
        builds: defineAdapter("builds", { path: "/builds" }),
        github: githubWebhook,
    },
    resources: {
        repositories: repositoryStorage,
        credentials: credentialVault,
    },
    secrets: { "github-webhook": githubWebhookSecret, "github-private-key": githubPrivateKey },
    capabilities: {
        // call GitHub's API and the Git remotes repositories mirror
        mirror: {
            reason: "Call GitHub and the Git remotes your repositories mirror",
            sandbox: { network: { connect: ["*"] } },
        },
    },
    errors: ForgeError,
});

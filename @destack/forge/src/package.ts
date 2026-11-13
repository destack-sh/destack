import registryPackage from "@destack/registry/package";
import { BUILD_STORE_TABLES } from "@destack/build/table";
import { defineMount, definePackage } from "@destack/object";
import { githubWebhook } from "./stack/webhook.ts";
import { appInstallation, issuer } from "./object/index.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhookSecret,
    repositoryStorage,
} from "./stack/index.ts";
import { ForgeError } from "./error/error.ts";
import { t } from "@destack/locale";

export { FORGE_INSTALLATION } from "./client/installation.ts";

/** The handle stacks install the forge by, and clients call its packages and repositories by. */
export default definePackage({
    title: t`Forge`,
    description: t`Host the Git repositories of accounts and publish, resolve and distribute their Destack packages.`,
    icon: "git-branch",
    color: "green",
    objects: [issuer, appInstallation],
    // the store of pushed builds, indexed beside the forge's objects
    tables: BUILD_STORE_TABLES,
    // serve the registry's repositories, references, packages and tags
    protocols: [registryPackage],
    mounts: [defineMount({ name: "builds", path: "/builds" }), githubWebhook],
    resources: [repositoryStorage, credentialVault, githubWebhookSecret, githubPrivateKey],
    capabilities: [
        {
            name: "mirror",
            reason: t`Call GitHub and the Git remotes your repositories mirror`,
            sandbox: { network: { connect: ["*"] } },
        },
    ],
    errors: ForgeError,
});

import { defineAdapter, definePackage } from "@destack/runtime";
import { issuer, packageObject, reference, release, repository, tag } from "./object/index.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhookSecret,
    packageBucket,
    repositoryStorage,
} from "./stack/index.ts";

/** The name the platform stack installs the forge under in each residency home. */
export const FORGE_INSTALLATION = "forge";

/** The handle stacks install the forge by, and clients call its packages and repositories by. */
export default definePackage({
    objects: { repository, reference, package: packageObject, release, tag, issuer },
    adapters: {
        npm: defineAdapter("npm", { path: "/npm" }),
        builds: defineAdapter("builds", { path: "/builds" }),
        github: defineAdapter("github", { path: "/github" }),
    },
    resources: {
        packages: packageBucket,
        repositories: repositoryStorage,
        credentials: credentialVault,
    },
    secrets: { "github-webhook": githubWebhookSecret, "github-private-key": githubPrivateKey },
});

import { defineAdapter, definePackage } from "@destack/object";
import {
    appInstallation,
    issuer,
    packageObject,
    reference,
    release,
    repository,
    tag,
    unit,
} from "./object/index.ts";
import {
    credentialVault,
    githubPrivateKey,
    githubWebhookSecret,
    packageBucket,
    repositoryStorage,
} from "./stack/index.ts";
import { ForgeError } from "./error/error.ts";

/** The name the platform stack installs the forge under in each residency home. */
export const FORGE_INSTALLATION = "forge";

/** The handle stacks install the forge by, and clients call its packages and repositories by. */
export default definePackage({
    objects: {
        repository,
        reference,
        package: packageObject,
        release,
        tag,
        unit,
        issuer,
        appInstallation,
    },
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
    capabilities: {
        // call GitHub's API and the Git remotes repositories mirror
        mirror: {
            reason: "Call GitHub and the Git remotes your repositories mirror",
            sandbox: { network: { connect: ["*"] } },
        },
    },
    errors: ForgeError,
});

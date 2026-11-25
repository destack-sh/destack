import registryPackage from "@destack/registry/package";
import { BUILD_STORE_TABLES } from "@destack/build/table";
import { defineMount, definePackage } from "@destack/object";
import { credentialVault, repositoryStorage } from "./stack/index.ts";
import { ForgeError } from "./error/error.ts";
import { t } from "@destack/locale";

export { FORGE_INSTALLATION } from "./client/installation.ts";

/** The handle stacks install the forge by, and clients call its packages and repositories by. */
export default definePackage({
    title: t`Forge`,
    description: t`Host the Git repositories of accounts and publish, resolve and distribute their Destack packages.`,
    icon: "git-branch",
    color: "green",
    // the store of pushed builds, indexed beside the forge's objects
    tables: BUILD_STORE_TABLES,
    // serve the registry's repositories, references, packages and tags
    protocols: [registryPackage],
    mounts: [
        defineMount({ name: "builds", path: "/builds" }),
        defineMount({ name: "npm", path: "/.npm" }),
        defineMount({ name: "git", path: "/git" }),
    ],
    resources: [repositoryStorage, credentialVault],
    capabilities: [
        {
            name: "mirror",
            reason: t`Call GitHub and the Git remotes your repositories mirror`,
            sandbox: { network: { connect: ["*"] } },
        },
    ],
    errors: ForgeError,
});

import type { Environment } from "@destack/environment";
import { connectionShape } from "@destack/connection/server";
import connectionPackage from "@destack/connection/package";
import connectionServer from "@destack/connection/server";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import forgeServer, { claimFollow, standingFollow } from "@destack/forge/server";
import { credentialVault, github, repositoryStorage } from "@destack/forge/stack";
import { LocalGitStorage, localGitProvider } from "@destack/git/local";
import { testBuild } from "@destack/object/test";
import { describeSetting } from "@destack/setting/inspect";
import settingPackage from "@destack/setting/package";
import { gitStorage } from "@destack/stack/object";
import { testHost } from "@destack/test/host";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { describeSecret } from "@destack/vault/inspect";
import vaultServer from "@destack/vault/server";
import { install, SECRET_KIND } from "@destack/stack/declare";
import { TestPackage } from "./package.ts";

/** The vault the residency's stack declares for the platform's secrets. */
const PLATFORM_VAULT = defineVault({ name: "platform" });

/** The vault as the machine ships it, keeping the secrets of a space's installations. */
export const VAULT: TestPackage = {
    handle: vaultPackage,
    build: await testBuild(vaultPackage, testHost().crypto),
    server: vaultServer,
    connectors: [],
    providers: [],
};

/** Connections as the machine ships them, beside the vault keeping their credentials. */
export const CONNECTION: TestPackage = {
    handle: connectionPackage,
    build: await testBuild(connectionPackage, testHost().crypto),
    server: connectionServer,
    connectors: [],
    providers: [],
};

/** The forge as the machine ships it, beside the vault, with Git storage below its files. */
export const FORGE: TestPackage = {
    handle: forgePackage,
    build: await testBuild(forgePackage, testHost().crypto, {
        declared: [
            {
                kind: "setting",
                kindPackageId: settingPackage.id,
                name: github.name,
                description: describeSetting(github),
            },
            ...Object.values(forgePackage.secrets).map((declared) => ({
                ...SECRET_KIND.identity,
                name: declared.name,
                description: describeSecret(declared),
            })),
        ],
    }),
    server: forgeServer,
    connectors: [],
    providers: [],
};

/** The packages whose types the residency home's stack declares. */
export const FORGE_HOME_TYPES = [vaultPackage, forgePackage, gitStorage] as const;

/** The residency home's stack: the forge with its resources and issuer. */
export const FORGE_HOME_STACK = {
    vaults: { platform: PLATFORM_VAULT.provision(), credentials: credentialVault.provision() },
    gitStorages: { repositories: repositoryStorage.provision() },
    issuers: { github: {} },
    secrets: {
        "github-webhook": { parent: "platform", name: "github-webhook" },
        "github-private-key": { parent: "platform", name: "github-private-key" },
    },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        forge: install(
            forgePackage,
            {
                repositories: "repositories",
                credentials: "credentials",
                "github-webhook": "github-webhook",
                "github-private-key": "github-private-key",
            },
            { isolation: "host", name: FORGE_INSTALLATION },
        ),
    },
};

/** The options of a world whose residency home serves the vault, the connections and the forge. */
export const FORGE_HOME_OPTIONS = {
    ...TestPackage.options([VAULT, CONNECTION, FORGE]),
    tables: gitStorage.tables,
    // replicate the connections the forge follows as its installation, beside its names and callers
    follows: [{ shape: connectionShape, keys: [] }, claimFollow, standingFollow],
    resources: (directory: string, environment: Environment<"files" | "process" | "host">) => ({
        providers: [localGitProvider(directory, gitStorage, environment)],
        connectors: [LocalGitStorage.connector(directory, environment)],
    }),
};

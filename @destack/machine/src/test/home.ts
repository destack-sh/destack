import type { Environment } from "@destack/environment";
import { connectionShape } from "@destack/connection/server";
import connectionPackage from "@destack/connection/package";
import connectionServer from "@destack/connection/server";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import forgeServer, { claimFollow, standingFollow } from "@destack/forge/server";
import { credentialVault, repositoryStorage } from "@destack/forge/stack";
import { LocalGitStorage, localGitProvider } from "@destack/git/local";
import { testBuild } from "@destack/object/test";
import { gitStorage } from "@destack/stack/object";
import { testHost } from "@destack/test/host";
import vaultPackage from "@destack/vault/package";
import vaultServer from "@destack/vault/server";
import { install } from "@destack/stack/declare";
import { TestPackage } from "./package.ts";

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
    build: await testBuild(forgePackage, testHost().crypto),
    server: forgeServer,
    connectors: [],
    providers: [],
};

/** The packages whose types a home's forge stack declares. */
export const FORGE_HOME_TYPES = [vaultPackage, forgePackage, gitStorage] as const;

/** A home's forge stack: the forge with its vault and Git storage. */
export const FORGE_HOME_STACK = {
    vaults: { credentials: credentialVault.provision() },
    gitStorages: { repositories: repositoryStorage.provision() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        forge: install(
            forgePackage,
            { repositories: "repositories", credentials: "credentials" },
            { isolation: "host", name: FORGE_INSTALLATION },
        ),
    },
};

/** The options of a world whose platform home serves the vault, the connections and the forge. */
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

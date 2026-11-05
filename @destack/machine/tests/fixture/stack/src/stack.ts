import startPackage from "@destack/machine-start-fixture/package";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import forgePackage from "@destack/forge/package";
import { credentialVault, repositoryStorage } from "@destack/forge/stack";
import { gitStorage } from "@destack/stack/object";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import { defineVault } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";

/** The vault of a residency's home, whose secrets the forge binds. */
export const platformVault = defineVault({ name: "platform", spec: {} });

/** What every account's home runs, as the platform's: its connections and their vault. */
const ACCOUNT_HOME = {
    vaults: { credentials: connectionVault.declare() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "process", name: "vault" }),
        connection: install(
            connectionPackage,
            { credentials: "credentials" },
            { isolation: "process", name: CONNECTION_INSTALLATION },
        ),
    },
} as const;

/** The stack of every other account's home, as the platform's: its connections. */
export const ACCOUNT_STACK = defineStack([vaultPackage], ACCOUNT_HOME);

/** The stack of every person's home: an account's, with a start page, inbox and observability. */
export const PERSON_STACK = defineStack([vaultPackage], {
    ...ACCOUNT_HOME,
    installations: {
        ...ACCOUNT_HOME.installations,
        start: install(startPackage, {}, { name: "start" }),
        message: install(
            messagePackage,
            {},
            { isolation: "process", name: "message", capabilities: ["deliver"] },
        ),
        inbox: install(
            notificationPackage,
            {},
            { isolation: "process", name: "inbox", capabilities: ["deliver"] },
        ),
        observability: install(
            observabilityPackage,
            {},
            { isolation: "process", name: "observability" },
        ),
    },
});

/** The stack of every other space, as the platform's: observability. */
export const SPACE_STACK = defineStack([], {
    installations: {
        observability: install(
            observabilityPackage,
            {},
            { isolation: "process", name: "observability" },
        ),
    },
});

/** The stack of a residency's home that runs the forge. */
export const RESIDENCY_STACK = defineStack([vaultPackage, forgePackage, gitStorage], {
    vaults: { platform: platformVault.declare(), credentials: credentialVault.declare() },
    gitStorages: { repositories: repositoryStorage.declare() },
    issuers: { github: {} },
    secrets: {
        "github-webhook": { parent: "platform", name: "github-webhook" },
        "github-private-key": { parent: "platform", name: "github-private-key" },
    },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "process", name: "vault" }),
        forge: install(
            forgePackage,
            {
                repositories: "repositories",
                credentials: "credentials",
                "github-webhook": "github-webhook",
                "github-private-key": "github-private-key",
            },
            { isolation: "process", name: "forge" },
        ),
    },
});

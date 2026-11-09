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

/** The stack of every account's home: a start page, inbox, connections and observability. */
export const HOME_STACK = defineStack([vaultPackage], {
    vaults: { credentials: connectionVault.declare() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        connection: install(
            connectionPackage,
            { credentials: "credentials" },
            { isolation: "host", name: CONNECTION_INSTALLATION },
        ),
        start: install(startPackage, {}, { name: "start" }),
        message: install(
            messagePackage,
            {},
            { isolation: "host", name: "message", capabilities: ["deliver"] },
        ),
        inbox: install(
            notificationPackage,
            {},
            { isolation: "host", name: "inbox", capabilities: ["deliver"] },
        ),
        observability: install(
            observabilityPackage,
            {},
            { isolation: "host", name: "observability" },
        ),
    },
});

/** The stack of every other space, as the platform's: observability. */
export const SPACE_STACK = defineStack([], {
    installations: {
        observability: install(
            observabilityPackage,
            {},
            { isolation: "host", name: "observability" },
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
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        forge: install(
            forgePackage,
            {
                repositories: "repositories",
                credentials: "credentials",
                "github-webhook": "github-webhook",
                "github-private-key": "github-private-key",
            },
            { isolation: "host", name: "forge" },
        ),
    },
});

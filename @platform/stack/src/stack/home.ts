import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import forgePackage, { FORGE_INSTALLATION } from "@destack/forge/package";
import { credentialVault, repositoryStorage } from "@destack/forge/stack";
import { gitStorage } from "@destack/stack/object";
import { connectionVault } from "@destack/connection/stack";
import { HOME_INSTALLATION } from "@destack/account/object";
import { HOME_INPUT, PlatformInput } from "../input/index.ts";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import socialPackage from "@destack/social/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** Home installed as a space's front, following the release the host ships. */
export const HOME_DECLARATION = install(
    PlatformInput.handle(HOME_INPUT),
    {},
    { name: HOME_INSTALLATION, version: { kind: "host" } },
);

/** The declarations of every account's home: its start page, forge, inbox and connections. */
export const HOME_OPTIONS = {
    types: [vaultPackage, forgePackage, gitStorage],
    vaults: { connections: connectionVault.provision(), credentials: credentialVault.provision() },
    gitStorages: { repositories: repositoryStorage.provision() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        connection: install(
            connectionPackage,
            { credentials: "connections" },
            { isolation: "host", name: CONNECTION_INSTALLATION },
        ),
        forge: install(
            forgePackage,
            { repositories: "repositories", credentials: "credentials" },
            { isolation: "host", name: FORGE_INSTALLATION },
        ),
        home: HOME_DECLARATION,
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
        social: install(socialPackage, {}, { isolation: "host", name: "social" }),
        observability: install(
            observabilityPackage,
            {},
            { isolation: "host", name: "observability" },
        ),
    },
} as const;

/** Every account's home stack. */
export const HOME_STACK = defineStack(HOME_OPTIONS);

import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import { HOME_APPLICATION } from "@destack/account/object";
import homePackage from "@destack/home/package";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import socialPackage from "@destack/social/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** The stack of every account's home: Home, its inbox, connections, comments and observability. */
export const HOME_STACK = defineStack({
    types: [vaultPackage],
    vaults: { credentials: connectionVault.provision() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        connection: install(
            connectionPackage,
            { credentials: "credentials" },
            { isolation: "host", name: CONNECTION_INSTALLATION },
        ),
        home: install(homePackage, {}, { name: HOME_APPLICATION, selection: { kind: "host" } }),
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
});

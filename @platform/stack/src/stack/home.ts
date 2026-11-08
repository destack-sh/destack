import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import { HOME_APPLICATION } from "@destack/account/object";
import homePackage from "@destack/home/package";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** The stack of every account's home: Home, its inbox, connections and observability. */
export const HOME_STACK = defineStack([vaultPackage], {
    vaults: { credentials: connectionVault.declare() },
    installations: {
        vault: install(vaultPackage, {}, { isolation: "process", name: "vault" }),
        connection: install(
            connectionPackage,
            { credentials: "credentials" },
            { isolation: "process", name: CONNECTION_INSTALLATION },
        ),
        home: install(homePackage, {}, { name: HOME_APPLICATION }),
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

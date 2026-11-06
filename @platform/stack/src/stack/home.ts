import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { connectionVault } from "@destack/connection/stack";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import { PackageId } from "@destack/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";
import vaultPackage from "@destack/vault/package";

/** Home, the start app the platform organization publishes, which no stack declaration binds. */
export const HOME = {
    id: PackageId.parse("package-01a0c80b-614c-76c2-9b60-04b155f20ca9"),
    name: "@app/home",
    resources: {},
    secrets: {},
} as const;

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
        home: install(HOME, {}, { name: "home", selection: { kind: "tag", tag: "latest" } }),
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

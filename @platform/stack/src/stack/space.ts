import { install } from "@destack/stack/declare";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import socialPackage from "@destack/social/package";
import vaultPackage from "@destack/vault/package";
import { defineStack } from "@destack/stack";
import { HOME_INSTALLATION } from "./home.ts";

/** The stack of every other space: Home as its front, vault, and observability with its needs. */
export const SPACE_STACK = defineStack({
    types: [],
    installations: {
        home: HOME_INSTALLATION,
        vault: install(vaultPackage, {}, { isolation: "host", name: "vault" }),
        social: install(socialPackage, {}, { isolation: "host", name: "social" }),
        message: install(messagePackage, {}, { isolation: "host", name: "message" }),
        inbox: install(notificationPackage, {}, { isolation: "host", name: "inbox" }),
        observability: install(
            observabilityPackage,
            {},
            { isolation: "host", name: "observability" },
        ),
    },
});

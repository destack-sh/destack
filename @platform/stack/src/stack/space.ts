import { install } from "@destack/stack/declare";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import socialPackage from "@destack/social/package";
import vaultPackage from "@destack/vault/package";
import { defineStack } from "@destack/stack";

/** The stack of every other space: vault and observability, with what observability uses. */
export const SPACE_STACK = defineStack({
    types: [],
    installations: {
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

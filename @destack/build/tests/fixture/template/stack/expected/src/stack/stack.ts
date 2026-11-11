import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import observabilityPackage from "@destack/observability/package";
import socialPackage from "@destack/social/package";
import vaultPackage from "@destack/vault/package";
import { defineStack } from "@destack/stack";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** A space's stack: its baseline installations, files, credentials and a fatal issue rollback. */
export const personal = defineStack({
    types: [vaultPackage, observabilityPackage, notificationPackage, socialPackage, messagePackage],
    buckets: { files: files.provision({ retention: { within: { days: 30 } } }) },
    vaults: { credentials: credentials.provision({ retention: { within: { days: 30 } } }) },
    alertRules: {
        rollback: {
            title: "Roll back installation on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});

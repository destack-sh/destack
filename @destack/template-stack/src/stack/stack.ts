import observabilityPackage from "@destack/observability/package";
import vaultPackage from "@destack/vault/package";
import { defineStack } from "@destack/stack";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** A space's stack: the files bucket, the credentials vault, and a rollback on fatal issues. */
export const personal = defineStack([vaultPackage, observabilityPackage], {
    buckets: { files: files.declare({ retention: { within: { days: 30 } } }) },
    vaults: { credentials: credentials.declare({ retention: { within: { days: 30 } } }) },
    alertRules: {
        rollback: {
            title: "Roll back installation on new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});

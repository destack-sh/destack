import type {} from "@destack/observability/stack";
import { defineStack } from "@destack/stack";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** A space's stack: the files bucket, the credentials vault, and a rollback on fatal issues. */
export const personal = defineStack({
    resources: {
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    alertRules: {
        // roll an installation back to its previous revision when a new fatal issue opens
        rollback: {
            title: "Roll back new fatal issues",
            condition: { kind: "issue", on: "open", filter: "level = fatal" },
            actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
        },
    },
});

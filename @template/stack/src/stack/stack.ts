import { defineStack } from "@destack/stack";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** A space's stack: the files bucket and the credentials vault. */
export const personal = defineStack({
    resources: {
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
});

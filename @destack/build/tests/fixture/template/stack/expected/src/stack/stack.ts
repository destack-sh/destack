import { defineStack } from "@destack/stack";
import { database } from "../database/index.ts";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** A space's stack: the main database, the files bucket and the credentials vault. */
export const personal = defineStack({
    resources: {
        main: { declaration: database, retention: { within: { days: 30 } }, tags: {} },
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
});

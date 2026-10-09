import { defineSpace } from "@destack/space";
import { database } from "../database/index.ts";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** Shared resources administered independently in each selected space. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: { within: { days: 30 } }, tags: {} },
        files: { declaration: files, retention: { within: { days: 30 } }, tags: {} },
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
});

import { defineSpace } from "@destack/space";
import { database } from "../database/index.ts";
import { files } from "../bucket/index.ts";
import { credentials } from "../vault/index.ts";

/** Shared resources administered independently in each selected space. */
export const personal = defineSpace({
    resources: {
        main: { declaration: database, retention: "retain", tags: {} },
        files: { declaration: files, retention: "retain", tags: {} },
        credentials: { declaration: credentials, retention: "retain", tags: {} },
    },
});

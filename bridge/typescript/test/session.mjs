import assert from "node:assert/strict";

import { openSession } from "../dist/index.js";
import { openNapiSource } from "../dist/napi.js";

const source = {
    files: [
        { path: "destack.json", text: '{"name":"@test/app"}' },
        { path: "src/index.ds", text: "export const value = 1;" },
    ],
};

const session = await openSession({ root: "/workspace", source });
const napiSession = await openNapiSource("/workspace", source);

assert.equal(session.backend, "napi");
assert.equal(napiSession.backend, "napi");
assert.deepEqual(session.files(), ["destack.json", "src/index.ds"]);
assert.deepEqual(napiSession.files(), ["destack.json", "src/index.ds"]);

const update = session.update({
    edits: [
        {
            kind: "setText",
            setText: {
                path: "src/next.ds",
                text: "export const next = 2;",
            },
        },
    ],
});

assert.notDeepEqual(update.before, update.after);
assert.equal(update.files.length, 1);
assert.equal(update.files[0].path, "src/next.ds");

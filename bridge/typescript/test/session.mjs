import assert from "node:assert/strict";

import { openSession } from "../dist/index.js";
import { openNapiSource } from "../dist/napi.js";

const source = {
    files: [
        {
            path: "destack.json",
            content: { kind: "text", text: '{"name":"@test/app"}' },
        },
        {
            path: "src/index.ds",
            content: { kind: "text", text: "export const value = 1;" },
        },
    ],
};

const session = await openSession({ root: "/workspace", source });
const napiSession = await openNapiSource("/workspace", source);

assert.deepEqual(
    session.files().map((file) => file.path),
    ["destack.json", "src/index.ds"],
);
assert.deepEqual(
    napiSession.files().map((file) => file.path),
    ["destack.json", "src/index.ds"],
);

const module = session.loadModule("src/index.ds");
const version = session.require(session.revision(), {
    kind: "dirParsed",
    module: module.id,
});

assert.equal(version.key.kind, "dirParsed");
assert.deepEqual(version.key.module, module.id);
assert.match(version.fingerprint, /^f[0-9a-f]{32}$/);

const update = session.update({
    edits: [
        {
            kind: "setText",
            path: "src/next.ds",
            text: "export const next = 2;",
        },
    ],
});

assert.notDeepEqual(update.before, update.after);
assert.equal(update.files.length, 1);
assert.equal(update.files[0].path, "src/next.ds");

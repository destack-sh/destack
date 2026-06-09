import assert from "node:assert/strict";

import { FileEdit, Source, openSession } from "../dist/index.js";
import { openNapiSession } from "../dist/napi.js";

const source = Source.memory(
    "/workspace",
    [
        FileEdit.setText("destack.json", '{"name":"@test/app"}'),
        FileEdit.setText("src/index.ds", "export const value = 1;"),
    ],
);

const session = await openSession(source);
const napiSession = await openNapiSession(source);

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
const parsed = session.parse(session.revision(), module);

assert.equal(version.key.kind, "dirParsed");
assert.deepEqual(version.key.module, module.id);
assert.match(version.fingerprint, /^f[0-9a-f]{32}$/);
assert.equal(parsed.version.fingerprint, version.fingerprint);
assert.deepEqual(parsed.module, module.id);

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

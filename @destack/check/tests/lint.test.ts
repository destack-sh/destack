import { describe, it } from "vitest";
import { RuleTester } from "oxlint/plugins-dev";
import { rules } from "../src/lint/plugin.ts";

RuleTester.describe = describe;
RuleTester.it = it;

/** The tester running each rule's valid and invalid cases. */
const tester = new RuleTester();

/** The handle module path of a Destack package. */
const PACKAGE_HANDLE = new URL("../src/package.ts", import.meta.url).pathname;
/** The index module path of a Destack package. */
const PACKAGE_INDEX = new URL("../src/index.ts", import.meta.url).pathname;
/** A package source path for rules that depend on the module location. */
const SOURCE = new URL("../src/note/note.ts", import.meta.url).pathname;

tester.run("require-jsdoc", rules["require-jsdoc"], {
    valid: [
        "/** The note title. */\nexport const title = 1;",
        "/** A note. */\nclass Note {\n    /** The title. */\n    title = 1;\n}",
        'export * from "./note.ts";',
        "/** The identifier. */\n// oxlint-disable-next-line destack/prevent-abbreviations -- external identifier\nexport const id = 1;",
    ],
    invalid: [
        { code: "export const title = 1;", errors: [{ messageId: "missing" }] },
        {
            code: "/** A note. */\nclass Note {\n    title = 1;\n}",
            errors: [{ messageId: "missing" }],
        },
        { code: "/** The title. */\n\nconst title = 1;", errors: [{ messageId: "missing" }] },
    ],
});

tester.run("jsdoc-sentence", rules["jsdoc-sentence"], {
    valid: [
        "/** Send a message. */\nfunction send() {}",
        "/**\n * Send a message.\n *\n * Retry once on timeout.\n */\nfunction send() {}",
    ],
    invalid: [
        { code: "/** send a message. */\nfunction send() {}", errors: [{ messageId: "capital" }] },
        { code: "/** Send a message */\nfunction send() {}", errors: [{ messageId: "period" }] },
        {
            code: "/** Send a message. Retry once. */\nfunction send() {}",
            errors: [{ messageId: "sentence" }],
        },
        {
            code: "/**\n * Send a message.\n * Retry once.\n */\nfunction send() {}",
            errors: [{ messageId: "header" }],
        },
        { code: "/** Notes. */\n\nimport x from 'x';", errors: [{ messageId: "detached" }] },
    ],
});

tester.run("comment-style", rules["comment-style"], {
    valid: [
        "// read each note",
        "// SpaceService creates spaces",
        "// NOTE #Performance: scans every note",
        "// read each note\n//  and its revisions",
        "// read each note\n// TODO #Cleanup: batch the reads",
        '/// <reference types="bun" />',
    ],
    invalid: [
        {
            code: "// Read each note",
            output: "// read each note",
            errors: [{ messageId: "lowercase" }],
        },
        {
            code: "// read each note.",
            output: "// read each note",
            errors: [{ messageId: "period" }],
        },
        { code: "//read", errors: [{ messageId: "space" }] },
        { code: "// TODO fix this", errors: [{ messageId: "tag" }] },
        { code: "// read - then write", errors: [{ messageId: "dash" }] },
        {
            code: "// read each note\n// and its revisions",
            errors: [{ messageId: "continuation" }],
        },
    ],
});

tester.run("branch-comment-position", rules["branch-comment-position"], {
    valid: [
        "// small\nif (a) {\n    b();\n}\n// large\nelse {\n    c();\n}",
        "if (a) {\n    // only\n    b();\n}",
    ],
    invalid: [
        {
            code: "if (a) {\n    // small\n    b();\n} else {\n    c();\n}",
            errors: [{ messageId: "position" }],
        },
    ],
});

tester.run("padding-before-return", rules["padding-before-return"], {
    valid: [
        "function f() {\n    a();\n    b();\n\n    return 1;\n}",
        "function f() {\n    return 1;\n}",
        "function f() {\n    a();\n    b();\n\n    // stop listening\n    return () => a();\n}",
    ],
    invalid: [
        {
            code: "function f() {\n    a();\n    b();\n    return 1;\n}",
            output: "function f() {\n    a();\n    b();\n\n    return 1;\n}",
            errors: [{ messageId: "blank" }],
        },
        {
            code: "function f() {\n    a();\n    b();\n    // stop listening\n    return () => a();\n}",
            output: "function f() {\n    a();\n    b();\n\n    // stop listening\n    return () => a();\n}",
            errors: [{ messageId: "blank" }],
        },
    ],
});

tester.run("require-block-comment", rules["require-block-comment"], {
    valid: [
        "function f() {\n    // read\n    a();\n    b();\n\n    // write\n    c();\n    d();\n\n    return 1;\n}",
        "function f() {\n    a();\n    return 1;\n}",
    ],
    invalid: [
        {
            code: "function f() {\n    // read\n    a();\n    b();\n\n    c();\n    d();\n}",
            errors: [{ messageId: "missing" }],
        },
    ],
});

tester.run("prevent-abbreviations", rules["prevent-abbreviations"], {
    valid: ["const directory = 1;", "function f(message) {}"],
    invalid: [
        { code: "const dir = 1;", errors: [{ messageId: "word" }] },
        { code: "function f(msg) {}", errors: [{ messageId: "word" }] },
    ],
});

tester.run("boolean-prefix", rules["boolean-prefix"], {
    valid: ["const isOpen = a === b;", "const count = a + b;", "let hasNotes = false;"],
    invalid: [
        { code: "const open = a === b;", errors: [{ messageId: "prefix" }] },
        { code: "let changed = false;", errors: [{ messageId: "prefix" }] },
    ],
});

tester.run("no-import-alias", rules["no-import-alias"], {
    valid: [
        'import { note } from "./note.ts";',
        'import * as notes from "./note.ts";',
        'import { Symbol as TypeScriptSymbol } from "typescript";',
    ],
    invalid: [
        { code: 'import { note as entry } from "./note.ts";', errors: [{ messageId: "alias" }] },
    ],
});

tester.run("no-silent-fallback", rules["no-silent-fallback"], {
    valid: [
        "const notes = input ?? [];",
        "try {\n    a();\n} catch (error) {\n    throw new Error('read', { cause: error });\n}",
    ],
    invalid: [
        { code: "const count = input ?? 0;", errors: [{ messageId: "sentinel" }] },
        { code: "const title = input || '';", errors: [{ messageId: "sentinel" }] },
        { code: "const index = input ?? -1;", errors: [{ messageId: "sentinel" }] },
        { code: "try {\n    a();\n} catch {\n    b();\n}", errors: [{ messageId: "swallow" }] },
    ],
});

tester.run("no-partial-assertions", rules["no-partial-assertions"], {
    valid: [
        { code: "expect(title).toBe('note');", filename: "/package/tests/note.test.ts" },
        {
            code: "expect(id).toEqual(expect.stringMatching(/^note-[0-9a-f]+$/));",
            filename: "/package/tests/note.test.ts",
        },
    ],
    invalid: [
        {
            code: "expect(title).toContain('no');",
            filename: "/package/tests/note.test.ts",
            errors: [{ messageId: "partial" }],
        },
    ],
});

tester.run("valid-declaration", rules["valid-declaration"], {
    valid: [{ code: "export const main = defineDatabase({});", filename: SOURCE }],
    invalid: [
        {
            code: "const main = defineDatabase({});",
            filename: SOURCE,
            errors: [{ messageId: "export" }],
        },
        {
            code: "function f() {\n    return defineDatabase({});\n}",
            filename: SOURCE,
            errors: [{ messageId: "export" }],
        },
    ],
});

tester.run("valid-package-handle", rules["valid-package-handle"], {
    valid: [{ code: "export default definePackage({});", filename: PACKAGE_HANDLE }],
    invalid: [
        {
            code: "export const notes = definePackage({});",
            filename: PACKAGE_HANDLE,
            errors: [{ messageId: "missing" }, { messageId: "location" }],
        },
        {
            code: "export default definePackage({});",
            filename: SOURCE,
            errors: [{ messageId: "location" }],
        },
    ],
});

tester.run("no-manifest-import", rules["no-manifest-import"], {
    valid: [
        { code: 'import note from "./note.ts";', filename: SOURCE },
        {
            code: 'import manifest from "../package.json" with { type: "json" };',
            filename: "/tmp/site/src/main.ts",
        },
    ],
    invalid: [
        {
            code: 'import definition from "../../destack.json" with { type: "json" };',
            filename: SOURCE,
            errors: [{ messageId: "manifest" }],
        },
    ],
});

tester.run("no-index-logic", rules["no-index-logic"], {
    valid: [
        {
            code: 'export * from "./note.ts";\nexport { title } from "./title.ts";',
            filename: PACKAGE_INDEX,
        },
    ],
    invalid: [
        {
            code: "export const title = 1;",
            filename: PACKAGE_INDEX,
            errors: [{ messageId: "logic" }],
        },
    ],
});

import { describe, expect, it, test } from "vitest";
import { RuleTester } from "oxlint/plugins-dev";
import { rules } from "../src/lint/plugin.ts";
import { directiveLine } from "../src/lint/no-inline-config.ts";

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
        {
            code: "/** A note. */\nclass Note {\n    /** Write a value. */\n    write(value: string): void;\n    write(value: number): void;\n    write(value: unknown) {}\n}",
            filename: SOURCE,
        },
        {
            code: "/** A note. */\nclass Note {\n    /** Write a value. */\n    #write(value: string): void;\n    #write(value: number): void;\n    #write(value: unknown) {}\n}",
            filename: SOURCE,
        },
        {
            code: "export default defineConfiguration({ test: {} });",
            filename: new URL("../vitest.config.ts", import.meta.url).pathname,
        },
    ],
    invalid: [
        { code: "export const title = 1;", errors: [{ messageId: "missing" }] },
        {
            code: "export default defineConfiguration({ test: {} });",
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
        {
            code: "/** A note. */\nclass Note {\n    title = 1;\n}",
            errors: [{ messageId: "missing" }],
        },
        { code: "/** The title. */\n\nconst title = 1;", errors: [{ messageId: "missing" }] },
        {
            code: "/** A note. */\nclass Note {\n    /** Write text. */\n    #write(value: string) {}\n    write(value: string) {}\n}",
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
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

tester.run("error-message-style", rules["error-message-style"], {
    valid: [
        'throw new Error("note not found");',
        'throw new Error("HTTP request failed");',
        {
            code: 'throw new ServiceError("NOT_FOUND", { message: `no note ${id}` });',
            filename: SOURCE,
        },
        { code: "const failure = new ServiceError(code, options);", filename: SOURCE },
        {
            code: 'const failure = new ServiceError("FORBIDDEN", { ...options });',
            filename: SOURCE,
        },
        'throw new UpdateError("INSTALL", "invalid staged release digest");',
        "const failure = new NoteError(message);",
        'const notes = new Map([["Note", "Draft."]]);',
    ],
    invalid: [
        {
            code: 'throw new Error("Note not found.");',
            output: 'throw new Error("note not found");',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'reject(new TypeError("Missing note."));',
            output: 'reject(new TypeError("missing note"));',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw new UpdateError("INSTALL", `Unsafe archive path: ${path}.`);',
            output: 'throw new UpdateError("INSTALL", `unsafe archive path: ${path}`);',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw new UpdateError("INSTALL", "Invalid staged release digest.");',
            output: 'throw new UpdateError("INSTALL", "invalid staged release digest");',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw new ServiceError("NOT_FOUND");',
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
        {
            code: 'throw new ServiceError("FORBIDDEN", { data: { permission: "read" } });',
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
        {
            code: 'throw new ServiceError("FORBIDDEN", { message: "Permission denied." });',
            output: 'throw new ServiceError("FORBIDDEN", { message: "permission denied" });',
            filename: SOURCE,
            errors: [{ messageId: "style" }],
        },
    ],
});

tester.run("exact-optional", rules["exact-optional"], {
    valid: [
        "const title = schema.string().exactOptional();",
        "const title = field.string().max(80).optional();",
        "const title = parser.optional(value);",
    ],
    invalid: [
        { code: "const title = schema.string().optional();", errors: [{ messageId: "optional" }] },
        { code: "const release = Version.optional();", errors: [{ messageId: "optional" }] },
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

tester.run("no-overload-cast", rules["no-overload-cast"], {
    valid: [
        { code: "function read(id: string): Note { return notes.get(id); }", filename: SOURCE },
        {
            code: "function read(id: string): Note;\nfunction read(ids: string[]): Note[];\n/**\n * Read one note or several.\n *\n * @construct an identifier reads its note and a list reads each note in order\n */\nfunction read(input: string | string[]): unknown { return lookup(input); }",
            filename: SOURCE,
        },
        {
            code: "class Store {\n    read(id: string): Note { return this.notes.get(id); }\n}",
            filename: SOURCE,
        },
        {
            code: "function match<Value>(value: Typed<Value>, list: Value[]): SQL;\nfunction match(value: SQLWrapper, list: unknown[]): SQL;\nfunction match(value: SQLWrapper, list: unknown[]): SQL { return membership(value, list); }",
            filename: SOURCE,
        },
        {
            code: "function methods<Object>(object: Object): Methods<Object>;\n/**\n * Build each method's caller.\n *\n * @construct each key is a method of the object, whose caller the mapped type above describes\n */\nfunction methods(object: unknown): unknown { return build(object); }",
            filename: SOURCE,
        },
    ],
    invalid: [
        {
            code: "function read<Value>(): Value;\nfunction read(): unknown { return value; }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "function read<Value>(): Value;\n/** Read the value. @construct */\nfunction read(): unknown { return value; }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "export function read(id: string): Note;\nexport function read(id: string): unknown { return notes.get(id); }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "class Store {\n    read(): Note;\n    read(): unknown { return this.note; }\n}",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "function read(id: string): Note;\nfunction read(ids: string[]): Note[];\nfunction read(input: string | string[]): unknown { return lookup(input); }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "namespace Notes {\n    export function read<Value>(): Value;\n    export function read(): unknown { return value; }\n}",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "function outer() {\n    function read<Value>(): Value;\n    function read(): unknown { return value; }\n    return read;\n}",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
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
    valid: [
        {
            code: 'import { defineDatabase } from "@destack/db";\nexport const main = defineDatabase({});',
            filename: SOURCE,
        },
        { code: "const main = defineDatabase({});", filename: SOURCE },
        {
            code: 'import { valueOf } from "@destack/db";\nfunction f() {\n    return valueOf({});\n}',
            filename: SOURCE,
        },
    ],
    invalid: [
        {
            code: 'import { defineDatabase } from "@destack/db";\nconst main = defineDatabase({});',
            filename: SOURCE,
            errors: [{ messageId: "export" }],
        },
        {
            code: 'import * as db from "@destack/db";\nfunction f() {\n    return db.defineDatabase({});\n}',
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

/** A comment with its text and starting line. */
function at(line: number, value: string) {
    return { value, loc: { start: { line } } };
}

test("find the first comment that configures rules, ignoring ordinary comments", () => {
    // read directives in either tool's spelling, and nothing else
    expect([
        directiveLine([at(1, " a note about eslint"), at(2, " eslint-disable-next-line")]),
        directiveLine([at(3, " oxlint-disable no-console")]),
        directiveLine([at(4, "eslint curly: off")]),
        directiveLine([at(5, " plain comment")]),
    ]).toEqual([2, 3, 4, undefined]);
});

/** A component source path, which parses JSX. */
const COMPONENT = new URL("../src/note/note.tsx", import.meta.url).pathname;

tester.run("style-attribute", rules["style-attribute"], {
    valid: [
        {
            code: 'const styles = style.create({ wide: { width: "100%" } });\nconst element = <Button xstyle={styles.wide} style="color: red" />;',
            filename: COMPONENT,
        },
        { code: "const element = <div style={properties.style} />;", filename: COMPONENT },
    ],
    invalid: [
        {
            code: 'const styles = style.create({ wide: { width: "100%" } });\nconst element = <Button style={styles.wide} />;',
            filename: COMPONENT,
            errors: [{ messageId: "style" }],
        },
        {
            code: "const element = <Button style={[styles.wide, properties.xstyle]} />;",
            filename: COMPONENT,
            errors: [{ messageId: "style" }],
        },
    ],
});

tester.run("no-class-name", rules["no-class-name"], {
    valid: [{ code: 'const element = <div class="note" />;', filename: COMPONENT }],
    invalid: [
        {
            code: 'const element = <div className="note" />;',
            filename: COMPONENT,
            errors: [{ messageId: "className" }],
        },
    ],
});

tester.run("style-hover", rules["style-hover"], {
    valid: [
        'const styles = style.create({ link: { color: { default: "inherit", ":hover": { default: null, [media.hover]: "red" } } } });',
    ],
    invalid: [
        {
            code: 'const styles = style.create({ link: { color: { default: "inherit", ":hover": color.primary } } });',
            errors: [{ messageId: "hover" }],
        },
    ],
});

tester.run("style-tokens", rules["style-tokens"], {
    valid: [
        "const styles = style.create({ note: { color: color.primary, width: `calc(2 * ${space[4]})` } });",
    ],
    invalid: [
        {
            code: 'const styles = style.create({ note: { width: "var(--note-width)" } });',
            errors: [{ messageId: "variable" }],
        },
        {
            code: 'const styles = style.create({ note: { color: "#ff0000", backgroundColor: "oklch(0.7 0.1 30)" } });',
            errors: [{ messageId: "color" }, { messageId: "color" }],
        },
    ],
});

tester.run("style-shorthand", rules["style-shorthand"], {
    valid: [
        'const styles = style.create({ note: { margin: "auto", padding: "calc(1px + 2px)", translate: "0 4px" } });',
    ],
    invalid: [
        {
            code: 'const styles = style.create({ note: { margin: "0 auto" } });',
            errors: [{ messageId: "shorthand" }],
        },
    ],
});

tester.run("style-xstyle-last", rules["style-xstyle-last"], {
    valid: [
        "const attributes = style.attributes([styles.note, properties.xstyle], properties.style);",
    ],
    invalid: [
        {
            code: "const attributes = style.attributes([properties.xstyle, styles.note]);",
            errors: [{ messageId: "last" }],
        },
    ],
});

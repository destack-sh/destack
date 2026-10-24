import { describe, expect, it, test } from "vitest";
import { RuleTester } from "oxlint/plugins-dev";
import { rules } from "../src/lint/plugin.ts";
import { directiveLine } from "../src/lint/no-inline-config.ts";

RuleTester.describe = describe;
RuleTester.it = it;

/** The tester running each rule's valid and invalid cases. */
const tester = new RuleTester();

/** The lint settings listing this package as the workspace's one Destack package. */
const SETTINGS = {
    destack: { packages: ["@destack/check"], entrypoints: ["@destack/check/src/main.ts"] },
};

/** The module this package's bin runs. */
const ENTRYPOINT = new URL("../src/main.ts", import.meta.url).pathname;

/** The handle module path of a Destack package. */
const PACKAGE_HANDLE = new URL("../src/package.ts", import.meta.url).pathname;
/** The index module path of a Destack package. */
const PACKAGE_INDEX = new URL("../src/index.ts", import.meta.url).pathname;
/** A package source path for rules that depend on the module location. */
const SOURCE = new URL("../src/note/note.ts", import.meta.url).pathname;

/** A host module path of the environment package, which alone reaches the host's effects. */
const ENVIRONMENT_HOST = "/workspace/@destack/environment/src/file/bun.ts";

/** A test module path of a Destack package. */
const TEST = new URL("../src/note/note.test.ts", import.meta.url).pathname;

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
        `/** Send a message to every member${" ".repeat(55)}at once. */\nfunction send() {}`,
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
        {
            code: `/** Send a message to every member of the space ${"and each of its guests ".repeat(3)}at once. */\nfunction send() {}`,
            errors: [{ messageId: "width" }],
        },
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
            code: 'throw new CallError("NOT_FOUND", { message: `no note ${id}` });',
            filename: SOURCE,
        },
        { code: "const failure = new CallError(code, options);", filename: SOURCE },
        {
            code: 'const failure = new CallError("FORBIDDEN", { ...options });',
            filename: SOURCE,
        },
        'import { UpdateError } from "./error.ts"; throw UpdateError.of("INSTALL", "invalid staged release digest");',
        "const failure = new NoteError(message);",
        'const Note = new Map(); Note.of("DRAFT", "Draft.");',
        'throw CallError.http("NOT_FOUND", { message: "note not found" });',
        'throw new FamilyError(failure, "note not found");',
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
            code: 'import { UpdateError } from "./error.ts"; throw UpdateError.of("INSTALL", `Unsafe archive path: ${path}.`);',
            output: 'import { UpdateError } from "./error.ts"; throw UpdateError.of("INSTALL", `unsafe archive path: ${path}`);',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'const Update = defineErrorFamily(owner, {}); throw Update.of("INSTALL", "Invalid staged release digest.");',
            output: 'const Update = defineErrorFamily(owner, {}); throw Update.of("INSTALL", "invalid staged release digest");',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw CallError.http("NOT_FOUND", { message: "Note not found." });',
            output: 'throw CallError.http("NOT_FOUND", { message: "note not found" });',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw CallError.http("NOT_FOUND");',
            errors: [{ messageId: "missing" }],
        },
        {
            code: 'throw new FamilyError(failure, "Note not found.");',
            output: 'throw new FamilyError(failure, "note not found");',
            errors: [{ messageId: "style" }],
        },
        {
            code: 'throw new CallError("NOT_FOUND");',
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
        {
            code: 'throw new CallError("FORBIDDEN", { data: { permission: "read" } });',
            filename: SOURCE,
            errors: [{ messageId: "missing" }],
        },
        {
            code: 'throw new CallError("FORBIDDEN", { message: "Permission denied." });',
            output: 'throw new CallError("FORBIDDEN", { message: "permission denied" });',
            filename: SOURCE,
            errors: [{ messageId: "style" }],
        },
    ],
});

tester.run("deterministic-handler", rules["deterministic-handler"], {
    valid: [
        "task.implement({ methods: { archive: (call) => call.update({ archivedAt: call.now }) } });",
        "task.implement({ methods: { export: { prepare: async () => fetch(url), handle: (call) => call.prepared } } });",
        "const startedAt = Date.now();",
        "task.implement({ methods: { archive: (call) => call.update({ at: new Date(call.now) }) } });",
        'digest.implement(async (step) => step.run("roll", schema.number(), async () => Math.random()));',
    ],
    invalid: [
        {
            code: "task.implement({ methods: { archive: (call) => call.update({ archivedAt: Date.now() }) } });",
            errors: [{ messageId: "call", data: { name: "Date.now()", replacement: "call.now" } }],
        },
        {
            code: "task.implement({ methods: { draw: { handle: async () => Math.random() } } });",
            errors: [
                {
                    messageId: "call",
                    data: { name: "Math.random()", replacement: "call.random()" },
                },
            ],
        },
        {
            code: "task.implement({ methods: { check: { authorize: async () => { await fetch(url); } } } });",
            errors: [
                { messageId: "call", data: { name: "fetch()", replacement: "a prepare phase" } },
            ],
        },
        {
            code: "task.implement({ methods: { create: (call, next) => next(call.with({ id: crypto.randomUUID(), at: new Date() })) } });",
            errors: [
                {
                    messageId: "call",
                    data: { name: "crypto.randomUUID()", replacement: "call.generateId(type)" },
                },
                { messageId: "call", data: { name: "new Date()", replacement: "call.now" } },
            ],
        },
        {
            code: "digest.implement(async (step) => { const at = Date.now(); await fetch(url); return new Date(); });",
            errors: [
                {
                    messageId: "step",
                    data: { name: "Date.now()", replacement: "step.run(name, schema, work)" },
                },
                {
                    messageId: "step",
                    data: { name: "fetch()", replacement: "step.run(name, schema, work)" },
                },
                {
                    messageId: "step",
                    data: { name: "new Date()", replacement: "step.run(name, schema, work)" },
                },
            ],
        },
    ],
});

tester.run("no-ambient-effect", rules["no-ambient-effect"], {
    valid: [
        { code: 'import { validate } from "uuid";', filename: SOURCE },
        { code: "const answer = await environment.network.fetch(request);", filename: SOURCE },
        {
            code: "function read(fetch: typeof globalThis.fetch, process: { env: object }) {\n    return [fetch(url), process.env];\n}",
            filename: SOURCE,
        },
        { code: "const now = environment.clock.now();", filename: SOURCE },
        {
            code: 'const imports = new Bun.Transpiler({ loader: "ts" }).scanImports(text);',
            filename: SOURCE,
        },
        { code: 'import { join } from "node:path";', filename: SOURCE },
        { code: "const at = new Date(environment.clock.now());", filename: SOURCE },
        { code: "const value = options.process.env;", filename: SOURCE },
        { code: 'const text = at.toLocaleString("en-US");', filename: SOURCE },
        { code: 'const gzip = environment.compression.compress("gzip");', filename: SOURCE },
        {
            code: 'const { process } = environment;\nconst child = process.spawn(["git"]);',
            filename: SOURCE,
        },
        {
            code: 'import { readFile } from "node:fs/promises";\nconst now = Date.now();',
            filename: ENVIRONMENT_HOST,
        },
    ],
    invalid: [
        {
            code: "const active = context.active();\nconst span = trace.getActiveSpan();",
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: {
                        name: "context.active",
                        replacement: "environment.telemetry.context.active",
                    },
                },
                {
                    messageId: "effect",
                    data: {
                        name: "trace.getActiveSpan",
                        replacement: "trace.getSpan(environment.telemetry.context.active())",
                    },
                },
            ],
        },
        {
            code: "const answer = await fetch(url);\nconst socket = new WebSocket(url);",
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "fetch", replacement: "environment.network.fetch" },
                },
                {
                    messageId: "effect",
                    data: { name: "new WebSocket", replacement: "environment.network.websocket" },
                },
            ],
        },
        {
            code: 'import { v4, v7, validate } from "uuid";',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "uuid's v4", replacement: "Uuid.v4(environment.random)" },
                },
                {
                    messageId: "effect",
                    data: { name: "uuid's v7", replacement: "Uuid.v7(environment)" },
                },
            ],
        },
        {
            code: 'const gzip = new CompressionStream("gzip");\nconst text = at.toLocaleString();\nconst tracer = trace.getTracer("notes");\nconst format = new Intl.DateTimeFormat();',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: {
                        name: "new CompressionStream",
                        replacement: "environment.compression.compress",
                    },
                },
                {
                    messageId: "effect",
                    data: { name: "toLocaleString()", replacement: "environment.locale" },
                },
                {
                    messageId: "effect",
                    data: { name: "trace.getTracer", replacement: "environment.telemetry.tracers" },
                },
                {
                    messageId: "effect",
                    data: { name: "Intl without a locale", replacement: "environment.locale" },
                },
            ],
        },
        {
            code: 'import { readFile } from "node:fs/promises";',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "node:fs/promises", replacement: "environment.files" },
                },
            ],
        },
        {
            code: "const home = process.env.HOME;\nconst at = new Date();",
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "process.env", replacement: "environment.process.variables" },
                },
                {
                    messageId: "effect",
                    data: { name: "new Date()", replacement: "new Date(environment.clock.now())" },
                },
            ],
        },
        {
            code: 'const git = Bun.which("git");\nconst version = process.version;\nconst host = globalThis["process"];\nconst isNode = "process" in globalThis;',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "Bun.which", replacement: "the environment" },
                },
                {
                    messageId: "effect",
                    data: { name: "process.version", replacement: "environment.process" },
                },
                {
                    messageId: "effect",
                    data: { name: "globalThis.process", replacement: "environment.process" },
                },
                {
                    messageId: "effect",
                    data: { name: "globalThis.process", replacement: "environment.process" },
                },
            ],
        },
        {
            code: 'import { createHash } from "node:crypto";',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: {
                        name: "node:crypto",
                        replacement: "environment.crypto, or Digest.of for hashes",
                    },
                },
            ],
        },
        {
            code: 'import { connect } from "node:net";\nconst { subtle } = crypto;\nconst { env } = process;\nglobalThis.console = logger;',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "node:net", replacement: "environment.network.connect" },
                },
                {
                    messageId: "effect",
                    data: { name: "crypto.subtle", replacement: "environment.crypto" },
                },
                {
                    messageId: "effect",
                    data: { name: "process.env", replacement: "environment.process.variables" },
                },
                {
                    messageId: "effect",
                    data: { name: "globalThis.console", replacement: "environment.process.stderr" },
                },
            ],
        },
        {
            code: 'const online = navigator.onLine;\nawait navigator.share(data);\nnavigator.vibrate(200);\nconst camera = await navigator.permissions.query({ name: "camera" });',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: {
                        name: "navigator.onLine",
                        replacement: "environment.connectivity.isOnline",
                    },
                },
                {
                    messageId: "effect",
                    data: { name: "navigator.share", replacement: "environment.share.share" },
                },
                {
                    messageId: "effect",
                    data: {
                        name: "navigator.vibrate",
                        replacement: "environment.vibration.vibrate",
                    },
                },
                {
                    messageId: "effect",
                    data: { name: "navigator.permissions", replacement: "environment.permissions" },
                },
            ],
        },
        {
            code: 'const theme = localStorage.getItem("theme");\nconst path = window.location.pathname;\nawait navigator.clipboard.writeText(text);\nrequestAnimationFrame(draw);\nconst events = new EventSource(url);',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "localStorage.getItem", replacement: "environment.storage" },
                },
                {
                    messageId: "effect",
                    data: {
                        name: "window.location",
                        replacement: "environment.navigation.currentEntry",
                    },
                },
                {
                    messageId: "effect",
                    data: { name: "navigator.clipboard", replacement: "environment.clipboard" },
                },
                {
                    messageId: "effect",
                    data: {
                        name: "requestAnimationFrame",
                        replacement: "environment.display.frame",
                    },
                },
                {
                    messageId: "effect",
                    data: {
                        name: "new EventSource",
                        replacement: "a streamed environment.network.fetch",
                    },
                },
            ],
        },
        {
            code: 'setTimeout(retry, 10);\nconst bytes = await crypto.subtle.digest("SHA-256", data);',
            filename: SOURCE,
            errors: [
                {
                    messageId: "effect",
                    data: { name: "setTimeout", replacement: "environment.clock.sleep" },
                },
                {
                    messageId: "effect",
                    data: { name: "crypto.subtle", replacement: "environment.crypto" },
                },
            ],
        },
    ],
});

tester.run("no-system-environment", rules["no-system-environment"], {
    valid: [
        {
            code: 'import { BunEnvironment } from "@destack/environment/bun";\nconst environment = BunEnvironment.system;',
            filename: ENTRYPOINT,
            settings: SETTINGS,
        },
        {
            code: 'import { Crypto } from "./crypto.ts";\nconst crypto = Crypto.system;',
            filename: ENVIRONMENT_HOST,
            settings: SETTINGS,
        },
        {
            code: 'import { BunEnvironment } from "@destack/environment/bun";\nexport default defineConfiguration({}, BunEnvironment.system);',
            filename: "/workspace/@destack/check/vitest.config.ts",
            settings: SETTINGS,
        },
        { code: "const crypto = environment.crypto;", filename: SOURCE, settings: SETTINGS },
        {
            code: 'import { Theme } from "./theme.ts";\nconst theme = Theme.system;',
            filename: SOURCE,
            settings: SETTINGS,
        },
        {
            code: 'import type { Clock } from "@destack/environment";\nfunction read(Clock: { system: number }) {\n    return Clock.system;\n}',
            filename: SOURCE,
            settings: SETTINGS,
        },
    ],
    invalid: [
        {
            code: 'import { Crypto, Environment } from "@destack/environment";\nimport { BrowserEnvironment } from "@destack/environment/browser";\nconst crypto = Crypto.system;\nconst environment = Environment.system;\nconst browser = BrowserEnvironment.system;',
            filename: SOURCE,
            settings: SETTINGS,
            errors: [
                { messageId: "system", data: { name: "Crypto.system" } },
                { messageId: "system", data: { name: "Environment.system" } },
                { messageId: "system", data: { name: "BrowserEnvironment.system" } },
            ],
        },
        {
            code: 'import * as environment from "@destack/environment";\nimport { Random } from "@destack/environment";\nconst clock = environment.Clock.system;\nconst { system } = Random;\nconst random = Random["system"];',
            filename: SOURCE,
            settings: SETTINGS,
            errors: [
                { messageId: "system", data: { name: "environment.Clock.system" } },
                { messageId: "system", data: { name: "Random.system" } },
                { messageId: "system", data: { name: "Random.system" } },
            ],
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

tester.run("method-signature-style", rules["method-signature-style"], {
    valid: [
        {
            code: "interface Handler { readonly handle: (call: Call) => Promise<void>; }",
            filename: SOURCE,
        },
        { code: "type Reader = { readonly read: (key: string) => string };", filename: SOURCE },
        { code: "interface Named { get name(): string; }", filename: SOURCE },
        { code: "interface Codec<Value> { encode(value: Value): string; }", filename: SOURCE },
        {
            code: "type Codec<Value> = { encode(value: readonly Value[]): string };",
            filename: SOURCE,
        },
    ],
    invalid: [
        {
            code: "interface Handler { handle(call: Call): Promise<void>; }",
            filename: SOURCE,
            errors: [{ messageId: "method", data: { name: "handle" } }],
        },
        {
            code: "type Reader = { read(key: string): string };",
            filename: SOURCE,
            errors: [{ messageId: "method", data: { name: "read" } }],
        },
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
            code: "class Store {\n    read(id: string): Note { return this.notes.get(id); }\n}",
            filename: SOURCE,
        },
        {
            code: "function match<Value>(value: Typed<Value>, list: Value[]): SQL;\nfunction match(value: SQLWrapper, list: unknown[]): SQL;\nfunction match(value: SQLWrapper, list: unknown[]): SQL { return membership(value, list); }",
            filename: SOURCE,
        },
    ],
    invalid: [
        {
            code: "function read(id: string): Note;\nfunction read(ids: string[]): Note[];\n/**\n * Read one note or several.\n *\n * @construct an identifier reads its note and a list reads each note in order\n */\nfunction read(input: string | string[]): unknown { return lookup(input); }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
        {
            code: "function methods<Object>(object: Object): Methods<Object>;\n/**\n * Build each method's caller.\n *\n * @construct each key is a method of the object, whose caller the mapped type above describes\n */\nfunction methods(object: unknown): unknown { return build(object); }",
            filename: SOURCE,
            errors: [{ messageId: "cast" }],
        },
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

tester.run("valid-package-handle", rules["valid-package-handle"], {
    valid: [
        { code: "export default definePackage({});", filename: PACKAGE_HANDLE, settings: SETTINGS },
        { code: "const notes = definePackage({});", filename: TEST, settings: SETTINGS },
    ],
    invalid: [
        {
            code: "export const notes = definePackage({});",
            filename: PACKAGE_HANDLE,
            settings: SETTINGS,
            errors: [{ messageId: "missing" }, { messageId: "location" }],
        },
        {
            code: "export default definePackage({});",
            filename: SOURCE,
            settings: SETTINGS,
            errors: [{ messageId: "location" }],
        },
    ],
});

tester.run("no-manifest-import", rules["no-manifest-import"], {
    valid: [
        { code: 'import note from "./note.ts";', filename: SOURCE, settings: SETTINGS },
        {
            code: 'import manifest from "../package.json" with { type: "json" };',
            filename: "/tmp/site/src/main.ts",
            settings: SETTINGS,
        },
    ],
    invalid: [
        {
            code: 'import definition from "../../destack.json" with { type: "json" };',
            filename: SOURCE,
            settings: SETTINGS,
            errors: [{ messageId: "manifest" }],
        },
    ],
});

tester.run("no-index-logic", rules["no-index-logic"], {
    valid: [
        {
            code: 'export * from "./note.ts";\nexport { title } from "./title.ts";',
            filename: PACKAGE_INDEX,
            settings: SETTINGS,
        },
    ],
    invalid: [
        {
            code: "export const title = 1;",
            filename: PACKAGE_INDEX,
            settings: SETTINGS,
            errors: [{ messageId: "logic" }],
        },
    ],
});

/** A comment with its text and starting line. */
function at(line: number, value: string) {
    return { value, loc: { start: { line } } };
}

test("find the first comment that configures rules, ignoring ordinary comments and reasoned next-line suppressions", () => {
    // read directives in either tool's form, allowing a named next-line suppression with its reason
    expect([
        directiveLine([at(1, " a note about eslint"), at(2, " eslint-disable-next-line")]),
        directiveLine([at(3, " oxlint-disable no-console")]),
        directiveLine([at(4, "eslint curly: off")]),
        directiveLine([at(5, " plain comment")]),
        directiveLine([at(6, " oxlint-disable-next-line typescript/no-explicit-any -- trusted")]),
        directiveLine([at(7, " oxlint-disable-next-line typescript/no-explicit-any")]),
        directiveLine([at(8, " eslint-disable-next-line curly -- reason")]),
    ]).toEqual([2, 3, 4, undefined, undefined, 7, 8]);
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

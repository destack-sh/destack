import { testSimulation } from "@destack/test/world";
import type { FileSystem } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { Message } from "@destack/locale";
import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { testHost } from "@destack/test/host";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import {
    agentsFile,
    checkConfiguration,
    configurePackage,
    settingsRoot,
    typescriptConfiguration,
    lintSettings,
} from "../src/check/configuration.ts";
import { readExpectations, applyExpectations } from "../src/check/expectation.ts";
import { applyTranslations } from "../src/check/translation.ts";
import { OUTPUT_DIRECTORIES } from "../src/check/sources.ts";
import { Toolchain } from "../src/toolchain/index.ts";

/** The language libraries every package compiles against. */
const LANGUAGE = [
    "ES2024",
    "ESNext.Array",
    "ESNext.TypedArrays",
    "ES2025.Collection",
    "ES2025.Promise",
    "ES2025.RegExp",
    "ESNext.Disposable",
];

/** Open a world for one test, whose files it removes once disposed. */
function world(): Simulation {
    return testSimulation();
}

/** Write a package with its manifests and sources into a temporary directory. */
async function writePackage(
    files: FileSystem,
    runtimes: string[],
    manifest: Record<string, unknown>,
    sources: Record<string, string>,
): Promise<string> {
    const directory = await files.makeTemporaryDirectory("configuration-");
    const definition = {
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01a0c80b-614d-75ac-841e-b27280bf3db3",
        role: "library",
        language: "typescript",
        runtimes,
    };
    await files.writeTextFile(join(directory, "destack.json"), JSON.stringify(definition));
    await files.writeTextFile(join(directory, "package.json"), JSON.stringify(manifest));
    for (const [path, source] of Object.entries(sources)) {
        await files.mkdir(join(directory, path, ".."), { recursive: true });
        await files.writeTextFile(join(directory, path), source);
    }

    return directory;
}

/** Build a finding of a rule in a file. */
function finding(filename: string, code: string) {
    return { code, message: "finding", severity: "error" as const, filename, labels: [] };
}

test("compile a browser package with views against DOM, the view JSX runtime and its asset types", async () => {
    await using simulation = world();
    const { files } = simulation.environment("test");
    const directory = await writePackage(
        files,
        ["browser"],
        { name: "@example/app", dependencies: { "@destack/view": "2026.9.0" } },
        { "src/app.tsx": "export {};\n" },
    );
    expect(await typescriptConfiguration(directory, files)).toEqual({
        compilerOptions: {
            ...TYPESCRIPT_OPTIONS,
            incremental: true,
            tsBuildInfoFile: "node_modules/.cache/destack/tsc.tsbuildinfo",
            lib: [...LANGUAGE, "DOM", "DOM.Iterable"],
            jsx: "preserve",
            jsxImportSource: "@destack/view",
            types: ["@destack/view/client"],
        },
        include: ["**/*"],
        exclude: [...OUTPUT_DIRECTORIES],
    });
});

test("compile a Bun-only package against Bun's types without DOM", async () => {
    await using simulation = world();
    const { files } = simulation.environment("test");
    const directory = await writePackage(
        files,
        ["bun"],
        { name: "@example/server", devDependencies: { "@types/bun": "1.4.2" } },
        { "src/index.ts": "export {};\n" },
    );
    const { compilerOptions } = await typescriptConfiguration(directory, files);
    expect(compilerOptions).toEqual({
        ...TYPESCRIPT_OPTIONS,
        incremental: true,
        tsBuildInfoFile: "node_modules/.cache/destack/tsc.tsbuildinfo",
        lib: [...LANGUAGE],
        types: ["bun"],
    });
});

test("compile a Bun package with Solid's terminal renderer against DOM and the renderer's JSX runtime", async () => {
    await using simulation = world();
    const { files } = simulation.environment("test");
    const directory = await writePackage(
        files,
        ["bun"],
        {
            name: "@example/terminal",
            dependencies: { "@opentui/solid": "0.1.0" },
            devDependencies: { "@types/bun": "1.4.2" },
        },
        { "src/view.tsx": "export {};\n" },
    );
    const { compilerOptions } = await typescriptConfiguration(directory, files);
    expect(compilerOptions).toEqual({
        ...TYPESCRIPT_OPTIONS,
        incremental: true,
        tsBuildInfoFile: "node_modules/.cache/destack/tsc.tsbuildinfo",
        lib: [...LANGUAGE, "DOM", "DOM.Iterable"],
        types: ["bun"],
        jsx: "preserve",
        jsxImportSource: "@opentui/solid",
    });
});

test("refuse a hand-edited tsconfig.json", async () => {
    await using simulation = world();
    const { files } = simulation.environment("test");
    const directory = await writePackage(files, ["bun"], { name: "@example/server" }, {});
    // accept the written configuration and refuse an edited option
    await configurePackage(directory, Toolchain.workspace(), files);
    await checkConfiguration(directory, files);
    const path = join(directory, "tsconfig.json");
    const source = await files.readTextFile(path);
    await files.writeTextFile(path, source.replace('"strict": true', '"strict": false'));
    expect(await refusal(checkConfiguration(directory, files))).toEqual([
        "BAD_REQUEST",
        "CONFIGURATION_INVALID",
        "'tsconfig.json' is invalid: it differs from the shared configuration; run configure",
    ]);
});

test("write lint and format settings at the workspace root and compiler settings in each member", async () => {
    // place a member package inside a workspace root in a separate temporary directory
    await using simulation = world();
    const { files } = simulation.environment("test");
    const workspace = await files.makeTemporaryDirectory("workspace-");
    const member = join(workspace, "member");
    await files.mkdir(member);
    await files.writeTextFile(
        join(workspace, "package.json"),
        JSON.stringify({ workspaces: ["member", "assets"] }),
    );
    await files.writeTextFile(
        join(member, "package.json"),
        JSON.stringify({
            name: "@example/member",
            bin: { member: "./src/main.ts" },
            main: "./src/index.ts",
            scripts: {
                refresh: "bun run scripts/refresh.ts --all",
                lint: "destack-check check src content.ts",
                test: "vitest run",
            },
            exports: { "./preload": "./src/preload.ts" },
            dependencies: { "@example/notes": "2026.9.0", zod: "4.0.0" },
        }),
    );
    await files.writeTextFile(
        join(member, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01a0c80b-614d-75ac-841e-b27280bf3db3",
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
            entrypoints: ["src/worker.ts"],
        }),
    );

    // configure both, the member taking only its compiler settings
    await configurePackage(member, Toolchain.workspace(), files);
    expect([
        await files.exists(join(member, "tsconfig.json")),
        await files.exists(join(member, ".oxlintrc.json")),
        await files.exists(join(member, ".oxfmtrc.json")),
    ]).toEqual([true, false, false]);

    // read the root's expectations beside the member's, each against its directory
    const expectation = {
        files: ["scripts/release.ts"],
        rules: ["eslint/no-console"],
        reason: "the release script reports to its terminal",
    };
    await files.writeTextFile(
        join(workspace, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            workspace: { check: { expect: [expectation] } },
        }),
    );
    expect(await readExpectations(workspace, files)).toEqual([
        { directory: workspace, expectation },
    ]);

    // list the modules its bin, declared entrypoints, Bun scripts and preloads run, leaving main and arguments out
    await files.writeTextFile(
        join(member, "bunfig.toml"),
        'preload = ["@example/member/preload", "@example/outside/preload"]\n',
    );
    // add a package of plain files whose script is a program too
    const assets = join(workspace, "assets");
    await files.mkdir(assets);
    await files.writeTextFile(
        join(assets, "package.json"),
        JSON.stringify({ name: "@example/assets", scripts: { generate: "bun run generate.ts" } }),
    );
    // install an app and an npm package it depends on, reading the app's role
    const notes = join(workspace, "node_modules", "@example", "notes");
    await files.mkdir(notes, { recursive: true });
    await files.writeTextFile(
        join(notes, "package.json"),
        JSON.stringify({ name: "@example/notes" }),
    );
    await files.writeTextFile(
        join(notes, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01a0c80b-614d-75ac-841e-b27280bf3db4",
            role: "app",
            language: "typescript",
        }),
    );
    expect(await lintSettings(workspace, files)).toEqual({
        packages: [{ directory: "member", name: "@example/member" }],
        roles: { "@example/member": "library", "@example/notes": "app" },
        entrypoints: [
            "member/src/main.ts",
            "member/src/worker.ts",
            "member/scripts/refresh.ts",
            "member/src/preload.ts",
            "assets/generate.ts",
        ],
    });

    // find the workspace's settings from the root, the member and a directory inside it
    expect(
        await Promise.all(
            [workspace, member, join(member, "src")].map((path) => settingsRoot(path, files)),
        ),
    ).toEqual([workspace, workspace, workspace]);
});

test("accept the findings a package expects and refuse an expectation nothing matches", async () => {
    // expect console use in one file and an unused rule in another
    await using simulation = world();
    const directory = await simulation
        .environment("test")
        .files.makeTemporaryDirectory("expectation-");
    const expectations = [
        {
            directory,
            expectation: {
                files: ["src/log.ts"],
                rules: ["eslint/no-console"],
                reason: "logs to the console",
            },
        },
        {
            directory,
            expectation: {
                files: ["src/quiet.ts"],
                rules: ["eslint/no-console"],
                reason: "no longer logs",
            },
        },
    ];

    // report the unexpected finding and the stale expectation, dropping the expected one
    const applied = applyExpectations(
        [finding("src/log.ts", "eslint(no-console)"), finding("src/log.ts", "eslint(curly)")],
        expectations,
        directory,
        ["src"],
    );
    expect(applied.map((diagnostic) => [diagnostic.code, diagnostic.filename])).toEqual([
        ["eslint(curly)", "src/log.ts"],
        ["destack(unfulfilled-expectation)", "src/quiet.ts"],
    ]);

    // leave expectations over unchecked files alone
    expect(applyExpectations([], expectations, directory, ["tests"])).toEqual([]);
});

test("refuse a message a language lacks or varies, and a translation of no message once all is checked", async () => {
    // write two packages, one with catalogs of a language, its region and another language
    await using simulation = world();
    const { files } = simulation.environment("test");
    const root = await files.makeTemporaryDirectory("translation-");
    const write = async (tag: string, messages: Readonly<Record<string, string>>) => {
        await files.mkdir(join(root, "notes/locale"), { recursive: true });
        const catalog = {
            package: "package-01996ab0-0000-7000-8000-0000000000e1",
            locale: tag,
            messages,
        };
        await files.writeTextFile(join(root, `notes/locale/${tag}.json`), JSON.stringify(catalog));
    };
    const saved = Message.identify("Saved", "");
    const menu = Message.identify("Saved", "menu");
    const lost = Message.identify("one {{Lost}}\n* {{Lost}}", "");
    const count = Message.identify("{$p0} notes", "");
    await write("de", { [saved]: "Gespeichert", gone: "Weg", [count]: "{$p0} Notizen" });
    await write("de-CH", { [saved]: "Gspeicheret" });
    await write("fr", { [count]: "{$p1} notes" });
    const settings = {
        packages: [
            { directory: "notes", name: "@example/notes" },
            { directory: "tasks", name: "@example/tasks" },
        ],
        roles: {},
        entrypoints: [],
    };
    const found = (filename: string, source: string, context = "") => ({
        ...finding(filename, "destack(no-missing-translations)"),
        message: `${context}\u0000${source}`,
    });
    const diagnostics = [
        found("notes/src/note.ts", "Saved"),
        found("notes/src/note.ts", "Saved", "menu"),
        found("notes/src/note.ts", "one {{Lost}}\n* {{Lost}}"),
        found("notes/src/note.ts", "{$p0} notes"),
        found("tasks/src/task.ts", "Done"),
        finding("notes/src/note.ts", "eslint(curly)"),
    ];
    const apply = async (selection: readonly string[]) =>
        (
            await applyTranslations(diagnostics, {
                directory: root,
                root,
                settings,
                selection,
                files,
            })
        ).map(({ code, filename, message }) => `${filename} ${code} ${message}`);
    const missing = [
        "notes/src/note.ts eslint(curly) finding",
        `notes/src/note.ts destack(no-missing-translations) translate 'Saved' ('${saved}') into fr`,
        `notes/src/note.ts destack(no-missing-translations) translate 'Saved' in 'menu' ('${menu}') into de, fr`,
        `notes/src/note.ts destack(no-missing-translations) translate 'one {{Lost}} * {{Lost}}' ('${lost}') into de, fr`,
        `notes/src/note.ts destack(no-missing-translations) translate '{$p0} notes' ('${count}') with '$p0' in fr`,
    ];

    expect({
        file: await apply(["notes/src/note.ts"]),
        all: await apply(["notes/src", "tasks"]),
    }).toEqual({
        file: missing,
        all: [
            ...missing,
            "notes/locale/de.json destack(no-unused-translations) 'gone' ('Weg') translates no message: remove it",
        ],
    });
});

/** A standard's AGENTS.md, shortened to one rule. */
const STANDARD = "# Agents\n\n- **PS01** Agents MUST follow these rules proactively.\n";

test("carry a workspace's custom section over exactly when writing its AGENTS.md", () => {
    // regenerate a file whose standard part is current and whose custom section has rules
    const existing = `${STANDARD}\n## Custom\n\n- **X01** Name the forks after rivers.\n\nSee the wiki.\n`;

    // expect the same file back, byte for byte
    expect(agentsFile(STANDARD, existing)).toEqual(existing);
});

test("append an empty custom section to an AGENTS.md without one, or to none", () => {
    // regenerate a file without a custom section, and write a first one
    const written = [agentsFile(STANDARD, STANDARD), agentsFile(STANDARD, undefined)];

    // expect the standard with an empty custom section in both
    const expected = `${STANDARD}\n## Custom\n`;
    expect(written).toEqual([expected, expected]);
});

test("replace an outdated standard part of an AGENTS.md, keeping its custom section", () => {
    // regenerate a file written from an earlier standard
    const existing = "# Agents\n\n- **PS01** Old rule.\n\n## Custom\n\nOwn rule.\n";

    // expect the current standard above the same custom section
    expect(agentsFile(STANDARD, existing)).toEqual(`${STANDARD}\n## Custom\n\nOwn rule.\n`);
});

test("write a workspace's AGENTS.md from the toolchain's standard, and refuse one edited above its custom section", async () => {
    // configure a workspace root, then edit its standard part
    const { files } = testHost();
    const workspace = await files.makeTemporaryDirectory("agents-");
    await files.writeTextFile(join(workspace, "package.json"), JSON.stringify({ workspaces: [] }));
    await configurePackage(workspace, Toolchain.workspace(), files);
    const written = await files.readTextFile(join(workspace, "AGENTS.md"));
    await checkConfiguration(workspace, files);
    await files.writeTextFile(join(workspace, "AGENTS.md"), `Edited.\n${written}`);
    const refused = await refusal(checkConfiguration(workspace, files));
    await files.remove(workspace, { recursive: true });

    // expect the standard with an empty custom section, and the edit refused
    const standard = await files.readTextFile(join(Toolchain.packageDirectory(), "AGENTS.md"));
    expect({ written, refused }).toEqual({
        written: `${standard.trimEnd()}\n\n## Custom\n`,
        refused: [
            "BAD_REQUEST",
            "CONFIGURATION_INVALID",
            "'AGENTS.md' is invalid: it differs from the toolchain's standard; run configure",
        ],
    });
});

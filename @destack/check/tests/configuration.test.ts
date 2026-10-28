import { testSimulation } from "@destack/test/world";
import type { FileSystem } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";
import { expect, refusal, test } from "@destack/test";
import { join } from "node:path";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import {
    checkConfiguration,
    configurePackage,
    settingsRoot,
    typescriptConfiguration,
    lintSettings,
} from "../src/check/configuration.ts";
import { readExpectations, applyExpectations } from "../src/check/expectation.ts";

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
            lib: [...LANGUAGE, "DOM", "DOM.Iterable"],
            jsx: "preserve",
            jsxImportSource: "@destack/view",
            types: ["@destack/view/client"],
        },
        include: ["src", "tests", "*.config.ts"],
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
    expect(compilerOptions).toEqual({ ...TYPESCRIPT_OPTIONS, lib: [...LANGUAGE], types: ["bun"] });
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
    await configurePackage(directory, files);
    await checkConfiguration(directory, files);
    const path = join(directory, "tsconfig.json");
    const source = await files.readTextFile(path);
    await files.writeTextFile(path, source.replace('"strict": true', '"strict": false'));
    expect(await refusal(checkConfiguration(directory, files))).toEqual([
        "BAD_REQUEST",
        "CONFIGURATION_INVALID",
        "tsconfig.json differs from the shared configuration; run configure",
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
        JSON.stringify({ workspaces: ["member"] }),
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
        }),
    );
    await files.writeTextFile(
        join(member, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01a0c80b-614d-75ac-841e-b27280bf3db3",
            language: "typescript",
            runtimes: ["bun"],
            entrypoints: ["src/worker.ts"],
        }),
    );

    // configure both, the member taking only its compiler settings
    await configurePackage(member, files);
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
    expect(await lintSettings(workspace, files)).toEqual({
        packages: ["member"],
        entrypoints: [
            "member/src/main.ts",
            "member/src/worker.ts",
            "member/scripts/refresh.ts",
            "member/src/preload.ts",
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

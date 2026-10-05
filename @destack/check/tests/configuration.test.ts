import { expect, onTestFinished, test } from "vitest";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { TYPESCRIPT_OPTIONS } from "@destack/package/build";
import { CheckError } from "../src/error/index.ts";
import {
    checkConfiguration,
    configurePackage,
    settingsRoot,
    typescriptConfiguration,
} from "../src/check/configuration.ts";
import { readExpectations, applyExpectations } from "../src/check/expectation.ts";

/** Write a package with its manifests and sources into a temporary directory. */
async function writePackage(
    runtimes: string[],
    manifest: Record<string, unknown>,
    files: Record<string, string>,
): Promise<string> {
    const directory = await mkdtemp(join(tmpdir(), "destack-configuration-"));
    const definition = {
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01a0c80b-614d-75ac-841e-b27280bf3db3",
        language: "typescript",
        runtimes,
    };
    await writeFile(join(directory, "destack.json"), JSON.stringify(definition));
    await writeFile(join(directory, "package.json"), JSON.stringify(manifest));
    for (const [path, source] of Object.entries(files)) {
        await mkdir(join(directory, path, ".."), { recursive: true });
        await writeFile(join(directory, path), source);
    }

    return directory;
}

/** Report whether a file exists. */
function exists(path: string): Promise<boolean> {
    return readFile(path, "utf8").then(
        () => true,
        () => false,
    );
}

/** Build a finding of a rule in a file. */
function finding(filename: string, code: string) {
    return { code, message: "finding", severity: "error" as const, filename, labels: [] };
}

test("compile a browser package with views against DOM and the view JSX runtime", async () => {
    const directory = await writePackage(
        ["browser"],
        { name: "@example/app", dependencies: { "@destack/view": "2026.9.0" } },
        { "src/app.tsx": "export {};\n" },
    );
    try {
        expect(await typescriptConfiguration(directory)).toEqual({
            compilerOptions: {
                ...TYPESCRIPT_OPTIONS,
                lib: ["ESNext", "DOM", "DOM.Iterable"],
                jsx: "preserve",
                jsxImportSource: "@destack/view",
            },
            include: ["src", "tests", "*.config.ts"],
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("compile a Bun-only package against Bun's types without DOM", async () => {
    const directory = await writePackage(
        ["bun"],
        { name: "@example/server", devDependencies: { "@types/bun": "1.4.2" } },
        { "src/index.ts": "export {};\n" },
    );
    try {
        const { compilerOptions } = await typescriptConfiguration(directory);
        expect(compilerOptions).toEqual({ ...TYPESCRIPT_OPTIONS, lib: ["ESNext"], types: ["bun"] });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("compile a Bun package with Solid's terminal renderer against DOM and the renderer's JSX runtime", async () => {
    const directory = await writePackage(
        ["bun"],
        {
            name: "@example/terminal",
            dependencies: { "@opentui/solid": "0.1.0" },
            devDependencies: { "@types/bun": "1.4.2" },
        },
        { "src/view.tsx": "export {};\n" },
    );
    try {
        const { compilerOptions } = await typescriptConfiguration(directory);
        expect(compilerOptions).toEqual({
            ...TYPESCRIPT_OPTIONS,
            lib: ["ESNext", "DOM", "DOM.Iterable"],
            types: ["bun"],
            jsx: "preserve",
            jsxImportSource: "@opentui/solid",
        });
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("refuse a hand-edited tsconfig.json", async () => {
    const directory = await writePackage(["bun"], { name: "@example/server" }, {});
    try {
        // accept the written configuration and refuse an edited option
        await configurePackage(directory);
        await checkConfiguration(directory);
        const path = join(directory, "tsconfig.json");
        const source = await readFile(path, "utf8");
        await writeFile(path, source.replace('"strict": true', '"strict": false'));
        await expect(checkConfiguration(directory)).rejects.toThrow(
            new CheckError(
                "CONFIGURATION",
                "tsconfig.json differs from the shared configuration; run configure",
            ),
        );
    } finally {
        await rm(directory, { recursive: true });
    }
});

test("write lint and format settings at the workspace root and compiler settings in each member", async () => {
    // place a member package inside a workspace root of its own temporary directory
    const workspace = await mkdtemp(join(tmpdir(), "destack-workspace-"));
    onTestFinished(() => rm(workspace, { recursive: true }));
    const member = join(workspace, "member");
    await mkdir(member);
    await writeFile(join(workspace, "package.json"), JSON.stringify({ workspaces: ["member"] }));
    await writeFile(join(member, "package.json"), JSON.stringify({ name: "@example/member" }));
    await writeFile(
        join(member, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01a0c80b-614d-75ac-841e-b27280bf3db3",
            language: "typescript",
            runtimes: ["bun"],
        }),
    );

    // configure both, the member taking only its compiler settings
    await configurePackage(member);
    expect([
        await exists(join(member, "tsconfig.json")),
        await exists(join(member, ".oxlintrc.json")),
        await exists(join(member, ".oxfmtrc.json")),
    ]).toEqual([true, false, false]);

    // read the root's own expectations beside the member's, each against its directory
    const expectation = {
        files: ["scripts/release.ts"],
        rules: ["eslint/no-console"],
        reason: "the release script reports to its terminal",
    };
    await writeFile(
        join(workspace, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            workspace: { check: { expect: [expectation] } },
        }),
    );
    expect(await readExpectations(workspace)).toEqual([{ directory: workspace, expectation }]);

    // find the workspace's settings from the root, the member and a directory inside it
    expect(await Promise.all([workspace, member, join(member, "src")].map(settingsRoot))).toEqual([
        workspace,
        workspace,
        workspace,
    ]);
});

test("accept the findings a package expects and refuse an expectation nothing matches", async () => {
    // expect console use in one file and an unused rule in another
    const directory = await mkdtemp(join(tmpdir(), "destack-expectation-"));
    onTestFinished(() => rm(directory, { recursive: true }));
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

    // keep the unexpected finding, drop the expected one, and report the stale expectation
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

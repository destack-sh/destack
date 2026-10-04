import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createServer } from "vite";
import { DependencyResolution } from "@destack/package";
import { expect, test } from "@destack/test";
import { buildPackage } from "../src/index.ts";
import { resolutionPlugin } from "../src/compile/dependency.ts";
import { openSource } from "../src/source/index.ts";

/** The schema every package definition names. */
const SCHEMA = "https://destack.app/schemas/2026.9.0/destack.json";

/** Write a package's manifest, definition and one module into a directory. */
async function writePackage(
    directory: string,
    manifest: Readonly<Record<string, unknown>>,
    definition: { readonly id: string; readonly runtimes: readonly string[] },
    module: string,
): Promise<void> {
    await mkdir(join(directory, "src"), { recursive: true });
    await writeFile(
        join(directory, "package.json"),
        JSON.stringify({ version: "2026.9.0", type: "module", ...manifest }),
    );
    await writeFile(
        join(directory, "destack.json"),
        JSON.stringify({ $schema: SCHEMA, language: "typescript", ...definition }),
    );
    await writeFile(join(directory, "src", "index.ts"), module);
}

/** Write an application importing three linked dependencies, one of them undeclared, returning the root. */
async function writeApplication(): Promise<string> {
    // write a dependency for the server only, one the host pins to another release, and one undeclared
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-build-resolution-")));
    const names = ["server", "pinned", "undeclared"];
    for (const [index, name] of names.entries()) {
        await writePackage(
            join(root, name),
            { name: `@fixture/${name}`, exports: { ".": "./src/index.ts" } },
            {
                id: `package-01a10a00-0000-7000-8000-00000000000${index + 1}`,
                runtimes: name === "server" ? ["bun"] : ["browser", "bun"],
            },
            `export const ${name} = "${name}";\n`,
        );
        await mkdir(join(root, "application", "node_modules", "@fixture"), { recursive: true });
        await symlink(
            join(root, name),
            join(root, "application", "node_modules", "@fixture", name),
        );
    }

    // write the application importing each from its own module
    await writePackage(
        join(root, "application"),
        {
            name: "@fixture/application",
            exports: { ".": "./src/index.ts" },
            dependencies: { "@fixture/server": "2026.9.0", "@fixture/pinned": "2026.9.0" },
        },
        { id: "package-01a10a00-0000-7000-8000-000000000004", runtimes: ["browser", "bun"] },
        "export {};\n",
    );
    for (const name of names) {
        await writeFile(
            join(root, "application", "src", `${name}.ts`),
            `export { ${name} } from "@fixture/${name}";\n`,
        );
    }

    return root;
}

test("refuse an undeclared, differently selected or unsupported dependency in the development server", async () => {
    // serve the application with the host pinning another release of one dependency
    const root = await writeApplication();
    const pinned = DependencyResolution.parse({
        kind: "source",
        package: {
            id: "package-01a10a00-0000-7000-8000-000000000002",
            name: "@fixture/pinned",
            version: "2026.8.0",
        },
        files: [],
    });
    try {
        await using source = await openSource({
            directory: join(root, "application"),
            runtime: "browser",
        });
        const server = await createServer({
            root: source.directory,
            configFile: false,
            logLevel: "silent",
            plugins: [resolutionPlugin(source, { "@fixture/pinned": pinned }, "browser")],
            server: { middlewareMode: true, ws: false },
        });
        try {
            // transform each module for the browser, reading each refusal
            const refusals = await Promise.all(
                ["undeclared", "pinned", "server"].map((name) =>
                    server.transformRequest(`/src/${name}.ts`).then(
                        () => "transformed",
                        (error: unknown) => (error instanceof Error ? error.message : "unknown"),
                    ),
                ),
            );
            expect(refusals).toEqual([
                "undeclared runtime dependency: @fixture/undeclared",
                "dependency resolution differs from installation: @fixture/pinned",
                "unsupported browser dependency: @fixture/server",
            ]);
        } finally {
            await server.close();
        }
    } finally {
        await rm(root, { recursive: true });
    }
});

test("refuse an unsupported dependency in a build as the development server does", async () => {
    // let the sandboxed compiler read the dependencies beside the application as one workspace
    const root = await writeApplication();
    await writeFile(join(root, "bun.lock"), JSON.stringify({ lockfileVersion: 1, workspaces: {} }));

    // build the module importing a dependency for the server only, for the server and for the browser
    const directory = join(root, "application");
    await writeFile(join(directory, "src", "index.ts"), 'export { server } from "./server.ts";\n');
    try {
        const build = await buildPackage({
            directory,
            dependencies: {},
            outputs: { bun: { kind: "module", runtime: "bun", bundle: true } },
        });
        expect(Object.keys(build.manifest.outputs)).toEqual(["bun"]);
        await expect(
            buildPackage({
                directory,
                dependencies: {},
                outputs: { browser: { kind: "module", runtime: "browser", bundle: true } },
            }),
        ).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: "unsupported browser dependency: @fixture/server",
        });
    } finally {
        await rm(root, { recursive: true });
    }
});

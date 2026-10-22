import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createServer } from "vite";
import { DependencyResolution } from "@destack/package";
import { expect, test } from "@destack/test";
import { Compiler, Workspace, CompilerPool } from "../src/index.ts";
import { resolutionPlugin } from "../src/compile/dependency.ts";
import { PackageSource } from "../src/source/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";

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

/** Write an application importing three linked dependencies, one undeclared, returning its root. */
async function writeApplication(): Promise<string> {
    // write a dependency for the server only, one the host locks to another release, and one undeclared
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

    // write the application importing each from a separate module
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
        await using source = await PackageSource.open({
            directory: join(root, "application"),
            runtime: "browser",
            tree: {},
            roots: "package",
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

test("resolve a dependency the build releases at a given version exactly, refusing any other version", async () => {
    // build the application importing one dependency the host selects at a version
    const root = await writeApplication();
    await writeFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
    );
    const directory = join(root, "application");
    await writeFile(join(directory, "src", "index.ts"), 'export { pinned } from "./pinned.ts";\n');
    const dev = { "@fixture/application": "2026.9.0-dev.4", "@fixture/pinned": "2026.9.0-dev.4" };
    await using store = await LocalPackageStore.temporary();
    await using compiler = await Compiler.start(directory, { store });
    const build = (version: string, versions?: typeof dev) =>
        compiler.build({
            directory,
            dependencies: {
                "@fixture/pinned": DependencyResolution.parse({
                    kind: "source",
                    package: {
                        id: "package-01a10a00-0000-7000-8000-000000000002",
                        name: "@fixture/pinned",
                        version,
                    },
                    files: [],
                }),
            },
            outputs: { browser: { kind: "module", runtime: "browser" } },
            ...(versions === undefined ? {} : { versions }),
        });
    try {
        // release the application and its dependency at the dev version
        await using built = await build("2026.9.0-dev.4", dev);
        expect(built.manifest.package.version).toEqual("2026.9.0-dev.4");

        // refuse the declared version in a dev build, and the dev version in a release build
        const refusal = {
            code: "BUILD_FAILED",
            message: "dependency resolution differs from installation: @fixture/pinned",
        };
        await expect(build("2026.9.0", dev)).rejects.toMatchObject(refusal);
        await expect(build("2026.9.0-dev.4")).rejects.toMatchObject(refusal);
    } finally {
        await rm(root, { recursive: true });
    }
});

test("refuse an unsupported dependency in a build as the development server does", async () => {
    // let the dependencies beside the application be the members of one checkout
    const root = await writeApplication();
    const members = ["server", "pinned", "undeclared", "application"];
    const workspaces = Object.fromEntries(
        members.map((name) => [name, { name: `@fixture/${name}` }]),
    );
    await writeFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces, packages: {} }),
    );
    await writeFile(join(root, ".gitignore"), "node_modules/\n");
    expect(Bun.spawnSync(["git", "init", "--quiet", root]).exitCode).toBe(0);

    // build the module importing a dependency for the server only, for the server and for the browser
    const directory = join(root, "application");
    await writeFile(join(directory, "src", "index.ts"), 'export { server } from "./server.ts";\n');
    try {
        await using store = await LocalPackageStore.temporary();
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(directory, compilers);
        await using build = await workspace.build({
            outputs: { bun: { kind: "module", runtime: "bun" } },
        });
        expect({
            outputs: Object.keys(build.manifest.outputs),
            server: build.manifest.outputs["bun"]?.dependencies["@fixture/server"]?.kind,
        }).toEqual({ outputs: ["bun"], server: "destack" });
        await expect(
            compilers.use(directory, (compiler) =>
                compiler.build({
                    directory,
                    dependencies: {},
                    outputs: { browser: { kind: "module", runtime: "browser" } },
                }),
            ),
        ).rejects.toMatchObject({
            code: "BUILD_FAILED",
            message: "unsupported browser dependency: @fixture/server",
        });
    } finally {
        await rm(root, { recursive: true });
    }
});

test("build a browser-safe export whose entry re-exports a sibling of a package that runs in Bun alone", async () => {
    // write a Bun package with a browser export that re-exports a sibling module
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-build-export-")));
    await writeFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
    );
    const library = join(root, "library");
    await writePackage(
        library,
        {
            name: "@fixture/library",
            exports: { ".": "./src/index.ts", "./release": "./src/release/index.ts" },
        },
        { id: "package-01a10a00-0000-7000-8000-000000000011", runtimes: ["bun"] },
        "export const server = true;\n",
    );
    await writeFile(
        join(library, "destack.json"),
        JSON.stringify({
            $schema: SCHEMA,
            language: "typescript",
            id: "package-01a10a00-0000-7000-8000-000000000011",
            runtimes: ["bun"],
            exports: { "./release": { runtimes: ["browser", "bun"] } },
        }),
    );
    await mkdir(join(library, "src", "release"), { recursive: true });
    await writeFile(join(library, "src", "release", "index.ts"), 'export * from "./release.ts";\n');
    await writeFile(
        join(library, "src", "release", "release.ts"),
        'export const channel = "stable";\n',
    );

    // build an application importing the export for the browser
    const directory = join(root, "application");
    await writePackage(
        directory,
        {
            name: "@fixture/application",
            exports: { ".": "./src/index.ts" },
            dependencies: { "@fixture/library": "2026.9.0" },
        },
        { id: "package-01a10a00-0000-7000-8000-000000000012", runtimes: ["browser", "bun"] },
        'export { channel } from "@fixture/library/release";\n',
    );
    await mkdir(join(directory, "node_modules", "@fixture"), { recursive: true });
    await symlink(library, join(directory, "node_modules", "@fixture", "library"));
    try {
        await using store = await LocalPackageStore.temporary();
        await using compiler = await Compiler.start(directory, { store });
        await using build = await compiler.build({
            directory,
            dependencies: {},
            outputs: { browser: { kind: "module", runtime: "browser" } },
        });
        expect(Object.keys(build.manifest.outputs)).toEqual(["browser"]);
    } finally {
        await rm(root, { recursive: true });
    }
});

test("refuse star exports that lead back into a module, listing the cycle", async () => {
    // write a package whose modules star-export each other
    const root = await realpath(await mkdtemp(join(tmpdir(), "destack-build-cycle-")));
    await writeFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
    );
    const directory = join(root, "cycle");
    await writePackage(
        directory,
        { name: "@fixture/cycle", exports: { ".": "./src/index.ts" } },
        { id: "package-01a10a00-0000-7000-8000-000000000013", runtimes: ["bun"] },
        'export * from "./left.ts";\n',
    );
    await writeFile(
        join(directory, "src", "left.ts"),
        'export * from "./right.ts";\nexport const left = 1;\n',
    );
    await writeFile(
        join(directory, "src", "right.ts"),
        'export * from "./left.ts";\nexport const right = 2;\n',
    );
    try {
        await using store = await LocalPackageStore.temporary();
        await using compiler = await Compiler.start(directory, { store });
        await expect(
            compiler.build({
                directory,
                dependencies: {},
                outputs: { bun: { kind: "module", runtime: "bun" } },
            }),
        ).rejects.toMatchObject({
            code: "INSPECTION_FAILED",
            message: "export * cycle: src/left.ts -> src/right.ts -> src/left.ts",
        });
    } finally {
        await rm(root, { recursive: true });
    }
});

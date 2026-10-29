import type { FileSystem } from "@destack/environment";
import { testSimulation } from "../src/test/world.ts";
import { join } from "node:path";
import { createServer } from "vite";
import { DependencyResolution } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
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
    files: FileSystem,
): Promise<void> {
    await files.mkdir(join(directory, "src"), { recursive: true });
    await files.writeTextFile(
        join(directory, "package.json"),
        JSON.stringify({ version: "2026.9.0", type: "module", ...manifest }),
    );
    await files.writeTextFile(
        join(directory, "destack.json"),
        JSON.stringify({ $schema: SCHEMA, language: "typescript", ...definition }),
    );
    await files.writeTextFile(join(directory, "src", "index.ts"), module);
}

/** Write an application importing three linked dependencies, one undeclared, returning its root. */
async function writeApplication(files: FileSystem): Promise<string> {
    // write a dependency for the server only, one the host locks to another release, and one undeclared
    const root = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-resolution-"),
    );
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
            files,
        );
        await files.mkdir(join(root, "application", "node_modules", "@fixture"), {
            recursive: true,
        });
        await files.symlink(
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
        files,
    );
    for (const name of names) {
        await files.writeTextFile(
            join(root, "application", "src", `${name}.ts`),
            `export { ${name} } from "@fixture/${name}";\n`,
        );
    }

    return root;
}

test("refuse an undeclared, differently selected or unsupported dependency in the development server", async () => {
    // serve the application with the host pinning another release of one dependency
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
    const pinned = DependencyResolution.parse({
        kind: "source",
        package: {
            id: "package-01a10a00-0000-7000-8000-000000000002",
            name: "@fixture/pinned",
            version: "2026.8.0",
        },
        files: [],
    });
    const source = await PackageSource.open(
        { directory: join(root, "application"), runtime: "browser", tree: {} },
        environment,
    );
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
});

test("refuse an unsupported dependency in a build as the development server does", async () => {
    // let the dependencies beside the application be the members of one checkout
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await writeApplication(files);
    const members = ["server", "pinned", "undeclared", "application"];
    const workspaces = Object.fromEntries(
        members.map((name) => [name, { name: `@fixture/${name}` }]),
    );
    await files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces, packages: {} }),
    );
    await files.writeTextFile(join(root, ".gitignore"), "node_modules/\n");
    expect((await environment.process.spawn(["git", "init", "--quiet", root]).output()).code).toBe(
        0,
    );

    // build the module importing a dependency for the server only, for the server and for the browser
    const directory = join(root, "application");
    await files.writeTextFile(
        join(directory, "src", "index.ts"),
        'export { server } from "./server.ts";\n',
    );
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const workspace = await Workspace.open(directory, compilers);
    await using build = await workspace.build({
        outputs: { bun: { kind: "module", runtime: "bun" } },
    });
    expect({
        outputs: Object.keys(build.manifest.outputs),
        server: build.manifest.outputs["bun"]?.dependencies["@fixture/server"]?.kind,
    }).toEqual({ outputs: ["bun"], server: "destack" });
    const dependencies = await workspace.storeDependencies({});
    expect(
        await refusal(
            compilers.use(directory, (compiler) =>
                compiler.build({
                    directory,
                    dependencies,
                    outputs: { browser: { kind: "module", runtime: "browser" } },
                }),
            ),
        ),
    ).toEqual([
        "UNPROCESSABLE_CONTENT",
        "RUNTIME_MODULE_UNSUPPORTED",
        "unsupported browser dependency: @fixture/server",
    ]);
});

test("build a browser-safe export whose entry re-exports a sibling of a package that runs in Bun alone", async () => {
    // write a Bun package with a browser export that re-exports a sibling module
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-build-export-"));
    const workspaces = {
        library: { name: "@fixture/library" },
        application: { name: "@fixture/application" },
    };
    await files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces, packages: {} }),
    );
    await files.writeTextFile(join(root, ".gitignore"), "node_modules/\n");
    const library = join(root, "library");
    await writePackage(
        library,
        {
            name: "@fixture/library",
            exports: { ".": "./src/index.ts", "./release": "./src/release/index.ts" },
        },
        { id: "package-01a10a00-0000-7000-8000-000000000011", runtimes: ["bun"] },
        "export const server = true;\n",
        files,
    );
    await files.writeTextFile(
        join(library, "destack.json"),
        JSON.stringify({
            $schema: SCHEMA,
            language: "typescript",
            id: "package-01a10a00-0000-7000-8000-000000000011",
            runtimes: ["bun"],
            exports: { "./release": { runtimes: ["browser", "bun"] } },
        }),
    );
    await files.mkdir(join(library, "src", "release"), { recursive: true });
    await files.writeTextFile(
        join(library, "src", "release", "index.ts"),
        'export * from "./release.ts";\n',
    );
    await files.writeTextFile(
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
        files,
    );
    await files.mkdir(join(directory, "node_modules", "@fixture"), { recursive: true });
    await files.symlink(library, join(directory, "node_modules", "@fixture", "library"));
    expect((await environment.process.spawn(["git", "init", "--quiet", root]).output()).code).toBe(
        0,
    );
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const workspace = await Workspace.open(directory, compilers);
    await using build = await workspace.build({
        outputs: { browser: { kind: "module", runtime: "browser" } },
    });

    // load the library's browser export from its own browser output
    expect({
        outputs: Object.keys(build.manifest.outputs),
        library: build.manifest.outputs["browser"]?.dependencies["@fixture/library"]?.kind,
    }).toEqual({ outputs: ["browser"], library: "destack" });
});

test("refuse indexing star exports that lead back into a module, listing the cycle", async () => {
    // write a package whose modules star-export each other
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-build-cycle-"));
    await files.writeTextFile(
        join(root, "bun.lock"),
        JSON.stringify({ lockfileVersion: 1, workspaces: {}, packages: {} }),
    );
    const directory = join(root, "cycle");
    await writePackage(
        directory,
        { name: "@fixture/cycle", exports: { ".": "./src/index.ts" } },
        { id: "package-01a10a00-0000-7000-8000-000000000013", runtimes: ["bun"] },
        'export * from "./left.ts";\n',
        files,
    );
    await files.writeTextFile(
        join(directory, "src", "left.ts"),
        'export * from "./right.ts";\nexport const left = 1;\n',
    );
    await files.writeTextFile(
        join(directory, "src", "right.ts"),
        'export * from "./left.ts";\nexport const right = 2;\n',
    );
    await using store = await LocalPackageStore.temporary(environment);
    await using compiler = await Compiler.start(directory, { store });
    expect(await refusal(compiler.index({ directory, runtime: "bun", dependencies: {} }))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "INSPECTION_FAILED",
        "export * cycle: src/left.ts -> src/right.ts -> src/left.ts",
    ]);
});

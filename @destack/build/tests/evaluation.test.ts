import type { Environment, OperatingSystem } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { testSimulation } from "../src/test/world.ts";
import { join } from "node:path";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Workspace, CompilerPool } from "../src/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { LocalBuildStore } from "../src/store/local.ts";
import { writeThing } from "./fixture.ts";
import { Toolchain } from "@destack/check/toolchain";

/** One Bun module output of the app. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The error a denied read reports: Seatbelt refuses it, bubblewrap hides the file. */
const DENIED_READ: Partial<Record<OperatingSystem, string>> = { darwin: "EPERM", linux: "ENOENT" };

/** Write the thing kind's package and an app module into a fresh workspace, answering the app. */
async function thingPackage(
    app: string,
    environment: Environment<"files" | "process">,
): Promise<string> {
    const { files } = environment;
    const directory = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-thing-"),
    );

    return await writeThing(directory, app, environment);
}

/** Read the names of a build's things. */
async function thingNames(build: BuildReader): Promise<unknown[]> {
    const declarations = await build.declarations();

    return declarations.map((declaration) => declaration.description["name"]);
}

test("evaluate package code without the host's environment, files or network", async () => {
    // write a host file outside the package and serve a host server
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const server = testHost().server.http({ port: 0, fetch: () => new Response("answered") });
    onTestFinished(() => server.stop(true));
    const outside = await files.realPath(
        await files.makeTemporaryDirectory("destack-build-sandbox-"),
    );
    const secret = join(outside, "secret.txt");
    await files.writeTextFile(secret, "secret");

    // name a thing by what package code observes of the host while the build evaluates it
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\nimport { observed } from "./probe.ts";\n\nexport const greeting = defineThing({ name: observed });\n',
        environment,
    );
    const probe = join(directory, "src/probe.ts");
    await files.writeTextFile(
        probe,
        `/** The host process, as Bun provides it. */
declare const process: Readonly<Record<string, Readonly<Record<string, string>> | undefined>>;
/** Bun's file reader. */
declare const Bun: { file(path: string): { text(): Promise<string> } };

/** The host's user, the host file's text or its refusal, and the host server's reply. */
export const observed = [
    process["env"]?.["USER"],
    await read(${JSON.stringify(secret)}),
    (await fetch(${JSON.stringify(server.url.href)})).status,
].join("_");

/** Read a file's text, or the code of the error refusing it. */
async function read(path: string): Promise<string> {
    try {
        return await Bun.file(path).text();
    } catch (error) {
        return error instanceof Error && "code" in error ? String(error.code) : String(error);
    }
}
`,
    );

    // see no user, a refused file and the sandbox proxy's refusal
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const build = await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    expect(await thingNames(build)).toEqual([
        `_${DENIED_READ[environment.host.os] ?? "unsupported"}_403`,
    ]);
});

test("declare each exported value a constructor made, whether or not an exported list sorting first holds it", async () => {
    // export a list of two things beside both and a third outside the list
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const directory = await thingPackage(
        `import { defineThing } from "@example/owner";

export const first = defineThing({ name: "first" });
export const second = defineThing({ name: "second" });
export const third = defineThing({ name: "third" });
export const THINGS = [first, second];
`,
        environment,
    );

    // declare all three things once
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const build = await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    expect(await thingNames(build)).toEqual(["first", "second", "third"]);
});

test("reinspect edited modules a declaration imports in a retained compiler", async () => {
    // declare a thing whose name a title module returns, and write a noisy module beside it
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\nimport { title } from "./title.ts";\n\nexport const greeting = defineThing({ name: title() });\n',
        environment,
    );
    const title = join(directory, "src/title.ts");
    const original =
        '/** Return the thing\'s name. */\nexport function title(): string {\n    return "title";\n}\n';
    await files.writeTextFile(title, original);
    await files.writeTextFile(
        join(directory, "src/noisy.ts"),
        '/** Log a name and return it. */\nexport function noisy(name: string): string {\n    console.info("inspecting", name);\n\n    return name;\n}\n',
    );
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const build = async () =>
        await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    const first = await build();

    // make the title call the noisy module, which writes to standard output, then restore it
    await files.writeTextFile(
        title,
        'import { noisy } from "./noisy.ts";\n\n/** Return the thing\'s name. */\nexport function title(): string {\n    return noisy("heading");\n}\n',
    );
    const edited = await build();
    await files.writeTextFile(title, original);
    const restored = await build();

    // describe the edited name, and restore the first manifest and every distributed file
    expect({
        names: [await thingNames(first), await thingNames(edited)],
        restored: restored.manifest,
        files: await restored.distributed(),
    }).toEqual({
        names: [["title"], ["heading"]],
        restored: first.manifest,
        files: await first.distributed(),
    });
});

test("refuse a declared kind function its module does not export, by its package export and name", async () => {
    // declare the thing kind's vocabulary at an export its module lacks
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\n\nexport const greeting = defineThing({ name: "greeting" });\n',
        environment,
    );
    const definition = join(directory, "../owner/destack.json");
    const text = await files.readTextFile(definition);
    const describe = '"function":"./inspect#describe"';
    expect(text.includes(describe)).toBe(true);
    await files.writeTextFile(
        definition,
        text.replace(describe, `${describe},"vocabulary":"./inspect#missing"`),
    );

    // refuse the build while it evaluates the declaration
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspace = await Workspace.open(directory, compilers);

    expect(await refusal(workspace.build({ outputs: OUTPUTS }))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "INSPECTION_FAILED",
        "declaration evaluation failed: @example/owner/inspect#missing is not an exported function",
    ]);
});

/** Write a part kind's package beside the thing kind's, which reads a thing's part as its value. */
async function partPackage(app: string, environment: Environment<"files" | "process">) {
    // declare the part kind by its own package's constructor, linked into the thing kind's
    const { files } = environment;
    const directory = await thingPackage(app, environment);
    const parts = join(directory, "../parts");
    await files.mkdir(parts, { recursive: true });
    await files.writeTextFile(
        join(parts, "package.json"),
        JSON.stringify({
            name: "@example/parts",
            version: "2026.9.0",
            type: "module",
            exports: { ".": "./index.ts", "./inspect": "./inspect.ts" },
        }),
    );
    await files.writeTextFile(
        join(parts, "index.ts"),
        "export function definePart(definition: { part: string }) {\n    return definition;\n}\n",
    );
    await files.writeTextFile(
        join(parts, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-00000000000d",
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
            declarations: {
                definePart: { describes: [{ kind: "part", function: "./inspect#describe" }] },
            },
        }),
    );
    await files.writeTextFile(
        join(parts, "inspect.ts"),
        "export function describe(value: { part: string }) {\n    return { name: value.part };\n}\n",
    );
    await files.writeTextFile(
        join(directory, "../bun.lock"),
        JSON.stringify({
            lockfileVersion: 2,
            workspaces: {
                owner: { name: "@example/owner" },
                parts: { name: "@example/parts" },
                app: { name: "@example/app" },
            },
            packages: {},
        }),
    );
    await files.mkdir(join(directory, "../owner/node_modules/@example"), { recursive: true });
    await files.symlink(parts, join(directory, "../owner/node_modules/@example/parts"));
    await files.symlink(parts, join(directory, "node_modules/@example/parts"));

    // let a thing declare a part first, reading none from a thing without one
    const owner = join(directory, "../owner");
    await files.writeTextFile(
        join(owner, "destack.json"),
        JSON.stringify({
            $schema: "https://destack.app/schemas/2026.9.0/destack.json",
            id: "package-01996ab0-0000-7000-8000-00000000000a",
            role: "library",
            language: "typescript",
            runtimes: ["bun"],
            declarations: {
                defineThing: {
                    describes: [
                        { kind: "part", package: "@example/parts", value: "./inspect#part" },
                        { kind: "thing", function: "./inspect#describe" },
                    ],
                },
            },
            stamps: { defineThing: { module: 1 } },
        }),
    );
    await files.writeTextFile(
        join(owner, "package.json"),
        JSON.stringify({
            name: "@example/owner",
            version: "2026.9.0",
            type: "module",
            exports: { ".": "./index.ts", "./inspect": "./inspect.ts" },
            dependencies: { "@example/parts": "2026.9.0" },
        }),
    );
    await files.writeTextFile(
        join(owner, "index.ts"),
        `import { definePart } from "@example/parts";

export function defineThing(definition: { name: string; part?: string }, module?: { package: object }) {
    return { ...definition, package: module!.package, parts: definition.part === undefined ? [] : [definePart({ part: definition.part })] };
}
`,
    );
    await files.writeTextFile(
        join(owner, "inspect.ts"),
        `export function describe(value: { name: string }) {
    return { name: value.name };
}

export function part(value: { parts: { part: string }[] }) {
    return value.parts[0];
}
`,
    );

    return directory;
}

test("declare a dependency's kind by the value a constructor reads, and none where it reads none", async () => {
    // declare a thing with a part and one without
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const directory = await partPackage(
        `import { defineThing } from "@example/owner";

export const plain = defineThing({ name: "plain" });
export const wheeled = defineThing({ name: "wheeled", part: "wheel" });
`,
        environment,
    );

    // describe the part of the thing with one alone
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const build = await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    const declarations = await build.declarations();

    expect(declarations.map((each) => [each.kind, each.description["name"]])).toEqual([
        ["thing", "plain"],
        ["part", "wheel"],
        ["thing", "wheeled"],
    ]);
});

test("name a declaration its module does not export by the first kind describing it", async () => {
    // declare a thing without a part outside the module's exports
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const directory = await partPackage(
        `import { defineThing } from "@example/owner";

const hidden = defineThing({ name: "hidden" });
export const greeting = defineThing({ name: "greeting" });
void hidden;
`,
        environment,
    );

    // refuse it by the thing kind's name, the part kind reading none
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspace = await Workspace.open(directory, compilers);

    expect(await refusal(workspace.build({ outputs: OUTPUTS }))).toEqual([
        "UNPROCESSABLE_CONTENT",
        "DECLARATION_UNEXPORTED",
        "declared hidden is not exported by @example/app",
    ]);
});

import type { Environment, OperatingSystem } from "@destack/environment";
import { testHost } from "@destack/test/host";
import { testSimulation } from "../src/test/world.ts";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { type PackageBuild, Workspace, CompilerPool } from "../src/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";
import { writeThing } from "./fixture.ts";

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
async function thingNames(build: PackageBuild): Promise<unknown[]> {
    const declarations = await build.reader.declarations();

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
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    await using build = await (
        await Workspace.open(directory, compilers)
    ).build({ outputs: OUTPUTS });
    expect(await thingNames(build)).toEqual([
        `_${DENIED_READ[environment.host.os] ?? "unsupported"}_403`,
    ]);
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
    await using store = await LocalPackageStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const build = async () =>
        await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    await using first = await build();

    // make the title call the noisy module, which writes to standard output, then restore it
    await files.writeTextFile(
        title,
        'import { noisy } from "./noisy.ts";\n\n/** Return the thing\'s name. */\nexport function title(): string {\n    return noisy("heading");\n}\n',
    );
    await using edited = await build();
    await files.writeTextFile(title, original);
    await using restored = await build();

    // describe the edited name, and restore the first manifest and every distributed file
    expect({
        names: [await thingNames(first), await thingNames(edited)],
        restored: restored.manifest,
        files: await restored.reader.distributed(),
    }).toEqual({
        names: [["title"], ["heading"]],
        restored: first.manifest,
        files: await first.reader.distributed(),
    });
});

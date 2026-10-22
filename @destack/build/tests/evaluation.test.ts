import { mkdtemp, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { formatSource } from "@destack/check";
import { PackageManifest } from "@destack/package/manifest";
import { type History, Vocabulary } from "@destack/resource";
import { present } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { type PackageBuild, Workspace, CompilerPool } from "../src/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";
import { writeThing } from "./fixture.ts";

/** One Bun module output of the app. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The error a denied read reports: Seatbelt refuses it, bubblewrap hides the file. */
const DENIED_READ: Partial<Record<NodeJS.Platform, string>> = { darwin: "EPERM", linux: "ENOENT" };

/** Write the thing kind's package and an app module into a fresh workspace, answering the app. */
async function thingPackage(app: string): Promise<string> {
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-build-thing-")));
    onTestFinished(() => rm(directory, { recursive: true }));

    return await writeThing(directory, app);
}

/** Read the names of a build's things. */
async function thingNames(build: PackageBuild): Promise<unknown[]> {
    const declarations = await build.reader.declarations();

    return declarations.map((declaration) => declaration.description["name"]);
}

test("evaluate package code without the host's environment, files or network", async () => {
    // write a host file outside the package and serve a host server
    using server = Bun.serve({ port: 0, fetch: () => new Response("answered") });
    const outside = await realpath(await mkdtemp(join(tmpdir(), "destack-build-sandbox-")));
    onTestFinished(() => rm(outside, { recursive: true }));
    const secret = join(outside, "secret.txt");
    await writeFile(secret, "secret");

    // name a thing by what package code observes of the host while the build evaluates it
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\nimport { observed } from "./probe.ts";\n\nexport const greeting = defineThing({ name: observed });\n',
    );
    const probe = join(directory, "src/probe.ts");
    await writeFile(
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
    await using store = await LocalPackageStore.temporary();
    await using compilers = await CompilerPool.open(store);
    await using build = await (
        await Workspace.open(directory, compilers)
    ).build({ outputs: OUTPUTS });
    expect(await thingNames(build)).toEqual([
        `_${DENIED_READ[process.platform] ?? "unsupported"}_403`,
    ]);
});

test("reinspect edited modules a declaration imports in a retained compiler", async () => {
    // declare a thing whose name a title module returns, and write a noisy module beside it
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\nimport { title } from "./title.ts";\n\nexport const greeting = defineThing({ name: title() });\n',
    );
    const title = join(directory, "src/title.ts");
    const original =
        '/** Return the thing\'s name. */\nexport function title(): string {\n    return "title";\n}\n';
    await writeFile(title, original);
    await writeFile(
        join(directory, "src/noisy.ts"),
        '/** Log a name and return it. */\nexport function noisy(name: string): string {\n    console.info("inspecting", name);\n\n    return name;\n}\n',
    );
    await using store = await LocalPackageStore.temporary();
    await using compilers = await CompilerPool.open(store);
    const build = async () =>
        await (await Workspace.open(directory, compilers)).build({ outputs: OUTPUTS });
    await using first = await build();

    // make the title call the noisy module, which writes to standard output, then restore it
    await writeFile(
        title,
        'import { noisy } from "./noisy.ts";\n\n/** Return the thing\'s name. */\nexport function title(): string {\n    return noisy("heading");\n}\n',
    );
    await using edited = await build();
    await writeFile(title, original);
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

test("plan the upgrade from what a package published", async () => {
    // publish the app's first release, declaring one thing
    const directory = await thingPackage(
        'import { defineThing } from "@example/owner";\n\nexport const greeting = defineThing({ name: "greeting" });\n',
    );
    await using store = await LocalPackageStore.temporary();
    await using compilers = await CompilerPool.open(store);
    const build = async (history?: Readonly<Record<string, History>>) =>
        await (
            await Workspace.open(directory, compilers)
        ).build({ outputs: OUTPUTS, ...(history === undefined ? {} : { history }) });
    await using first = await build();
    const published = await first.reader.declarations();
    const release = PackageManifest.package(first.manifest).version;
    const history = {
        release,
        declarations: published,
        vocabulary: Vocabulary.advance({}, published, release),
    };

    // rename the thing and add another in the next release
    const manifest = join(directory, "package.json");
    await writeFile(
        manifest,
        (await readFile(manifest, "utf8")).replace('"version":"2026.9.0"', '"version":"2026.10.0"'),
    );
    const app = join(directory, "src/app.ts");
    await writeFile(
        app,
        await formatSource(
            app,
            'import { defineThing } from "@example/owner";\n\nexport const welcome = defineThing({ name: "welcome" });\nexport const farewell = defineThing({ name: "farewell" });\n',
        ),
    );
    await using second = await build({ "@example/app": history });

    // plan the next release's steps from the published one
    const upgrade = present(second.manifest.upgrade, "the upgrade");
    const planned: unknown = JSON.parse(
        new TextDecoder().decode(await second.reader.load(upgrade.file.path)),
    );
    expect({ package: PackageManifest.package(second.manifest).version, planned }).toEqual({
        package: "2026.10.0",
        planned: {
            from: "2026.9.0",
            steps: [
                { action: "create", target: "thing/farewell", risk: "safe", detail: "add thing" },
                { action: "create", target: "thing/welcome", risk: "safe", detail: "add thing" },
                {
                    action: "delete",
                    target: "thing/greeting",
                    risk: "backward-incompatible",
                    detail: "remove thing: earlier releases still serve it",
                },
            ],
        },
    });
});

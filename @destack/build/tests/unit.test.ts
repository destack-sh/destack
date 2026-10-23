import { expect, onTestFinished, test } from "@destack/test";
import { cp, mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { CompilerPool, DependencyTree, PackageBuild, Workspace } from "../src/index.ts";
import { PackageManifest } from "@destack/package/manifest";
import { PackageStore } from "../src/store/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";
import { countObjects, exported, importBuilt, property } from "./fixture.ts";

/** One module output per unit, its dependencies external. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The library unit's package name, as the application requires it. */
const LIBRARY = "@destack/build-unit-library-fixture";

/** Copy a unit fixture's source into a workspace directory. */
async function copyUnit(name: string, destination: string): Promise<void> {
    const source = fileURLToPath(new URL(`./fixture/${name}/source/`, import.meta.url));
    await cp(source, destination, {
        recursive: true,
        filter: (path) => !path.split(sep).includes("node_modules"),
    });
}

/** Report whether a built export is the greeting function. */
function isGreeter(value: unknown): value is () => unknown {
    return typeof value === "function";
}

test("build a workspace's units on their dependencies' builds, restore every unit while unchanged, and rebuild the dependents of an edited unit", async () => {
    // lay out a workspace whose application links its library unit
    const directory = await mkdtemp(join(tmpdir(), "destack-build-unit-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const workspace = join(directory, "workspace");
    const library = join(workspace, "library");
    const application = join(workspace, "application");
    await copyUnit("unit-library", library);
    await copyUnit("unit-application", application);
    await writeFile(
        join(workspace, "bun.lock"),
        JSON.stringify({
            lockfileVersion: 2,
            workspaces: {
                library: { name: LIBRARY },
                application: { name: "@destack/build-unit-application-fixture" },
            },
            packages: {},
        }),
    );
    await mkdir(join(application, "node_modules", "@destack"), { recursive: true });
    await symlink(library, join(application, "node_modules", LIBRARY), "junction");
    await writeFile(join(workspace, ".gitignore"), "node_modules/\n");
    expect(Bun.spawnSync(["git", "init", "--quiet", workspace]).exitCode).toBe(0);
    await using store = await LocalPackageStore.temporary();
    await using compilers = await CompilerPool.open(store);
    const units = await Workspace.open(application, compilers);
    const build = async () =>
        await (await Workspace.open(application, compilers)).build({ outputs: OUTPUTS });

    // build both units, then again unchanged without writing to the store
    await using built = await build();
    const stored = await countObjects(store);
    await using restored = await build();
    const unchanged = await countObjects(store);

    // rebuild the application on the edited library's new build
    await writeFile(
        join(library, "src", "greeting.ts"),
        (await Bun.file(join(library, "src", "greeting.ts")).text()).replace("hello", "hi"),
    );
    await using rebuilt = await build();

    // run each application output with its library build installed beside it
    const greetings: unknown[] = [];
    for (const [index, each] of [built, restored, rebuilt].entries()) {
        const destination = join(directory, `application-${index}`);
        await PackageBuild.write(each.reader, destination);
        const dependency = each.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
        if (dependency?.kind !== "destack") {
            throw new TypeError("the application's output records no library build");
        }
        const tree = await DependencyTree.materialize(
            store,
            [dependency],
            join(directory, "builds"),
            workspace,
        );
        await tree.link(join(destination, "node_modules"));
        const module = await importBuilt(destination, exported(each.manifest.outputs["bun"], "."));
        const greetWorld = property(module, "greetWorld");
        greetings.push(isGreeter(greetWorld) ? greetWorld() : greetWorld);
    }

    // refuse a tree needing the first and the edited library builds at once
    const first = {
        kind: "destack",
        package: PackageManifest.package(built.manifest),
        manifest: await PackageStore.digest(built.manifest),
    } as const;
    const edited = rebuilt.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
    if (edited?.kind !== "destack") {
        throw new TypeError("the rebuilt application records no library build");
    }
    const conflict = DependencyTree.materialize(
        store,
        [edited, first],
        join(directory, "builds"),
        workspace,
    ).catch((error: unknown) => (error instanceof Error ? error.message : error));
    const previous = built.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
    const before = previous?.kind === "destack" ? previous.manifest : "";
    const refused = `${LIBRARY} is required as two different builds: ${before} and ${edited.manifest}`;

    // expect the restored build to equal the first one, and the rebuilt one to greet anew
    expect({
        units: units.units.map((unit) => unit.name),
        restored: restored.manifest,
        reused: restored.reused,
        written: unchanged - stored,
        greetings,
        conflict: await conflict,
    }).toEqual({
        units: [LIBRARY, "@destack/build-unit-application-fixture"],
        restored: built.manifest,
        reused: ["bun"],
        written: 0,
        greetings: [
            { name: "world", text: "hello world" },
            { name: "world", text: "hello world" },
            { name: "world", text: "hi world" },
        ],
        conflict: refused,
    });
});

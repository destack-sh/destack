import { testSimulation } from "../src/test/world.ts";
import { expect, refusal, test } from "@destack/test";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CompilerPool, DependencyTree, Workspace } from "../src/index.ts";
import type { BuildReader } from "@destack/package/manifest";
import { PackageManifest } from "@destack/package/manifest";
import { BuildStore } from "../src/store/index.ts";
import { LocalBuildStore } from "../src/store/local.ts";
import { countObjects, exported, importBuilt, property } from "./fixture.ts";
import { copySource } from "../src/test/source.ts";

/** One module output per unit, its dependencies external. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The library unit's package name, as the application requires it. */
const LIBRARY = "@destack/build-unit-library-fixture";

/** Copy a unit fixture's source into a workspace directory. */
async function copyUnit(name: string, destination: string, files: FileSystem): Promise<void> {
    const source = fileURLToPath(new URL(`./fixture/${name}/source/`, import.meta.url));
    await copySource(source, destination, files);
}

/** List a build's files by path and digest. */
async function digests(build: BuildReader): Promise<string[][]> {
    return (await build.files()).map((file) => [file.path, file.digest]);
}

/** Lay out a git workspace whose application links its library unit. */
async function layOut(environment: Environment<"files" | "process">) {
    // copy both units into one workspace
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("destack-build-unit-");
    const workspace = join(directory, "workspace");
    const library = join(workspace, "library");
    const application = join(workspace, "application");
    await copyUnit("unit-library", library, files);
    await copyUnit("unit-application", application, files);

    // link the library into the application, and make the workspace a checkout of both
    await files.mkdir(join(application, "node_modules", "@destack"), { recursive: true });
    await files.symlink(library, join(application, "node_modules", LIBRARY));
    await initializeWorkspace(
        workspace,
        { library: LIBRARY, application: "@destack/build-unit-application-fixture" },
        environment,
    );

    return { directory, workspace, library, application };
}

/** Make a directory a git checkout of a Bun workspace with some members, by path and name. */
async function initializeWorkspace(
    workspace: string,
    members: Readonly<Record<string, string>>,
    environment: Environment<"files" | "process">,
): Promise<void> {
    // write the lockfile listing the members, then initialize the checkout
    const { files, process } = environment;
    const workspaces = Object.fromEntries(
        Object.entries(members).map(([path, name]) => [path, { name }]),
    );
    await files.writeTextFile(
        join(workspace, "bun.lock"),
        JSON.stringify({ lockfileVersion: 2, workspaces, packages: {} }),
    );
    await files.writeTextFile(join(workspace, ".gitignore"), "node_modules/\n");
    expect((await process.spawn(["git", "init", "--quiet", workspace]).output()).code).toBe(0);
}

/** Report whether a built export is the greeting function. */
function isGreeter(value: unknown): value is () => unknown {
    return typeof value === "function";
}

test("build a workspace's units on their dependencies' builds, restore every unit while unchanged, and relink the dependents of an edited unit", async () => {
    // lay out a workspace whose application links its library unit
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { directory, workspace, library, application } = await layOut(environment);
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const units = await Workspace.open(application, compilers);
    const build = async () =>
        await (await Workspace.open(application, compilers)).build({ outputs: OUTPUTS });

    // build both units, then again unchanged without writing to the store
    const built = await build();
    const stored = await countObjects(store);
    const restored = await build();
    const unchanged = await countObjects(store);

    // rebuild the application on the edited library's new build
    await files.writeTextFile(
        join(library, "src", "greeting.ts"),
        (await files.readTextFile(join(library, "src", "greeting.ts"))).replace("hello", "hi"),
    );
    const rebuilt = await build();

    // run each application output with its library build installed beside it
    const greetings: unknown[] = [];
    for (const [index, each] of [built, restored, rebuilt].entries()) {
        const destination = join(directory, `application-${index}`);
        await each.writeDirectory(destination, files);
        const dependency = each.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
        if (dependency?.kind !== "destack") {
            throw new TypeError("the application's output records no library build");
        }
        const tree = await DependencyTree.materialize(
            store,
            [dependency],
            join(directory, "builds"),
            { workspace },
        );
        await tree.link(join(destination, "node_modules"), files);
        const module = await importBuilt(destination, exported(each.manifest.outputs["bun"], "."));
        const greetWorld = property(module, "greetWorld");
        greetings.push(isGreeter(greetWorld) ? greetWorld() : greetWorld);
    }

    // refuse a tree needing the first and the edited library builds at once
    const first = {
        kind: "destack",
        package: PackageManifest.package(built.manifest),
        manifest: await BuildStore.digest(built.manifest, environment.crypto),
    } as const;
    const edited = rebuilt.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
    if (edited?.kind !== "destack") {
        throw new TypeError("the rebuilt application records no library build");
    }
    const conflict = DependencyTree.materialize(store, [edited, first], join(directory, "builds"), {
        workspace,
    }).catch((error: unknown) => (error instanceof Error ? error.message : error));
    const previous = built.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
    if (previous?.kind !== "destack") {
        throw new TypeError("the application records no library build");
    }
    const before = previous.manifest;
    const refused = `${LIBRARY} is required as two different builds: ${before} and ${edited.manifest}`;

    // relink the application on the edited library: its files as they were, its pins anew
    const relinked = {
        files: JSON.stringify(await digests(rebuilt)) === JSON.stringify(await digests(built)),
        manifest: JSON.stringify(rebuilt.manifest) === JSON.stringify(built.manifest),
    };

    // expect the restored build to equal the first one, and the rebuilt one to greet anew
    expect({
        units: units.units.map((unit) => unit.name),
        restored: restored.manifest,
        relinked,
        written: unchanged - stored,
        greetings,
        conflict: await conflict,
        imports: built.manifest.outputs["bun"]?.exports["."]?.imports,
        releases: await built.releases(),
        relinkedReleases: await rebuilt.releases(),
    }).toEqual({
        units: [LIBRARY, "@destack/build-unit-application-fixture"],
        restored: built.manifest,
        relinked: { files: true, manifest: false },
        written: 0,
        greetings: [
            { name: "world", text: "hello world" },
            { name: "world", text: "hello world" },
            { name: "world", text: "hi world" },
        ],
        imports: [LIBRARY],
        releases: [{ package: previous.package, manifest: before }],
        relinkedReleases: [{ package: edited.package, manifest: edited.manifest }],
        conflict: refused,
    });
});

test("refuse to cache a unit whose committed sources changed after the workspace listed them, building it anew next time", async () => {
    // commit the workspace, open it, then edit a committed library file
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, process } = environment;
    const { workspace, library, application } = await layOut(environment);
    const git = async (...arguments_: string[]) =>
        (await process.spawn(["git", "-C", workspace, ...arguments_]).output()).code;
    const committed = [
        await git("add", "."),
        await git(
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ),
    ];
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store);
    const stale = await Workspace.open(application, compilers);
    const greeting = join(library, "src", "greeting.ts");
    const edited = (await files.readTextFile(greeting)).replace("hello", "hi");
    await files.writeTextFile(greeting, edited);

    // build from the stale listing, then from a fresh one
    const refused = await refusal(stale.build({ outputs: OUTPUTS }));
    const built = await (await Workspace.open(application, compilers)).build({ outputs: OUTPUTS });
    const dependency = built.manifest.outputs["bun"]?.dependencies?.[LIBRARY];
    if (dependency?.kind !== "destack") {
        throw new TypeError("the application records no library build");
    }
    const source = await (await store.contents(dependency.manifest)).load("src/greeting.ts");

    // refuse the stale build, and build the edited source once listed again
    expect({
        committed,
        refused,
        source: new TextDecoder().decode(source),
    }).toEqual({
        committed: [0, 0],
        refused: ["CONFLICT", "SOURCE_CHANGED", `${LIBRARY} changed while it built`],
        source: edited,
    });
});

test("build a package against the release of a Destack package installed from outside its workspace, refusing it without a resolver", async () => {
    // build the library as a release in a workspace of its own
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    const directory = await files.makeTemporaryDirectory("destack-build-release-");
    const library = join(directory, "library", "library");
    const application = join(directory, "application", "application");
    await copyUnit("unit-library", library, files);
    await initializeWorkspace(join(directory, "library"), { library: LIBRARY }, environment);
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store);
    const released = await (
        await Workspace.open(library, compilers)
    ).build({
        outputs: OUTPUTS,
    });
    const release = {
        kind: "destack",
        package: PackageManifest.package(released.manifest),
        manifest: await BuildStore.digest(released.manifest, crypto),
    } as const;

    // install it into an application whose workspace has no library member
    await copyUnit("unit-application", application, files);
    await files.mkdir(join(application, "node_modules", "@destack"), { recursive: true });
    await files.symlink(library, join(application, "node_modules", LIBRARY));
    await initializeWorkspace(
        join(directory, "application"),
        { application: "@destack/build-unit-application-fixture" },
        environment,
    );

    // refuse it without a resolver, then build it against the resolved release
    const refused = await refusal(
        (await Workspace.open(application, compilers)).build({ outputs: OUTPUTS }),
    );
    const asked: string[] = [];
    const resolver = async (dependency: { readonly name: string }) => {
        asked.push(dependency.name);

        return release;
    };
    const built = await (
        await Workspace.open(application, compilers, resolver)
    ).build({
        outputs: OUTPUTS,
    });

    // pin the release in its output and its manifest, building the library only once
    expect({
        refused,
        asked,
        pinned: built.manifest.outputs["bun"]?.dependencies?.[LIBRARY],
        releases: await built.releases(),
    }).toEqual({
        refused: [
            "UNPROCESSABLE_CONTENT",
            "DEPENDENCY_UNRESOLVED",
            `the workspace requires Destack packages outside it, which it has no release resolver for: ${LIBRARY}`,
        ],
        asked: [LIBRARY],
        pinned: release,
        releases: [{ package: release.package, manifest: release.manifest }],
    });
});

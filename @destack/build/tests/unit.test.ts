import { testSimulation } from "../src/test/world.ts";
import { expect, refusal, test } from "@destack/test";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CompilerPool, DependencyTree, Workspace } from "../src/index.ts";
import { Lock } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import { BuildStore } from "../src/store/index.ts";
import { LocalBuildStore } from "../src/store/local.ts";
import { countObjects, exported, importBuilt, property } from "./fixture.ts";
import { copySource } from "../src/test/source.ts";
import { Toolchain } from "@destack/check/toolchain";

/** One module output per unit, its dependencies external. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The library unit's package name, as the application requires it. */
const LIBRARY = "@destack/build-unit-library-fixture";

/** Copy a unit fixture's source into a workspace directory. */
async function copyUnit(name: string, destination: string, files: FileSystem): Promise<void> {
    const source = fileURLToPath(new URL(`./fixture/${name}/source/`, import.meta.url));
    await copySource(source, destination, files);
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

test("build a workspace's units once, keep the application's build on a library edit, and lock the edited library", async () => {
    // lay out a workspace whose application links its library unit
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const { directory, workspace, library, application } = await layOut(environment);
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const units = await (await Workspace.open(application, compilers)).package();
    const build = async () => {
        const opened = await (await Workspace.open(application, compilers)).package();
        const built = await opened.build({ outputs: OUTPUTS });
        const inputs = await opened.storeDependencies({});
        const unit = present(inputs[LIBRARY], "the library unit");
        const lock = await store.lock(await opened.lock(await built.digest()));

        return { built, library: unit.manifest, lock };
    };

    // build both units and lock them
    const first = await build();
    const stored = await countObjects(store);
    const restored = await build();
    const unchanged = await countObjects(store);

    // build again once the library's implementation changed
    await files.writeTextFile(
        join(library, "src", "greeting.ts"),
        (await files.readTextFile(join(library, "src", "greeting.ts"))).replace("hello", "hi"),
    );
    const edited = await build();

    // run each application output with its locked library placed beside it
    const greetings: unknown[] = [];
    for (const [index, { built, lock }] of [first, restored, edited].entries()) {
        const destination = join(directory, `application-${index}`);
        await built.writeDirectory(destination, files);
        const entries = await store.entriesOf(lock);
        const tree = await DependencyTree.materialize(
            store,
            [present(entries[LIBRARY], "the library build")],
            () => undefined,
            join(directory, "builds"),
            { kind: "compile" },
            { workspace },
        );
        await tree.link(join(destination, "node_modules"), files);
        const module = await importBuilt(destination, exported(built.manifest.outputs["bun"], "."));
        const greetWorld = property(module, "greetWorld");
        greetings.push(isGreeter(greetWorld) ? greetWorld() : greetWorld);
    }

    // keep one application build under every lock
    const libraries = [first, restored, edited].map(({ lock }) =>
        Lock.resolve(lock, lock.root, LIBRARY),
    );
    expect({
        units: units.units.map((unit) => unit.name),
        written: unchanged - stored,
        roots: [restored.lock.root, edited.lock.root],
        libraries,
        greetings,
        imports: first.built.manifest.outputs["bun"]?.exports["."]?.imports,
        requires: first.built.manifest.requires,
    }).toEqual({
        units: [LIBRARY, "@destack/build-unit-application-fixture"],
        written: 0,
        roots: [first.lock.root, first.lock.root],
        libraries: [first.library, first.library, edited.library],
        greetings: [
            { name: "world", text: "hello world" },
            { name: "world", text: "hello world" },
            { name: "world", text: "hi world" },
        ],
        imports: [LIBRARY],
        requires: { [LIBRARY]: "2026.9.0" },
    });
});

test("refuse to cache a unit whose committed sources changed after listing, building it anew next time", async () => {
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
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const stale = await (await Workspace.open(application, compilers)).package();
    const greeting = join(library, "src", "greeting.ts");
    const edited = (await files.readTextFile(greeting)).replace("hello", "hi");
    await files.writeTextFile(greeting, edited);

    // build from the stale listing, then from a fresh one
    const refused = await refusal(stale.build({ outputs: OUTPUTS }));
    const fresh = await (await Workspace.open(application, compilers)).package();
    const built = await fresh.build({ outputs: OUTPUTS });
    const lock = await store.lock(await fresh.lock(await built.digest()));
    const dependency = present(Lock.resolve(lock, lock.root, LIBRARY), "the library build");
    const source = await (await store.contents(dependency)).load("src/greeting.ts");

    // refuse the stale build, and build the edited source once listed again
    expect({
        committed,
        refused,
        source: new TextDecoder().decode(source),
    }).toEqual({
        committed: [0, 0],
        refused: [
            "CONFLICT",
            "SOURCE_CONFLICT",
            `cannot build '${LIBRARY}': it changed while it built`,
        ],
        source: edited,
    });
});

test("build and lock a package against a release from outside its workspace, refusing it without a resolver", async () => {
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
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const releasing = await (await Workspace.open(library, compilers)).package();
    const released = await releasing.build({ outputs: OUTPUTS });
    const manifest = await BuildStore.digest(released.manifest, crypto);
    const release = await releasing.lock(manifest);

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
        (await (await Workspace.open(application, compilers)).package()).build({
            outputs: OUTPUTS,
        }),
    );
    const asked: string[] = [];
    const resolver = async (dependency: { readonly name: string }) => {
        asked.push(dependency.name);

        return release;
    };
    const resolving = await (await Workspace.open(application, compilers, { resolver })).package();
    const built = await resolving.build({ outputs: OUTPUTS });

    // require the release's range
    const lock = await store.lock(await resolving.lock(await built.digest()));
    expect({
        refused,
        asked,
        requires: built.manifest.requires,
        lock,
    }).toEqual({
        refused: [
            "BAD_REQUEST",
            "DEPENDENCY_INVALID",
            `workspace is invalid: it requires Destack packages '${LIBRARY}' outside it and has no release resolver for them`,
        ],
        asked: [LIBRARY],
        requires: { [LIBRARY]: "2026.9.0" },
        lock: Lock.of(await built.digest(), {
            [await built.digest()]: { [LIBRARY]: manifest },
            [manifest]: {},
        }),
    });
});

/** Declare a unit a builtin, as the platform's own packages are. */
async function declareBuiltin(unit: string, files: FileSystem): Promise<void> {
    const definition = join(unit, "destack.json");
    const declared = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(definition)));
    await files.writeFile(
        definition,
        new TextEncoder().encode(JSON.stringify({ ...declared, role: "builtin" })),
    );
}

test("declare a builtin release in a build's requirements, leaving it out of its lock", async () => {
    // build the library as a builtin release in a workspace of its own
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    const directory = await files.makeTemporaryDirectory("destack-build-builtin-");
    const library = join(directory, "library", "library");
    const application = join(directory, "application", "application");
    await copyUnit("unit-library", library, files);
    await declareBuiltin(library, files);
    await initializeWorkspace(join(directory, "library"), { library: LIBRARY }, environment);
    await using store = await LocalBuildStore.temporary(environment);
    const compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const releasing = await (await Workspace.open(library, compilers)).package();
    const released = await releasing.build({ outputs: OUTPUTS });
    const release = await releasing.lock(await BuildStore.digest(released.manifest, crypto));

    // build an application against it
    await copyUnit("unit-application", application, files);
    await files.mkdir(join(application, "node_modules", "@destack"), { recursive: true });
    await files.symlink(library, join(application, "node_modules", LIBRARY));
    await initializeWorkspace(
        join(directory, "application"),
        { application: "@destack/build-unit-application-fixture" },
        environment,
    );
    const resolving = await (
        await Workspace.open(application, compilers, {
            resolver: async () => release,
        })
    ).package();
    const built = await resolving.build({ outputs: OUTPUTS });

    // require the builtin, and lock nothing
    const root = await built.digest();
    expect({
        requires: built.manifest.requires,
        lock: await store.lock(await resolving.lock(root)),
    }).toEqual({
        requires: { [LIBRARY]: "2026.9.0" },
        lock: Lock.of(root),
    });
});

test("declare a builtin workspace member in a build's requirements, leaving it out of its lock", async () => {
    // lay out a workspace whose application links its builtin library unit from source
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { library, application } = await layOut(environment);
    await declareBuiltin(library, environment.files);
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());

    // build both units
    const opened = await (await Workspace.open(application, compilers)).package();
    const built = await opened.build({ outputs: OUTPUTS });
    const root = await built.digest();
    expect({
        requires: built.manifest.requires,
        lock: await store.lock(await opened.lock(root)),
    }).toEqual({
        requires: { [LIBRARY]: "2026.9.0" },
        lock: Lock.of(root),
    });
});

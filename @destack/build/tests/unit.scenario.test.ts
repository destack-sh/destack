import { testSimulation } from "../src/test/world.ts";
import { expect, refusal, test } from "@destack/test";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { CompilerPool, BuildTree, Workspace } from "../src/index.ts";
import { BuildImage } from "@destack/package/manifest";
import { present, schema } from "@destack/schema";
import { BuildStore } from "../src/store/index.ts";
import { LocalBuildStore } from "../src/store/local.ts";
import { countObjects, exported, importBuilt, property } from "./fixture.ts";
import { copySource } from "../src/test/source.ts";
import { Toolchain } from "@destack/check/toolchain";
import { COMPILER_VERSION } from "../src/cache/fingerprint.ts";

/** One module output per unit, its dependencies external. */
const OUTPUTS = { bun: { kind: "module", runtime: "bun" } } as const;

/** The integrity a fixture lockfile records for its npm release, which the build requires. */
const FIXTURE_INTEGRITY = "sha512-fixture";

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

/** Make a library declare left-pad, installed in its checkout and locked in the workspace. */
async function installLeftPad(
    workspace: string,
    library: string,
    files: FileSystem,
): Promise<void> {
    // make an image of the release beside the members
    const lockfile = join(workspace, "bun.lock");
    const locked = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(lockfile)));
    const packages = { "left-pad": ["left-pad@1.3.0", "", {}, FIXTURE_INTEGRITY] };
    await files.writeTextFile(lockfile, JSON.stringify({ ...locked, packages }));

    // declare it in the library
    const manifest = join(library, "package.json");
    const authored = schema.looseObject({}).parse(JSON.parse(await files.readTextFile(manifest)));
    const dependencies = { "left-pad": "1.3.0" };
    await files.writeTextFile(manifest, JSON.stringify({ ...authored, dependencies }));

    // install it in the library's node_modules
    const installed = join(library, "node_modules", "left-pad");
    await files.mkdir(installed, { recursive: true });
    const release = { name: "left-pad", version: "1.3.0", main: "index.js" };
    await files.writeTextFile(join(installed, "package.json"), JSON.stringify(release));
    await files.writeTextFile(join(installed, "index.js"), "module.exports = () => '';\n");
}

/** Report whether a built export is the greeting function. */
function isGreeter(value: unknown): value is () => unknown {
    return typeof value === "function";
}

test("pin the compiler's version to the workspace image it compiles", async () => {
    // build the application with its library unit and its npm release, then make an image of them
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { application } = await layOut(environment);
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const opened = await (await Workspace.open(application, compilers)).package();
    const built = await opened.build({ outputs: OUTPUTS });
    const image = await opened.image(await built.digest());

    // pair the version with its image: changed output takes a new COMPILER_VERSION and this digest
    expect({ version: COMPILER_VERSION, image }).toEqual({
        version: 8,
        image: "9264c2bdaf9750e00af249efc702c3215f5c0edef47bb27f5a16c2ad4de956ac",
    });
});

test("build each unit once, rebuilding only an edited library", async () => {
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
        const image = await store.image(await opened.image(await built.digest()));

        return { built, library: unit.manifest, image };
    };

    // build both units and make an image of them
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

    // run each application output with its library from its image placed beside it
    const greetings: unknown[] = [];
    for (const [index, { built, image }] of [first, restored, edited].entries()) {
        const destination = join(directory, `application-${index}`);
        await built.writeDirectory(destination, files);
        const entries = await store.entriesOf(image);
        const tree = await BuildTree.materialize(
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

    // keep one application build under every image
    const libraries = [first, restored, edited].map(({ image }) =>
        BuildImage.resolve(image, image.root, LIBRARY),
    );
    expect({
        units: units.units.map((unit) => unit.name),
        written: unchanged - stored,
        roots: [restored.image.root, edited.image.root],
        libraries,
        greetings,
        imports: first.built.manifest.outputs["bun"]?.exports["."]?.imports,
        requires: first.built.manifest.requires,
    }).toEqual({
        units: [LIBRARY, "@destack/build-unit-application-fixture"],
        written: 0,
        roots: [first.image.root, first.image.root],
        libraries: [first.library, first.library, edited.library],
        greetings: [
            { name: "world", text: "hello world" },
            { name: "world", text: "hello world" },
            { name: "world", text: "hi world" },
        ],
        imports: [LIBRARY],
        requires: { [LIBRARY]: "workspace:*" },
    });
});

test("refuse to cache a unit whose sources changed while it built", async () => {
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
    const image = await store.image(await fresh.image(await built.digest()));
    const dependency = present(BuildImage.resolve(image, image.root, LIBRARY), "the library build");
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

test("build a package against an outside release, refusing it without a resolver", async () => {
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
    const release = await releasing.image(manifest);

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
    const image = await store.image(await resolving.image(await built.digest()));
    expect({
        refused,
        asked,
        requires: built.manifest.requires,
        image,
    }).toEqual({
        refused: [
            "BAD_REQUEST",
            "DEPENDENCY_INVALID",
            `workspace is invalid: it requires Destack packages '${LIBRARY}' outside it and has no release resolver for them`,
        ],
        asked: [LIBRARY],
        requires: { [LIBRARY]: "workspace:*" },
        image: BuildImage.of(await built.digest(), {
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

test("require a builtin release, leaving it out of the image", async () => {
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
    const release = await releasing.image(await BuildStore.digest(released.manifest, crypto));

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

    // require the builtin, and image nothing
    const root = await built.digest();
    expect({
        requires: built.manifest.requires,
        image: await store.image(await resolving.image(root)),
    }).toEqual({
        requires: { [LIBRARY]: "workspace:*" },
        image: BuildImage.of(root),
    });
});

test("require a builtin member, leaving it out of the image", async () => {
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
        image: await store.image(await opened.image(root)),
    }).toEqual({
        requires: { [LIBRARY]: "workspace:*" },
        image: BuildImage.of(root),
    });
});

test("trace each unit's build: a compile on a miss, the cache on a hit", async () => {
    // lay out a workspace, then build its application twice
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { application } = await layOut(environment);
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const build = async () => {
        const opened = await (await Workspace.open(application, compilers)).package();
        await opened.build({ outputs: OUTPUTS });
    };
    await build();
    await build();

    // read how often each of the build's spans ran, a unit's build by its cache answer, and its own counts
    const names = simulation.telemetry.spans
        .filter((span) => span.instrumentationScope.name === "@destack/build")
        .map((span) => {
            const cache = span.attributes["destack.workspace.build.cache"];

            return typeof cache === "string" ? `${span.name}[${cache}]` : span.name;
        });
    const spans = Object.fromEntries(
        [...new Set(names)]
            .toSorted()
            .map((name) => [name, names.filter((each) => each === name).length]),
    );
    const counts = Object.fromEntries(
        Object.entries(await simulation.telemetry.counts()).filter(
            ([key]) =>
                /^destack\.(build|compiler|workspace)\./u.test(key) && !key.endsWith(".seconds"),
        ),
    );

    // expect each unit compiled once by its own started compiler, then taken from the cache
    expect({ spans, counts }).toEqual({
        spans: {
            "destack.build.move": 2,
            "destack.build.write": 2,
            "destack.compiler.build": 2,
            "destack.compiler.compile": 2,
            "destack.compiler.evaluate": 2,
            "destack.compiler.start": 2,
            "destack.dependency.install": 2,
            "destack.evaluate.kinds": 2,
            "destack.workspace.build[hit]": 2,
            "destack.workspace.build[miss]": 2,
            "destack.workspace.open": 2,
        },
        counts: {
            "destack.build.move.duration.count": 2,
            "destack.build.write.duration.count": 2,
            "destack.compiler.build.duration.count": 2,
            "destack.compiler.compile.duration.count": 2,
            "destack.compiler.count": 2,
            "destack.compiler.evaluate.duration.count": 2,
            "destack.compiler.start.duration.count": 2,
            "destack.compiler.wait_time.count": 2,
            "destack.workspace.build.duration.count": 4,
            "destack.workspace.open.duration.count": 2,
        },
    });
});

test("materialize a library's tree once per checkout, linking that checkout's npm packages", async () => {
    // lay out two checkouts of one workspace, each installing the library's npm package
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const first = await layOut(environment);
    const second = await layOut(environment);
    for (const { workspace, library } of [first, second]) {
        await installLeftPad(workspace, library, files);
    }

    // build the library from the first checkout into one store
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const opened = await (await Workspace.open(first.application, compilers)).package();
    const inputs = await opened.storeDependencies({});
    const built = present(inputs[LIBRARY], "the library unit");

    // materialize its tree for each checkout into the store's shared trees
    const linked: string[] = [];
    for (const { workspace } of [first, second]) {
        const tree = await BuildTree.materialize(
            store,
            [built],
            () => undefined,
            store.trees,
            { kind: "compile" },
            { workspace },
        );
        const directory = present(tree.directories[LIBRARY], "the library's directory");
        linked.push(await files.realPath(join(directory, "node_modules", "left-pad")));
    }

    // link each tree to its own checkout's install
    expect(linked).toEqual([
        await files.realPath(join(first.library, "node_modules", "left-pad")),
        await files.realPath(join(second.library, "node_modules", "left-pad")),
    ]);
});

import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import type { Crypto } from "@destack/environment";
import { Digest, present } from "@destack/schema";
import { PackageId } from "../definition/index.ts";
import { MemoryBuild } from "../test/memory.ts";
import { BuildImage, IMAGE_FORMAT } from "./image.ts";
import { type PackageManifest } from "./manifest.ts";
import { ImportMap, type PackageOutput } from "./output.ts";
import type { BuildReader } from "./reader.ts";

/** The build of an app. */
const APP = Digest.parse("a".repeat(64));

/** The build of a package requiring js-tokens 4. */
const ENVIFY = Digest.parse("b".repeat(64));

/** The build of a package requiring js-tokens 10. */
const TOKENIZER = Digest.parse("c".repeat(64));

/** The build of js-tokens 4. */
const TOKENS_4 = Digest.parse("d".repeat(64));

/** The build of js-tokens 10. */
const TOKENS_10 = Digest.parse("e".repeat(64));

test("digest one image for builds in any order, list its root first, and resolve per build", async () => {
    // describe one image in both orders, each package resolving js-tokens to its own version
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("test");
    const forward: BuildImage = {
        formatVersion: IMAGE_FORMAT,
        root: APP,
        builds: {
            [APP]: { "loose-envify": ENVIFY, tokenizer: TOKENIZER },
            [ENVIFY]: { "js-tokens": TOKENS_4 },
            [TOKENIZER]: { "js-tokens": TOKENS_10 },
            [TOKENS_4]: {},
            [TOKENS_10]: {},
        },
    };
    const backward: BuildImage = {
        ...forward,
        builds: Object.fromEntries(Object.entries(forward.builds).toReversed()),
    };

    // serialize both to one digest, list the root first, and resolve js-tokens per package
    expect([
        (await BuildImage.digest(forward, crypto)) === (await BuildImage.digest(backward, crypto)),
        BuildImage.builds(forward),
        BuildImage.resolve(forward, ENVIFY, "js-tokens"),
        BuildImage.resolve(forward, TOKENIZER, "js-tokens"),
    ]).toEqual([true, [APP, ENVIFY, TOKENIZER, TOKENS_4, TOKENS_10], TOKENS_4, TOKENS_10]);
});

test("refuse an image resolving to a build it does not list", () => {
    // refuse an app resolving js-tokens to an unlisted build
    const parsed = BuildImage.safeParse({
        formatVersion: IMAGE_FORMAT,
        root: APP,
        builds: { [APP]: { "js-tokens": TOKENS_4 } },
    });

    expect(parsed.error?.issues.map((issue) => [issue.path, issue.message])).toEqual([
        [["builds", TOKENS_4], `build '${TOKENS_4}' is missing: the image resolves to it`],
    ]);
});

test("layer an image with a cycle, leaving the edge back into it", () => {
    // image an app linking a package and its peer, which links the package back
    const image = BuildImage.of(APP, {
        [APP]: { "loose-envify": ENVIFY },
        [ENVIFY]: { tokenizer: TOKENIZER },
        [TOKENIZER]: { "loose-envify": ENVIFY },
    });

    expect(BuildImage.layers(image)).toEqual([[TOKENIZER], [ENVIFY], [APP]]);
});

/** The package the written builds build, unless one names another. */
const ANSWER: PackageManifest["package"] = {
    id: PackageId.parse("package-00000000-0000-7000-8000-00000000a001"),
    name: "@example/answer",
    version: "2026.9.0",
};

/** The source files of an application's build. */
const SOURCES = {
    "src/answer.ts": "export const answer = 42;\n",
    "src/index.ts": 'export * from "./answer.ts";\n',
};

/** Write a build of some sources with outputs, as an application or another package. */
function written(
    sources: Readonly<Record<string, string>>,
    crypto: Crypto,
    outputs: Readonly<Record<string, PackageOutput>> = {},
    built: PackageManifest["package"] = ANSWER,
    requires: Readonly<Record<string, string>> = {},
): Promise<BuildReader> {
    const files = Object.entries(sources).map(
        ([path, text]) => [path, new TextEncoder().encode(text)] as const,
    );

    return MemoryBuild.write(
        new Map(files),
        { role: "app", package: built, outputs, requires },
        crypto,
    );
}

/** Keep written builds in memory by digest, reading each manifest back. */
function stored() {
    const readers = new Map<Digest, BuildReader>();

    return {
        put: async (reader: BuildReader) => {
            const digest = await reader.digest();
            readers.set(digest, reader);

            return digest;
        },
        read: async (digest: Digest) => present(readers.get(digest), `build ${digest}`).manifest,
    };
}

/** Place a browser export's file by its subpath. */
function browserPath(subpath: string): string {
    return `browser/${subpath.slice(2) || "index"}.js`;
}

/** Describe a build's browser output with its exports' imports and a view. */
function browserOutputs(
    imports: Readonly<Record<string, readonly string[]>>,
    view?: string,
): Readonly<Record<string, PackageOutput>> {
    // export each subpath at a file of its own, mounting a view at one
    const exports = Object.fromEntries(
        Object.entries(imports).map(([subpath, specifiers]) => [
            subpath,
            { path: browserPath(subpath), imports: [...specifiers] },
        ]),
    );
    const views =
        view === undefined
            ? {}
            : {
                  main: {
                      entrypoint: browserPath(view),
                      stylesheets: [],
                      sandbox: {},
                      presents: [],
                      opens: [],
                      categories: [],
                      audience: "person" as const,
                      packages: [],
                      render: { mode: "client" as const },
                  },
              };

    return {
        browser: {
            runtime: "browser",
            emit: true,
            workloads: {},
            views,
            directory: "browser",
            exports,
            peers: {},
            inlined: {},
        },
    };
}

/** Make an image of a root build linking each package's build it runs with, which link none. */
function imageOf(root: Digest, builds: Readonly<Record<string, Digest>>): BuildImage {
    const leaves = Object.values(builds).map((build) => [build, {}] as const);

    return BuildImage.of(root, { ...Object.fromEntries(leaves), [root]: builds });
}

test("map a view's browser modules and its dependencies' below a prefix, each build's imports in its scope", async () => {
    // store a deep library, a library importing it, and an application whose view imports the library
    const { crypto } = testSimulation().environment("test");
    const { put, read } = stored();
    const deep = await put(
        await written({ "src/deep.ts": "export {};\n" }, crypto, browserOutputs({ ".": [] })),
    );
    const middle = await put(
        await written(
            { "src/middle.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": ["@example/deep"], "./static": [] }),
        ),
    );
    const application = await put(
        await written(
            SOURCES,
            crypto,
            browserOutputs({ "./view": ["@example/middle/static", "@example/middle"] }, "./view"),
        ),
    );

    // map the application's imports at the root, the library's in its own scope
    const image = BuildImage.of(application, {
        [application]: { "@example/middle": middle },
        [middle]: { "@example/deep": deep },
        [deep]: {},
    });
    const map = await BuildImage.imports(image, "browser", "/.destack/build/", new Map(), read);

    expect({
        map,
        builds: new Set([application, ...ImportMap.builds(map, "/.destack/build/")]),
    }).toEqual({
        map: {
            imports: {
                "@example/middle": `/.destack/build/${middle}/browser/index.js`,
                "@example/middle/static": `/.destack/build/${middle}/browser/static.js`,
            },
            scopes: {
                [`/.destack/build/${middle}/`]: {
                    "@example/deep": `/.destack/build/${deep}/browser/index.js`,
                },
                [`/.destack/build/${deep}/`]: {},
            },
        },
        builds: new Set([application, middle, deep]),
    });
});

test("map a library again once a builtin imports it, with what its other exports import", async () => {
    // store a library whose stack export imports an extra library, and a builtin importing that export
    const { crypto } = testSimulation().environment("test");
    const { put, read } = stored();
    const extra = await put(
        await written({ "src/extra.ts": "export {};\n" }, crypto, browserOutputs({ ".": [] })),
    );
    const shared = await put(
        await written(
            { "src/shared.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": [], "./stack": ["@example/extra"] }),
        ),
    );
    const view = {
        id: PackageId.parse("package-00000000-0000-7000-8000-00000000a005"),
        name: "@example/view",
        version: "2026.11.0",
    };
    const linked = await put(
        await written(
            { "src/view.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": ["@example/shared/stack"] }),
            view,
        ),
    );

    // reach the library's root from the application's view before the builtin
    const application = await put(
        await written(
            SOURCES,
            crypto,
            browserOutputs({ "./view": ["@example/shared", "@example/view"] }, "./view"),
        ),
    );
    const map = await BuildImage.imports(
        imageOf(application, { "@example/extra": extra, "@example/shared": shared }),
        "browser",
        "/.destack/build/",
        new Map([
            [
                view.name,
                BuildImage.of(linked, {
                    [linked]: { "@example/shared": shared },
                    [shared]: { "@example/extra": extra },
                    [extra]: {},
                }),
            ],
        ]),
        read,
    );

    // map the extra library in the shared library's scope, which the builtin's import needs
    expect(map.scopes).toEqual({
        [`/.destack/build/${shared}/`]: {
            "@example/extra": `/.destack/build/${extra}/browser/index.js`,
        },
        [`/.destack/build/${linked}/`]: {
            "@example/shared": `/.destack/build/${shared}/browser/index.js`,
            "@example/shared/stack": `/.destack/build/${shared}/browser/stack.js`,
        },
        [`/.destack/build/${extra}/`]: {},
    });
});

test("map a builtin at its linked build in every scope, with the builds it imports", async () => {
    // store the linked builtin importing a deep library
    const { crypto } = testSimulation().environment("test");
    const { put, read } = stored();
    const deep = await put(
        await written({ "src/deep.ts": "export {};\n" }, crypto, browserOutputs({ ".": [] })),
    );
    const view = {
        id: PackageId.parse("package-00000000-0000-7000-8000-00000000a005"),
        name: "@example/view",
        version: "2026.11.0",
    };
    const linked = await put(
        await written(
            { "src/view.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": ["@example/deep"] }),
            view,
        ),
    );
    const middle = await put(
        await written(
            { "src/middle.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": ["@example/view"] }),
        ),
    );
    const application = await put(
        await written(
            SOURCES,
            crypto,
            browserOutputs({ "./view": ["@example/middle", "@example/view"] }, "./view"),
        ),
    );

    // map the builtin at the linked build in every scope, and its own imports in its scope
    const map = await BuildImage.imports(
        imageOf(application, { "@example/middle": middle }),
        "browser",
        "/.destack/build/",
        new Map([
            [view.name, BuildImage.of(linked, { [linked]: { "@example/deep": deep }, [deep]: {} })],
        ]),
        read,
    );

    expect({
        map,
        builds: new Set([application, ...ImportMap.builds(map, "/.destack/build/")]),
    }).toEqual({
        map: {
            imports: {
                "@example/middle": `/.destack/build/${middle}/browser/index.js`,
                "@example/view": `/.destack/build/${linked}/browser/index.js`,
            },
            scopes: {
                [`/.destack/build/${middle}/`]: {
                    "@example/view": `/.destack/build/${linked}/browser/index.js`,
                },
                [`/.destack/build/${linked}/`]: {
                    "@example/deep": `/.destack/build/${deep}/browser/index.js`,
                },
                [`/.destack/build/${deep}/`]: {},
            },
        },
        builds: new Set([application, middle, linked, deep]),
    });
});

test("map an npm package at the platform's unit where the range accepts it, else at the image's own", async () => {
    // store an npm unit the application links, and the platform's unit of the same package a builtin links
    const { crypto } = testSimulation().environment("test");
    const { put, read } = stored();
    const tiny = { name: "tiny", version: "1.0.0", integrity: "sha512-tiny" };
    const own = await put(
        await written({ "src/own.ts": "export {};\n" }, crypto, browserOutputs({ ".": [] }), tiny),
    );
    const platform = await put(
        await written(
            { "src/platform.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": [] }),
            tiny,
        ),
    );
    const builtin = {
        id: PackageId.parse("package-00000000-0000-7000-8000-00000000a006"),
        name: "@example/builtin",
        version: "2026.11.0",
    };
    const linked = await put(
        await written(
            { "src/builtin.ts": "export {};\n" },
            crypto,
            browserOutputs({ ".": ["tiny"] }),
            builtin,
        ),
    );
    const application = (range: string) =>
        written(
            SOURCES,
            crypto,
            browserOutputs({ "./view": ["tiny", "@example/builtin"] }, "./view"),
            ANSWER,
            { tiny: range },
        );
    const accepting = await put(await application("^1.0.0"));
    const refusing = await put(await application("^2.0.0"));

    // map each application's npm import: the platform's unit where its range accepts it, else its own
    const builtins = new Map([
        [builtin.name, BuildImage.of(linked, { [linked]: { tiny: platform }, [platform]: {} })],
    ]);
    const mapped = (root: typeof accepting) =>
        BuildImage.imports(
            imageOf(root, { tiny: own }),
            "browser",
            "/.destack/build/",
            builtins,
            read,
        );
    const [accepted, refused] = await Promise.all([mapped(accepting), mapped(refusing)]);

    expect({ accepted: accepted.imports["tiny"], refused: refused.imports["tiny"] }).toEqual({
        accepted: `/.destack/build/${platform}/browser/index.js`,
        refused: `/.destack/build/${own}/browser/index.js`,
    });
});

test("serve the apps a root requires beside its own views, mapping their modules as the root's", async () => {
    // store a library, an app whose view imports it, and a root app requiring that app without importing it
    const { crypto } = testSimulation().environment("test");
    const { put, read } = stored();
    const deep = await put(
        await written({ "src/deep.ts": "export {};\n" }, crypto, browserOutputs({ ".": [] })),
    );
    const shown = present(
        browserOutputs({ "./view": ["@example/deep"] }, "./view")["browser"],
        "the app's browser output",
    );
    const main = present(shown.views["main"], "the app's view");
    const tasks = await put(
        await written(
            { "src/tasks.ts": "export {};\n" },
            crypto,
            { browser: { ...shown, views: { main: { ...main, categories: ["main"] } } } },
            {
                id: PackageId.parse("package-00000000-0000-7000-8000-00000000a002"),
                name: "@example/tasks",
                version: "2026.9.0",
            },
        ),
    );
    const root = await put(
        await written(SOURCES, crypto, browserOutputs({ "./view": [] }, "./view"), ANSWER, {
            "@example/tasks": "latest",
        }),
    );
    const image = BuildImage.of(root, {
        [root]: { "@example/tasks": tasks },
        [tasks]: { "@example/deep": deep },
        [deep]: {},
    });

    // read the apps it requires, the views it serves, and its browser modules
    const apps = await BuildImage.apps(image, read);
    const views = await BuildImage.views(image, read);
    const map = await BuildImage.imports(image, "browser", "/.destack/build/", new Map(), read);

    // expect tasks as its app, its main view served as tasks, and its modules and library mapped
    expect({
        apps,
        views: Object.fromEntries(Object.entries(views).map(([name, { build }]) => [name, build])),
        builds: new Set(ImportMap.builds(map, "/.destack/build/")),
    }).toEqual({
        apps: [{ name: "@example/tasks", digest: tasks }],
        views: { main: root, tasks },
        builds: new Set([tasks, deep]),
    });
});

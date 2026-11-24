import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { type Digest, found } from "@destack/schema";
import { MemoryBuild } from "../test/memory.ts";
import { ImportMap } from "./output.ts";

/** A map of a build's imports at the top, and a dependency's own imports in its scope. */
const MAP: ImportMap = {
    imports: { "@example/view": "/view-1/index.js", "@example/kit/": "/kit/src/" },
    scopes: { "/view-1/": { "@example/view": "/view-2/index.js" } },
};

test("rewrite the mapped specifiers of static, re-exporting and dynamic imports, leaving the others", async () => {
    // rewrite a module of the build and one of the dependency's scope
    const code = [
        'import { view } from "@example/view";',
        'export * from "@example/kit/button.js";',
        'const later = import("@example/view");',
        'import { env } from "cloudflare:workers";',
        'import "./local.js";',
        "const pattern = import(`@example/kit/${name}.js`);",
        'const text = "import x from \\"@example/view\\"";',
    ].join("\n");

    // map the top-level and prefixed specifiers, the dependency's own in its scope
    expect([
        await ImportMap.apply(MAP, code, "/app/index.js"),
        await ImportMap.apply(MAP, 'import "@example/view";', "/view-1/chunk.js"),
    ]).toEqual([
        [
            'import { view } from "/view-1/index.js";',
            'export * from "/kit/src/button.js";',
            'const later = import("/view-1/index.js");',
            'import { env } from "cloudflare:workers";',
            'import "./local.js";',
            "const pattern = import(`@example/kit/${name}.js`);",
            'const text = "import x from \\"@example/view\\"";',
        ].join("\n"),
        'import "/view-2/index.js";',
    ]);
});

test("list the modules a module imports or names by URL, and whether it imports one by a computed name", async () => {
    // import a mapped package, a mapped subpath, a platform module, relative modules and a computed name
    const code = [
        'import { view } from "@example/view";',
        'export * from "@example/kit/button.js";',
        'import { env } from "cloudflare:workers";',
        'import "./local.js";',
        'const later = import("../shared/chunk.js");',
        "const here = import.meta.url;",
        'const worker = new Worker(new URL("./worker.js", import.meta.url));',
    ].join("\n");
    const computed = "const pattern = import(`./languages/${name}.js`);";

    // list each literal import where it resolves, and mark the module with a computed import open
    expect([
        await ImportMap.imports(MAP, code, "/app/src/index.js"),
        await ImportMap.imports(MAP, computed, "/app/src/index.js"),
    ]).toEqual([
        {
            modules: [
                "/view-1/index.js",
                "/kit/src/button.js",
                "/app/src/local.js",
                "/app/shared/chunk.js",
                "/app/src/worker.js",
            ],
            isOpen: false,
        },
        { modules: [], isOpen: true },
    ]);
});

/** Encode a module's code as a build file's bytes. */
function moduleBytes(code: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(code);
}

test("reach the modules an entry imports across builds, a whole build for a computed name, and list the rest unreached", async () => {
    // write an app importing one module of a kit, and a kit whose loader imports by a computed name
    const { crypto } = testSimulation().environment("test");
    const output = {
        runtime: "workerd",
        emit: true,
        workloads: {},
        views: {},
        exports: {},
        peers: {},
    } as const;
    const app = await MemoryBuild.write(
        new Map([
            [
                "output/workerd/main.js",
                moduleBytes('import "@example/kit/used";\nimport "./local.js";'),
            ],
            ["output/workerd/local.js", moduleBytes("export const local = 1;")],
            ["output/workerd/stray.js", moduleBytes("export const stray = 1;")],
        ]),
        {
            role: "app",
            package: { name: "@example/app", version: "1.0.0", integrity: "sha512-app" },
            outputs: { workerd: { ...output, directory: "output/workerd" } },
        },
        crypto,
    );
    const kit = await MemoryBuild.write(
        new Map([
            ["output/workerd/used.js", moduleBytes('import "./chunk.js";')],
            ["output/workerd/chunk.js", moduleBytes("export const chunk = 1;")],
            ["output/workerd/unused.js", moduleBytes("export const unused = 1;")],
            ["output/workerd/unused.js.map", moduleBytes("{}")],
        ]),
        {
            role: "library",
            package: { name: "@example/kit", version: "1.0.0", integrity: "sha512-kit" },
            outputs: { workerd: { ...output, directory: "output/workerd" } },
        },
        crypto,
    );
    const [appDigest, kitDigest] = [await app.digest(), await kit.digest()];
    const contents = async (build: Digest) => (build === appDigest ? app : kit);
    const map = {
        imports: { "@example/kit/used": `/${kitDigest}/output/workerd/used.js` },
        scopes: {},
    };

    // reach the entry, its local module and the kit's used module with its chunk
    const reached = await ImportMap.modules(
        map,
        "/",
        "workerd",
        [`${appDigest}/output/workerd/main.js`],
        contents,
    );
    const unreached = await kit.unused(await kit.files(), new Set(reached.keys()));
    const paths = new Map((await kit.files()).map((file) => [file.digest, file.path]));
    expect({
        reached: [...reached.keys()].toSorted(),
        unreached: [...unreached].map((digest) => found(paths, digest)).toSorted(),
    }).toEqual({
        reached: [
            `${appDigest}/output/workerd/local.js`,
            `${appDigest}/output/workerd/main.js`,
            `${kitDigest}/output/workerd/chunk.js`,
            `${kitDigest}/output/workerd/used.js`,
        ].toSorted(),
        unreached: ["output/workerd/unused.js", "output/workerd/unused.js.map"],
    });

    // reach every module of a build whose module imports by a computed name
    const open = await MemoryBuild.write(
        new Map([
            [
                "output/workerd/lazy.js",
                moduleBytes("export const load = (name) => import(`./${name}.js`);"),
            ],
            ["output/workerd/a.js", moduleBytes("export const a = 1;")],
        ]),
        {
            role: "library",
            package: { name: "@example/open", version: "1.0.0", integrity: "sha512-open" },
            outputs: { workerd: { ...output, directory: "output/workerd" } },
        },
        crypto,
    );
    const openDigest = await open.digest();
    const whole = await ImportMap.modules(
        { imports: {}, scopes: {} },
        "/",
        "workerd",
        [`${openDigest}/output/workerd/lazy.js`],
        async () => open,
    );
    expect([...whole.keys()].toSorted()).toEqual([
        `${openDigest}/output/workerd/a.js`,
        `${openDigest}/output/workerd/lazy.js`,
    ]);
});

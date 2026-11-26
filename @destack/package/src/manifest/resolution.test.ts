import { Digest, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import type { ImportMap } from "./output.ts";
import { ModuleResolution } from "./resolution.ts";

/** The prefixes of the builds' files and modules. */
const PREFIXES = { builds: "/.destack/build/", modules: "/.destack/module/" };

/** The image's root build. */
const ROOT = Digest.parse("a".repeat(64));
/** The library the root imports. */
const LIBRARY = Digest.parse("b".repeat(64));
/** The library's own dependency. */
const DEPENDENCY = Digest.parse("c".repeat(64));
/** A package hoisted to the root's imports, which the library imports. */
const HOISTED = Digest.parse("d".repeat(64));

/** Map the image, the library's dependency at a build. */
function mapOf(dependency: Digest): ImportMap {
    return {
        imports: {
            "@example/library": `/.destack/build/${LIBRARY}/index.js`,
            hoisted: `/.destack/build/${HOISTED}/index.js`,
        },
        scopes: {
            [`/.destack/build/${LIBRARY}/`]: {
                "@example/dependency": `/.destack/build/${dependency}/index.js`,
            },
            [`/.destack/build/${dependency}/`]: {},
            [`/.destack/build/${HOISTED}/`]: {},
        },
    };
}

/** Resolve the image, the library importing its dependency and the hoisted package. */
function resolve(dependency: Digest): Promise<ModuleResolution> {
    const imported = new Map([[LIBRARY, ["@example/dependency", "hoisted"]]]);

    return ModuleResolution.of(
        mapOf(dependency),
        ROOT,
        PREFIXES,
        async (build) => imported.get(build) ?? [],
        testHost().crypto,
    );
}

test("place each build's modules below its digest and resolution, rewriting its own and hoisted imports", async () => {
    // resolve the image and read the library's module URL
    const resolution = await resolve(DEPENDENCY);
    const library = resolution.url(LIBRARY, "index.js");

    // rewrite the library's module, its relative import left as it is
    const code = 'import "@example/dependency";\nimport "hoisted";\nimport "./chunk.js";\n';
    const rewritten = await resolution.rewrite(code, library);

    // expect its imports at the dependency's and the hoisted package's module URLs, and its URL located back
    expect({
        rewritten,
        located: resolution.locate(library),
        stale: resolution.locate(`/.destack/module/${LIBRARY}.${"0".repeat(16)}/index.js`),
    }).toEqual({
        rewritten: `import "${resolution.url(DEPENDENCY, "index.js")}";\nimport "${resolution.url(HOISTED, "index.js")}";\nimport "./chunk.js";\n`,
        located: { build: LIBRARY, path: "index.js" },
        stale: undefined,
    });
});

test("move a module's URL once a build its imports resolve to changes, and only then", async () => {
    // resolve the image twice, the library's dependency at another build the second time
    const first = await resolve(DEPENDENCY);
    const second = await resolve(Digest.parse("e".repeat(64)));

    // expect the library and the root to move, the hoisted package to stay
    expect({
        library: first.url(LIBRARY, "index.js") === second.url(LIBRARY, "index.js"),
        hoisted: first.url(HOISTED, "index.js") === second.url(HOISTED, "index.js"),
    }).toEqual({ library: false, hoisted: true });
});

test("rewrite a page's root module scripts and inline imports, leaving other sources", async () => {
    // rewrite a page loading a root module, a protocol-relative one, a module URL and an inline import
    const resolution = await resolve(DEPENDENCY);
    const module = resolution.url(HOISTED, "index.js");
    const html = [
        '<script type="module" src="/output/browser/view.js"></script>',
        '<script type="module" src="//cdn.example/x.js"></script>',
        `<script type="module" src="${module}"></script>`,
        '<script type="module">import "@example/library";</script>',
    ].join("\n");

    // expect the root module at its module URL, the inline import rewritten, the others as they were
    expect((await resolution.document(html, "")).split("\n")).toEqual([
        `<script type="module" src="${resolution.url(ROOT, "output/browser/view.js")}"></script>`,
        '<script type="module" src="//cdn.example/x.js"></script>',
        `<script type="module" src="${module}"></script>`,
        `<script type="module">import "${resolution.url(LIBRARY, "index.js")}";</script>`,
    ]);
});

test("preload the modules an entry imports statically, leaving dynamic imports", async () => {
    // resolve an entry importing the library statically and a chunk dynamically
    const resolution = await resolve(DEPENDENCY);
    const entry = resolution.url(ROOT, "view.js");
    const library = resolution.url(LIBRARY, "index.js");
    const codes = new Map([
        [entry, 'import "@example/library";\nconst later = import("./later.js");\n'],
        [library, 'import "@example/dependency";\n'],
        [resolution.url(DEPENDENCY, "index.js"), "export {};\n"],
    ]);
    const preloads = await resolution.preloads(entry, async (url) =>
        present(codes.get(url), `the module '${url}'`),
    );

    // expect the library and its dependency in the order reached, the dynamic chunk left out
    expect(preloads).toEqual([library, resolution.url(DEPENDENCY, "index.js")]);
});

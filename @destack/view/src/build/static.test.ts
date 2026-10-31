import { normalize } from "node:path";
import { afterAll, beforeAll, expect, test } from "@destack/test";
import { CompilerPool, type LocalBuildStore } from "@destack/build";
import { PackageCopy, testStore, testSimulation } from "@destack/build/test";
import type { Simulation } from "@destack/environment/simulation";

/** The site fixture, a package declaring one static view. */
const SITE = new URL("../../tests/fixture/site/source/", import.meta.url);

/** The outputs a static view's package ships: the browser's, and Bun's rendering its pages. */
const OUTPUTS = {
    browser: { kind: "module", runtime: "browser" },
    bun: { kind: "module", runtime: "bun" },
} as const;

/** The longest a cold build of the site on its stored closure takes. */
const SITE_BUILD_MILLISECONDS = 60_000;

/** The world the test builds in. */
let simulation: Simulation;
/** The checkout's test store. */
let store: LocalBuildStore;
/** The compilers building into the store. */
let compilers: CompilerPool;
/** The site's copy. */
let source: PackageCopy;

beforeAll(async () => {
    simulation = testSimulation();
    store = await testStore();
    compilers = await CompilerPool.open(store);
    source = await PackageCopy.open(SITE, compilers);
}, SITE_BUILD_MILLISECONDS);

afterAll(async () => {
    await source[Symbol.asyncDispose]();
    await compilers[Symbol.asyncDispose]();
    await store[Symbol.asyncDispose]();
    await simulation[Symbol.asyncDispose]();
});

/** List the module scripts a page loads. */
function scripts(document: string): string[] {
    return [...document.matchAll(/<script type="module"[^>]* src="([^"]+)"/gu)].map(
        (match) => match[1] ?? "",
    );
}

/** Read a module file's name without its directory, hash or extension. */
function moduleName(path: string): string {
    return path.slice(path.lastIndexOf("/") + 1).replace(/(?:-[\w-]+)?\.js$/u, "");
}

/** List the stylesheets a page links. */
function stylesheets(document: string): string[] {
    return [...document.matchAll(/<link rel="stylesheet" href="([^"]+)"/gu)].map(
        (match) => match[1] ?? "",
    );
}

/** List the text of a page's headings. */
function headings(document: string): string[] {
    return [...document.matchAll(/<h1[^>]*>([^<]*)<\/h1>/gu)].map((match) => match[1] ?? "");
}

test(
    "prerender a static view's pages from its declaration, hydrated by its view's chunk",
    { timeout: SITE_BUILD_MILLISECONDS },
    async () => {
        // build the package with no output requested, the view requesting its own
        const build = await source.build({ outputs: OUTPUTS });
        const distributed = new Set((await build.distributed()).map((file) => file.path));
        const read = async (path: string) =>
            new TextDecoder().decode(await build.load(`output/browser/view/site/${path}`));

        // read the view, and each page's headings, stylesheets and the scripts it loads
        const view = build.manifest.outputs["browser"]?.views["site"];
        const pages = await Promise.all(
            ["index.html", "about/index.html", "404.html"].map(async (path) => {
                const page = await read(path);

                return {
                    path,
                    headings: headings(page),
                    stylesheets: stylesheets(page),
                    scripts: scripts(page),
                };
            }),
        );
        const entry = `/${view?.entrypoint ?? ""}`;
        const linked = (view?.stylesheets ?? []).map((path) => `/${path}`);

        // list the browser scripts carrying the site's origin, which only its build reads
        const browserScripts = [...distributed].filter(
            (path) => path.startsWith("output/browser/") && path.endsWith(".js"),
        );
        const carrying = [];
        for (const path of browserScripts) {
            const text = new TextDecoder().decode(await build.load(path));
            if (text.includes("https://example.test")) {
                carrying.push(path);
            }
        }
        expect({
            outputs: Object.keys(build.manifest.outputs).toSorted(),
            render: view?.render,
            isEntryDistributed: view === undefined ? false : distributed.has(view.entrypoint),
            carrying,
            pages,
            sitemap: await read("sitemap.xml"),
            favicon: await read("favicon.svg"),
            logo: (await read("index.html"))
                .match(/<img[^>]* src="([^"]+)"/u)?.[1]
                ?.replace(/-[\w-]+\.png$/u, ".png"),
        }).toEqual({
            outputs: ["browser", "bun"],
            render: { mode: "static" },
            isEntryDistributed: true,
            carrying: [],
            pages: [
                { path: "index.html", headings: ["Home"], stylesheets: linked, scripts: [entry] },
                {
                    path: "about/index.html",
                    headings: ["About"],
                    stylesheets: linked,
                    scripts: [entry],
                },
                { path: "404.html", headings: ["Missing"], stylesheets: linked, scripts: [entry] },
            ],
            sitemap: [
                `<?xml version="1.0" encoding="UTF-8"?>`,
                `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
                `  <url><loc>https://example.test/</loc></url>`,
                `  <url><loc>https://example.test/about/</loc></url>`,
                `</urlset>`,
                ``,
            ].join("\n"),
            favicon: '<svg xmlns="http://www.w3.org/2000/svg"/>\n',
            logo: "/.destack/asset/logo.png",
        });
    },
);

/** The islands fixture, a static view whose counter alone hydrates. */
const ISLANDS = new URL("../../tests/fixture/islands/source/", import.meta.url);

/** List the package's own chunks a module imports statically, and those it loads on demand. */
async function imported(
    read: (path: string) => Promise<string>,
    entry: string,
): Promise<{ readonly loaded: string[]; readonly deferred: string[] }> {
    // follow each relative static import from the entry, collecting the dynamic ones
    const loaded = new Set<string>();
    const deferred = new Set<string>();
    const pending = [entry];
    for (let path = pending.pop(); path !== undefined; path = pending.pop()) {
        if (!loaded.has(path)) {
            loaded.add(path);
            const code = await read(path);
            const directory = path.slice(0, path.lastIndexOf("/") + 1);
            for (const match of code.matchAll(/(?:from|import)\s*"(\.\.?\/[^"]+\.js)"/gu)) {
                pending.push(normalize(`${directory}${match[1] ?? ""}`));
            }
            for (const match of code.matchAll(/import\(\s*"(\.\.?\/[^"]+\.js)"\s*\)/gu)) {
                deferred.add(normalize(`${directory}${match[1] ?? ""}`));
            }
        }
    }

    return { loaded: [...loaded].toSorted(), deferred: [...deferred].toSorted() };
}

test(
    "prerender a browser-only package's static page on a Bun output it does not ship, its island alone loading",
    { timeout: SITE_BUILD_MILLISECONDS },
    async () => {
        // build the islands fixture
        await using islands = await PackageCopy.open(ISLANDS, compilers);
        const build = await islands.build({ outputs: { browser: OUTPUTS.browser } });
        const read = async (path: string) => new TextDecoder().decode(await build.load(path));

        // read the page, what its script loads of the package at once and on demand, and of other packages
        const page = await read("output/browser/view/site/index.html");
        const [script = ""] = scripts(page);
        const entry = script.slice(1);
        const { loaded, deferred } = await imported(read, entry);
        const exported = build.manifest.outputs["browser"]?.exports["./view/site"];
        const loadedCode = await Promise.all(loaded.map(read));

        // render the heading as HTML and the counter as an island root, loading no page code at once
        expect({
            headings: headings(page),
            island: page.match(
                /<div data-island="[^"]+" data-island-id="[^"]+" data-island-properties="[^"]+"/u,
            )?.[0],
            isPageLoaded: loadedCode.some((code) => code.includes("Static heading")),
            deferred: deferred.map((path) => moduleName(path)).filter((name) => name === "counter"),
            imports: exported?.imports,
            prerender: build.manifest.outputs["prerender"]?.emit,
        }).toEqual({
            headings: ["Static heading"],
            island: '<div data-island="/src/islands/counter.tsx" data-island-id="/src/islands/counter.tsx#0" data-island-properties="{&quot;start&quot;:3}"',
            isPageLoaded: false,
            deferred: ["counter"],
            imports: ["@destack/view", "@destack/view/runtime", "@destack/view/static/browser"],
            prerender: false,
        });
    },
);

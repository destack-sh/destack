import { afterAll, beforeAll, expect, test } from "@destack/test";
import { CompilerPool, type LocalBuildStore, Workspace } from "@destack/build";
import { PackageCopy, testStore } from "@destack/build/test";
import { testHost } from "@destack/test/host";
import { DevelopmentServer } from "./server.ts";
import { Toolchain } from "@destack/check/toolchain";

/** The site fixture, a package declaring one static view. */
const SITE = new URL("../../tests/fixture/site/source/", import.meta.url);

/** The checkout's test store. */
let store: LocalBuildStore;
/** The compilers inspecting the site. */
let compilers: CompilerPool;
/** The site's copy. */
let source: PackageCopy;

beforeAll(async () => {
    store = await testStore();
    compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    source = await PackageCopy.open(SITE, compilers);
});

afterAll(async () => {
    await source[Symbol.asyncDispose]();
    await compilers[Symbol.asyncDispose]();
    await store[Symbol.asyncDispose]();
});

/** List the text of a page's headings. */
function headings(document: string): string[] {
    return [...document.matchAll(/<h1[^>]*>([^<]*)<\/h1>/gu)].map((match) => match[1] ?? "");
}

test("serve a static view's pages, endpoints and public files on request, as its prerender writes them", async () => {
    // serve the site from its workspace on a free port
    const workspacePackage = await (await Workspace.open(source.source, compilers)).package();
    const environment = testHost();
    await using development = await DevelopmentServer.start({
        workspace: workspacePackage,
        view: "site",
        environment,
        server: { port: 0 },
    });
    const address = development.vite.httpServer?.address();
    const port = typeof address === "object" && address !== null ? address.port : 0;
    const visit = async (path: string) => {
        const request = new Request(`http://127.0.0.1:${port}${path}`, {
            headers: { accept: "text/html" },
        });
        const response = await environment.network.fetch(request);

        return { status: response.status, text: await response.text() };
    };

    // answer the pages with their headings and the browser entry, the sitemap, a public file and the missing page
    const [home, about, sitemap, favicon, missing] = await Promise.all(
        ["/", "/about/", "/sitemap.xml", "/favicon.svg", "/nowhere"].map(visit),
    );
    expect({
        home: [home?.status, headings(home?.text ?? "")],
        about: [about?.status, headings(about?.text ?? "")],
        entry: home?.text.includes('src="/@id/__x00__virtual:@destack/view/view/site"'),
        sitemap: [sitemap?.status, sitemap?.text],
        favicon: [favicon?.status, favicon?.text],
        missing: [missing?.status, headings(missing?.text ?? "")],
    }).toEqual({
        home: [200, ["Home"]],
        about: [200, ["About"]],
        entry: true,
        sitemap: [
            200,
            [
                `<?xml version="1.0" encoding="UTF-8"?>`,
                `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
                `  <url><loc>https://example.test/</loc></url>`,
                `  <url><loc>https://example.test/about/</loc></url>`,
                `</urlset>`,
                ``,
            ].join("\n"),
        ],
        favicon: [200, '<svg xmlns="http://www.w3.org/2000/svg"/>\n'],
        missing: [404, ["Missing"]],
    });
});

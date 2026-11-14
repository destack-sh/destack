import { join } from "node:path";
import { TestBuildStore } from "@destack/build/test";
import { PackageVersion, Sandbox } from "@destack/package";
import { Lock } from "@destack/package/manifest";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { StaticExport } from "./export.ts";
import { TestViewBuild } from "./test/view.ts";

test("export a static view as a static host serves it: pages with the import map, assets, builds and headers", async () => {
    // store a site's build with its pages, an asset, a redirect and its own headers
    const host = testHost();
    await using store = await TestBuildStore.create(host);
    const encoder = new TextEncoder();
    const site = await TestViewBuild.of(
        PackageVersion.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000d1",
            name: "@acme/site",
            version: "2026.9.0",
        }),
        {
            site: {
                entrypoint: "browser/assets/hydrate.js",
                stylesheets: [],
                sandbox: {},
                presents: [],
                categories: [],
                audience: "person" as const,
                packages: [],
                render: { mode: "static" },
            },
        },
        {
            files: new Map([
                ["browser/view/site/index.html", encoder.encode("<head></head><h1>Home</h1>")],
                ["browser/view/site/blog/index.html", encoder.encode("<h1>Blog</h1>")],
                ["browser/view/site/404.html", encoder.encode("<h1>Not found</h1>")],
                ["browser/view/site/_redirects", encoder.encode("/old /blog/ 301\n")],
                [
                    "browser/view/site/_headers",
                    encoder.encode("/blog/*\n  X-Robots-Tag: noindex\n"),
                ],
                ["browser/asset/font.woff2", encoder.encode("font")],
            ]),
        },
        host.crypto,
    );
    const root = await store.put(site);
    const lock = await store.putLock(Lock.of(root));

    // export it below a fresh directory
    const directory = await host.files.makeTemporaryDirectory("destack-export-");
    try {
        await new StaticExport(directory, store, host).write(lock, "site", new Map());
        const paths = (await host.files.readDirectory(directory, { recursive: true }))
            .filter((entry) => entry.kind === "file")
            .map((entry) => entry.name)
            .toSorted();
        const index = await host.files.readTextFile(join(directory, "index.html"));
        const headers = await host.files.readTextFile(join(directory, "_headers"));

        // expect the pages at the root, the asset below the asset path, every file of the build below its digest
        const built = (path: string) => `.destack/build/${root}/${path}`;
        expect({ paths, isMapped: index.includes(`"importmap"`), headers }).toEqual({
            paths: [
                ".destack/asset/font.woff2",
                built("browser/asset/font.woff2"),
                built("browser/assets/hydrate.js"),
                built("browser/index.js"),
                built("browser/view/site/404.html"),
                built("browser/view/site/_headers"),
                built("browser/view/site/_redirects"),
                built("browser/view/site/blog/index.html"),
                built("browser/view/site/index.html"),
                built("destack.json"),
                built("package.json"),
                built("src/index.ts"),
                "404.html",
                "_headers",
                "_redirects",
                "blog/index.html",
                "browser/asset/font.woff2",
                "browser/assets/hydrate.js",
                "browser/index.js",
                "browser/view/site/404.html",
                "browser/view/site/_headers",
                "browser/view/site/_redirects",
                "browser/view/site/blog/index.html",
                "browser/view/site/index.html",
                "index.html",
            ],
            isMapped: true,
            headers: [
                "/blog/*",
                "  X-Robots-Tag: noindex",
                "/*",
                `  Permissions-Policy: ${Sandbox.policy(Sandbox.merge({}))}`,
                "/.destack/build/*",
                "  Cache-Control: public, max-age=31536000, immutable",
                "/*.html",
                "  Content-Type: text/html",
                "/*.js",
                "  Content-Type: text/javascript",
                "/*.json",
                "  Content-Type: application/json",
                "/*.ts",
                "  Content-Type: application/octet-stream",
                "/*.woff2",
                "  Content-Type: font/woff2",
                "",
            ].join("\n"),
        });
    } finally {
        await host.files.remove(directory, { recursive: true, force: true });
    }
});

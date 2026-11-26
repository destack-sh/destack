import { join } from "node:path";
import { TestBuildStore } from "@destack/build/test";
import { PackageVersion, Sandbox } from "@destack/package";
import { BuildImage } from "@destack/package/manifest";
import { expect, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { MachineError } from "../error/index.ts";
import { StaticExport } from "./export.ts";
import { TestViewBuild } from "./test/view.ts";

test("export a static view as a static host serves it: pages with the import map, assets, builds and headers", async () => {
    // store a site's build with its pages, an asset, a redirect and its own headers
    const host = testHost();
    await using store = await TestBuildStore.create(host);
    const encoder = new TextEncoder();
    const library = await TestViewBuild.of(
        PackageVersion.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000d2",
            name: "@example/type",
            version: "2026.9.0",
        }),
        {},
        { files: new Map([["browser/asset/type.woff2", encoder.encode("type")]]) },
        host.crypto,
    );
    const site = await TestViewBuild.of(
        PackageVersion.parse({
            id: "package-01996ab0-0000-7000-8000-0000000000d1",
            name: "@example/site",
            version: "2026.9.0",
        }),
        {
            site: {
                entrypoint: "browser/assets/hydrate.js",
                stylesheets: [],
                sandbox: {},
                presents: [],
                opens: [],
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
            dependencies: [library],
        },
        host.crypto,
    );
    const typeface = await store.put(library);
    const root = await store.put(site);
    const image = await store.putImage(
        await BuildImage.hoisted(root, { "@example/type": typeface }, (digest) =>
            store.get(digest),
        ),
    );

    // export it below a fresh directory
    const directory = await host.files.makeTemporaryDirectory("destack-export-");
    try {
        await new StaticExport(directory, store, host).write(image, "site", new Map());
        const paths = (await host.files.readDirectory(directory, { recursive: true }))
            .filter((entry) => entry.kind === "file")
            .map((entry) => entry.name)
            .toSorted();
        const index = await host.files.readTextFile(join(directory, "index.html"));
        const headers = await host.files.readTextFile(join(directory, "_headers"));

        // expect the pages at the root, its asset and its library's below the asset path, and every build below its digest
        const built = (path: string, digest?: string) => `.destack/build/${digest ?? root}/${path}`;
        expect({ paths, isMapped: index.includes(`"importmap"`), headers }).toEqual({
            paths: [
                ".destack/asset/font.woff2",
                ".destack/asset/type.woff2",
                built("browser/asset/type.woff2", typeface),
                built("browser/index.js", typeface),
                built("destack.json", typeface),
                built("package.json", typeface),
                built("src/index.ts", typeface),
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
            ].toSorted(),
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

test("refuse exporting a page that refers to a disk path or an unpublished asset", async () => {
    // store two sites: one whose page preloads a font on the build's disk, one naming an asset it lacks
    const host = testHost();
    await using store = await TestBuildStore.create(host);
    const encoder = new TextEncoder();
    const view = {
        entrypoint: "browser/assets/hydrate.js",
        stylesheets: [],
        sandbox: {},
        presents: [],
        opens: [],
        categories: [],
        audience: "person" as const,
        packages: [],
        render: { mode: "static" as const },
    };
    const exported = async (version: string, page: string) => {
        // build the site with the page, then export it into a fresh directory
        const site = await TestViewBuild.of(
            PackageVersion.parse({
                id: "package-01996ab0-0000-7000-8000-0000000000d3",
                name: "@example/site",
                version,
            }),
            { site: view },
            { files: new Map([["browser/view/site/index.html", encoder.encode(page)]]) },
            host.crypto,
        );
        const image = await store.putImage(BuildImage.of(await store.put(site)));
        const directory = await host.files.makeTemporaryDirectory("destack-export-");
        try {
            await new StaticExport(directory, store, host).write(image, "site", new Map());

            return "exported";
        } catch (error) {
            return MachineError.is(error) ? [error.reason, error.message] : error;
        } finally {
            await host.files.remove(directory, { recursive: true, force: true });
        }
    };

    expect({
        // export a page that only writes about such paths in its text
        written: await exported(
            "2026.9.2",
            "<p>Fonts once pointed at file:///home/build and /.destack/asset/type.woff2.</p>",
        ),
        disk: await exported(
            "2026.9.0",
            '<link rel="preload" href="file:///home/build/tree/asset/type.woff2">',
        ),
        missing: await exported(
            "2026.9.1",
            '<link rel="preload" href="/.destack/asset/type.woff2">',
        ),
    }).toEqual({
        written: "exported",
        disk: [
            "SITE_INVALID",
            "static file '/browser/view/site/index.html' is invalid: it refers to 'file:///home/build/tree/asset/type.woff2', which the site does not serve",
        ],
        missing: [
            "SITE_INVALID",
            "static file '/browser/view/site/index.html' is invalid: it refers to '/.destack/asset/type.woff2', which the site does not serve",
        ],
    });
});

import { expect, refusal, test } from "@destack/test";
import { Digest } from "@destack/schema";
import type { PackageFile } from "@destack/package/file";
import { StaticHost } from "./host.ts";

/** Read a site without files from its configuration's texts. */
function parse(redirects: string | undefined, headers: string | undefined) {
    return StaticHost.parse({ directory: "site", files: new Map(), redirects, headers });
}

/** Describe a file of a site's build at a path. */
function file(path: string): PackageFile {
    return { path, digest: Digest.parse("a".repeat(64)), size: 1, mediaType: "text/html" };
}

test("redirect exact, placeholder and splat paths, merging the headers of every matching rule", () => {
    // read the configuration as Cloudflare's files have it, comments and default status included
    const host = StaticHost.parse({
        directory: "site",
        files: new Map(),
        redirects:
            "# installers\n/install https://download.example/install 302\n/old/* /new/:splat 301\n/about /team#people\n/blog/:year/:slug /posts/:year-:slug 308\n",
        headers:
            "/*\n  X-Content-Type-Options: nosniff\n  X-Robots-Tag: noindex\n\n/assets/*\n  Cache-Control: public, max-age=31536000, immutable\n  ! X-Robots-Tag\n\n/docs/:page\n  Link: </docs/:page.md>; rel=alternate\n",
    });

    expect([
        host.redirect("/install"),
        host.redirect("/old/notes/1"),
        host.redirect("/about"),
        host.redirect("/blog/2026/hello"),
        host.redirect("/blog/2026"),
        host.redirect("/team"),
        host.headersOf("/assets/site.js"),
        host.headersOf("/"),
        host.headersOf("/docs/setup"),
    ]).toEqual([
        { location: "https://download.example/install", status: 302 },
        { location: "/new/notes/1", status: 301 },
        { location: "/team#people", status: 302 },
        { location: "/posts/2026-hello", status: 308 },
        undefined,
        undefined,
        {
            "x-content-type-options": "nosniff",
            "cache-control": "public, max-age=31536000, immutable",
        },
        { "x-content-type-options": "nosniff", "x-robots-tag": "noindex" },
        {
            "x-content-type-options": "nosniff",
            "x-robots-tag": "noindex",
            link: "</docs/setup.md>; rel=alternate",
        },
    ]);
});

test("refuse a redirect of an unknown status or two splats, and a header outside a path", async () => {
    expect([
        await refusal(() => parse("/a /b 200\n", undefined)),
        await refusal(() => parse("/a/*/b/* /c\n", undefined)),
        await refusal(() => parse(undefined, "  Cache-Control: no-store\n")),
    ]).toEqual([
        ["UNPROCESSABLE_CONTENT", "SITE_RULE_INVALID", "invalid _redirects line: /a /b 200"],
        ["UNPROCESSABLE_CONTENT", "SITE_RULE_INVALID", "invalid _redirects path: /a/*/b/*"],
        [
            "UNPROCESSABLE_CONTENT",
            "SITE_RULE_INVALID",
            "invalid _headers line:   Cache-Control: no-store",
        ],
    ]);
});

test("resolve a site's paths as Cloudflare's auto-trailing-slash, hiding its host configuration", () => {
    // build a site of an index, a page, a directory's index, an asset and the configuration files
    const paths = [
        "index.html",
        "about.html",
        "docs/index.html",
        "app.js",
        "_redirects",
        "_headers",
        "404.html",
    ];
    const files = new Map(paths.map((path) => [`site/${path}`, file(`site/${path}`)]));
    const site = StaticHost.parse({
        directory: "site",
        files,
        redirects: undefined,
        headers: undefined,
    });
    const answer = (pathname: string) => {
        const resolved = site.resolve(pathname);

        return resolved?.kind === "file"
            ? ["file", resolved.file.path]
            : resolved && ["redirect", resolved.path];
    };

    expect({
        root: answer("/"),
        page: answer("/about"),
        pageFile: answer("/about.html"),
        pageSlash: answer("/about/"),
        directory: answer("/docs"),
        directorySlash: answer("/docs/"),
        directoryIndex: answer("/docs/index.html"),
        asset: answer("/app.js"),
        redirects: answer("/_redirects"),
        headers: answer("/_headers"),
        missing: answer("/nowhere"),
        notFound: site.missing?.path,
    }).toEqual({
        root: ["file", "site/index.html"],
        page: ["file", "site/about.html"],
        pageFile: ["redirect", "/about"],
        pageSlash: ["redirect", "/about"],
        directory: ["redirect", "/docs/"],
        directorySlash: ["file", "site/docs/index.html"],
        directoryIndex: ["redirect", "/docs/"],
        asset: ["file", "site/app.js"],
        redirects: undefined,
        headers: undefined,
        missing: undefined,
        notFound: "site/404.html",
    });
});

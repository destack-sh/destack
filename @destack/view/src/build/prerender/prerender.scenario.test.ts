import { expect, refusal, test } from "@destack/test";
import { planRequests } from "./prerender.ts";

test("plan each page, the not-found page and each endpoint at its file", () => {
    expect(
        planRequests("https://example.test", {
            paths: ["/", "/blog/hello/"],
            notFound: "/404",
            endpoints: ["/sitemap.xml", "/.well-known/security.txt"],
        }),
    ).toEqual([
        { url: "https://example.test/", path: "index.html", kind: "page" },
        { url: "https://example.test/blog/hello/", path: "blog/hello/index.html", kind: "page" },
        { url: "https://example.test/404", path: "404.html", kind: "missing" },
        { url: "https://example.test/sitemap.xml", path: "sitemap.xml", kind: "endpoint" },
        {
            url: "https://example.test/.well-known/security.txt",
            path: ".well-known/security.txt",
            kind: "endpoint",
        },
    ]);
});

test("refuse an uncanonical route and two routes writing one file", async () => {
    const refusals = await Promise.all(
        [
            { paths: ["/notes/../about"], endpoints: [] },
            { paths: ["/about", "/about/"], endpoints: [] },
        ].map((routes) => refusal(() => planRequests("https://example.test", routes))),
    );

    expect(refusals).toEqual([
        [
            "BAD_REQUEST",
            "ROUTE_INVALID",
            "path '/notes/../about' is invalid: it must be a canonical URL path",
        ],
        [
            "BAD_REQUEST",
            "PAGE_INVALID",
            "prerender route set is invalid: the routes produce duplicate files",
        ],
    ]);
});

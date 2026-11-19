import { expect, test } from "@destack/test";
import { BrowserEnvironment } from "@destack/environment/browser";
import { testSimulation } from "@destack/test/world";
import { siteEndpoints } from "./metadata.ts";

test("answer each site file at its path with its media type, the robots naming the sitemap", async () => {
    // answer every endpoint at the site's origin
    await using simulation = testSimulation();
    const environment = BrowserEnvironment.server(simulation.environment("site"));
    const endpoints = siteEndpoints({
        sitemap: ["/"],
        robots: { rules: [{ userAgent: "*", allow: ["/"] }] },
        manifest: { name: "Example", icons: [] },
    });
    const answered = await Promise.all(
        Object.entries(endpoints).map(async ([path, endpoint]) => {
            const request = new Request(`https://example.test${path}`);
            const response = await endpoint({
                request,
                origin: "https://example.test",
                environment,
            });

            return {
                path,
                type: response.headers.get("content-type"),
                text: await response.text(),
            };
        }),
    );

    expect(answered).toEqual([
        {
            path: "/sitemap.xml",
            type: "application/xml; charset=utf-8",
            text: '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>https://example.test/</loc></url>\n</urlset>\n',
        },
        {
            path: "/robots.txt",
            type: "text/plain; charset=utf-8",
            text: "User-agent: *\nAllow: /\n\nSitemap: https://example.test/sitemap.xml\n",
        },
        {
            path: "/manifest.webmanifest",
            type: "application/manifest+json; charset=utf-8",
            text: '{\n    "name": "Example",\n    "start_url": "/",\n    "icons": []\n}\n',
        },
    ]);
});

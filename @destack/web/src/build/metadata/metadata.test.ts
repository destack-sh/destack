import { expect, test } from "@destack/test";
import { writeMetadata } from "./metadata.ts";

test("write each site file at its path with its media type, the robots naming the sitemap", () => {
    const files = writeMetadata(
        "https://example.test",
        {
            sitemap: ["/"],
            robots: { rules: [{ userAgent: "*", allow: ["/"] }] },
            manifest: { name: "Example", icons: [] },
        },
        new Date("2026-10-08T00:00:00Z"),
    );

    expect(files).toEqual([
        {
            path: "/sitemap.xml",
            type: "application/xml",
            text: '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n  <url><loc>https://example.test/</loc></url>\n</urlset>\n',
        },
        {
            path: "/robots.txt",
            type: "text/plain",
            text: "User-agent: *\nAllow: /\n\nSitemap: https://example.test/sitemap.xml\n",
        },
        {
            path: "/manifest.webmanifest",
            type: "application/manifest+json",
            text: '{\n    "name": "Example",\n    "start_url": "/",\n    "icons": []\n}\n',
        },
    ]);
});

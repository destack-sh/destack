import { expect, test } from "@destack/test";
import { writeRobots, writeSitemap } from "./sitemap.ts";

test("list a site's pages at their absolute addresses, escaping XML", () => {
    expect(writeSitemap("https://destack.sh", ["/", "/blog/seeing", "/docs?a=1&b=2"])).toBe(
        [
            '<?xml version="1.0" encoding="UTF-8"?>',
            '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
            "  <url><loc>https://destack.sh/</loc></url>",
            "  <url><loc>https://destack.sh/blog/seeing</loc></url>",
            "  <url><loc>https://destack.sh/docs?a=1&amp;b=2</loc></url>",
            "</urlset>",
            "",
        ].join("\n"),
    );
});

test("write each crawler rule and point crawlers at the sitemap", () => {
    expect(
        writeRobots(
            "https://destack.sh",
            {
                rules: [
                    { userAgent: "*", allow: ["/"], disallow: ["/drafts"] },
                    { userAgent: "GPTBot", disallow: ["/"] },
                ],
            },
            true,
        ),
    ).toBe(
        [
            "User-agent: *",
            "Allow: /",
            "Disallow: /drafts",
            "",
            "User-agent: GPTBot",
            "Disallow: /",
            "",
            "Sitemap: https://destack.sh/sitemap.xml",
            "",
        ].join("\n"),
    );
});

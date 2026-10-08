import { expect, test } from "@destack/test";
import { writeFeed } from "./feed.ts";

test("write an Atom feed dated by its newest article, at absolute addresses, escaping XML", () => {
    expect(
        writeFeed("https://destack.sh", {
            path: "/blog/feed.xml",
            home: "/blog/",
            title: "Destack blog",
            subtitle: "Updates & essays",
            author: { name: "Florian" },
            entries: [
                {
                    title: "Seeing <software>",
                    path: "/blog/seeing/",
                    published: "2026-10-14",
                    summary: "Look at it.",
                },
                {
                    title: "Hello",
                    path: "/blog/hello/",
                    published: "2026-09-01",
                    updated: "2026-09-02T10:00:00Z",
                },
            ],
        }),
    ).toBe(
        [
            '<?xml version="1.0" encoding="utf-8"?>',
            '<feed xmlns="http://www.w3.org/2005/Atom">',
            "  <id>https://destack.sh/blog/</id>",
            "  <title>Destack blog</title>",
            "  <subtitle>Updates &amp; essays</subtitle>",
            '  <link rel="self" href="https://destack.sh/blog/feed.xml"/>',
            '  <link rel="alternate" href="https://destack.sh/blog/"/>',
            "  <updated>2026-10-14T00:00:00.000Z</updated>",
            "  <author>",
            "    <name>Florian</name>",
            "  </author>",
            "  <entry>",
            "    <id>https://destack.sh/blog/seeing/</id>",
            "    <title>Seeing &lt;software&gt;</title>",
            '    <link href="https://destack.sh/blog/seeing/"/>',
            "    <published>2026-10-14T00:00:00.000Z</published>",
            "    <updated>2026-10-14T00:00:00.000Z</updated>",
            "    <summary>Look at it.</summary>",
            "  </entry>",
            "  <entry>",
            "    <id>https://destack.sh/blog/hello/</id>",
            "    <title>Hello</title>",
            '    <link href="https://destack.sh/blog/hello/"/>',
            "    <published>2026-09-01T00:00:00.000Z</published>",
            "    <updated>2026-09-02T10:00:00.000Z</updated>",
            "  </entry>",
            "</feed>",
            "",
        ].join("\n"),
    );
});

test("refuse a feed without articles and an article with an invalid date", () => {
    const feed = {
        path: "/blog/feed.xml",
        home: "/blog/",
        title: "Blog",
        author: { name: "Florian" },
    };

    // name the empty feed and the invalid date
    expect(() => writeFeed("https://destack.sh", { ...feed, entries: [] })).toThrow(
        "the feed /blog/feed.xml has no articles",
    );
    expect(() =>
        writeFeed("https://destack.sh", {
            ...feed,
            entries: [{ title: "A", path: "/a/", published: "someday" }],
        }),
    ).toThrow("invalid feed date: someday");
});

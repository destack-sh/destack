import { FeedOptions } from "./site.ts";
import { ViewError } from "../../error/error.ts";
import { escapeXml } from "./xml.ts";

/** Write an Atom feed of a site's articles at their absolute addresses. */
export function writeFeed(origin: string, options: FeedOptions): string {
    // date the feed by its newest change, refusing a feed without articles
    const updates = options.entries.map((entry) => instant(entry.updated ?? entry.published));
    const newest = updates.toSorted().at(-1);
    if (newest === undefined) {
        throw ViewError.of("FEED_EMPTY", `the feed ${options.path} has no articles`);
    }

    // describe the feed, then each article
    const absolute = (path: string) => escapeXml(new URL(path, origin).href);
    const author = [
        "  <author>",
        `    <name>${escapeXml(options.author.name)}</name>`,
        ...(options.author.email === undefined
            ? []
            : [`    <email>${escapeXml(options.author.email)}</email>`]),
        "  </author>",
    ];
    const entries = options.entries.map((entry, index) =>
        [
            "  <entry>",
            `    <id>${absolute(entry.path)}</id>`,
            `    <title>${escapeXml(entry.title)}</title>`,
            `    <link href="${absolute(entry.path)}"/>`,
            `    <published>${instant(entry.published)}</published>`,
            `    <updated>${updates[index]}</updated>`,
            ...(entry.summary === undefined
                ? []
                : [`    <summary>${escapeXml(entry.summary)}</summary>`]),
            "  </entry>",
        ].join("\n"),
    );

    return [
        '<?xml version="1.0" encoding="utf-8"?>',
        '<feed xmlns="http://www.w3.org/2005/Atom">',
        `  <id>${absolute(options.home)}</id>`,
        `  <title>${escapeXml(options.title)}</title>`,
        ...(options.subtitle === undefined
            ? []
            : [`  <subtitle>${escapeXml(options.subtitle)}</subtitle>`]),
        `  <link rel="self" href="${absolute(options.path)}"/>`,
        `  <link rel="alternate" href="${absolute(options.home)}"/>`,
        `  <updated>${newest}</updated>`,
        ...author,
        ...entries,
        "</feed>",
        "",
    ].join("\n");
}

/** Write a date or date and time as the RFC 3339 instant Atom requires, refusing an invalid one. */
function instant(date: string): string {
    const parsed = new Date(date);
    if (Number.isNaN(parsed.getTime())) {
        throw ViewError.of("FEED_DATE_INVALID", `invalid feed date: ${date}`);
    }

    return parsed.toISOString();
}

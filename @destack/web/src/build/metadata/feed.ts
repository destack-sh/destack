import { schema } from "@destack/schema";
import { escapeXml } from "./xml.ts";

/** A site's Atom feed of articles (RFC 4287), such as its blog's. */
export const FeedOptions = schema.object({
    /** The feed's own path, such as `/blog/feed.xml`. */
    path: schema.string().startsWith("/"),
    /** The page the feed's articles come from, such as `/blog/`. */
    home: schema.string().startsWith("/"),
    /** The feed's title. */
    title: schema.string().min(1),
    /** The feed's one-line description. */
    subtitle: schema.string().exactOptional(),
    /** The author of the articles. */
    author: schema.object({
        /** The author's name. */
        name: schema.string().min(1),
        /** The author's address. */
        email: schema.string().exactOptional(),
    }),
    /** The articles, newest first. */
    entries: schema
        .array(
            schema.object({
                /** The article's title. */
                title: schema.string().min(1),
                /** The article's path. */
                path: schema.string().startsWith("/"),
                /** When the article was published, as an ISO date or date and time. */
                published: schema.string().min(1),
                /** When the article last changed, the publication by default. */
                updated: schema.string().exactOptional(),
                /** The article's one-line summary. */
                summary: schema.string().exactOptional(),
            }),
        )
        .readonly(),
});
/** A site's Atom feed of articles. */
export type FeedOptions = schema.Infer<typeof FeedOptions>;

/** Write an Atom feed of a site's articles at their absolute addresses. */
export function writeFeed(origin: string, options: FeedOptions): string {
    // date the feed by its newest change, refusing a feed without articles
    const updates = options.entries.map((entry) => instant(entry.updated ?? entry.published));
    const newest = updates.toSorted().at(-1);
    if (newest === undefined) {
        throw new Error(`the feed ${options.path} has no articles`);
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
        throw new Error(`invalid feed date: ${date}`);
    }

    return parsed.toISOString();
}

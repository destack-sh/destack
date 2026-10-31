import { Duration, schema } from "@destack/schema";

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

/** A site's guide for language models, in the llms.txt format (llmstxt.org). */
export const LlmsOptions = schema.object({
    /** The site's name. */
    title: schema.string().min(1),
    /** The one-line summary models read first. */
    summary: schema.string().min(1),
    /** The paragraphs about the site below the summary. */
    details: schema.string().exactOptional(),
    /** The sections of links, each to a page's Markdown. */
    sections: schema
        .array(
            schema.object({
                /** The section's title, `Optional` for links a model may skip. */
                title: schema.string().min(1),
                /** The section's links. */
                links: schema
                    .array(
                        schema.object({
                            /** The page's title. */
                            title: schema.string().min(1),
                            /** The path of the page's Markdown. */
                            path: schema.string().startsWith("/"),
                            /** What the page holds. */
                            description: schema.string().exactOptional(),
                        }),
                    )
                    .readonly(),
            }),
        )
        .readonly(),
});
/** A site's guide for language models. */
export type LlmsOptions = schema.Infer<typeof LlmsOptions>;

/** A site's web app manifest, per the W3C Web App Manifest. */
export const ManifestOptions = schema.object({
    /** The app's name. */
    name: schema.string().min(1),
    /** The name where space is short, such as under a home screen icon. */
    shortName: schema.string().exactOptional(),
    /** What the app is for. */
    description: schema.string().exactOptional(),
    /** The page the app opens at, `/` by default. */
    startUrl: schema.string().exactOptional(),
    /** How the app shows when installed. */
    display: schema.enum(["fullscreen", "standalone", "minimal-ui", "browser"]).exactOptional(),
    /** The splash screen's background color. */
    backgroundColor: schema.string().exactOptional(),
    /** The browser interface's color. */
    themeColor: schema.string().exactOptional(),
    /** The app's icons. */
    icons: schema
        .array(
            schema.object({
                /** The icon's address. */
                src: schema.string().min(1),
                /** The sizes, such as `192x192` or `any`. */
                sizes: schema.string().min(1),
                /** The media type, such as `image/png`. */
                type: schema.string().min(1),
                /** Where the icon fits, such as `maskable`. */
                purpose: schema.string().exactOptional(),
            }),
        )
        .readonly(),
});
/** A site's web app manifest. */
export type ManifestOptions = schema.Infer<typeof ManifestOptions>;

/** A site's security contact (RFC 9116), published at `/.well-known/security.txt`. */
export const SecurityOptions = schema.object({
    /** Where to report vulnerabilities, as `mailto:` or `https:` addresses, preferred first. */
    contact: schema.array(schema.string().min(1)).min(1).readonly(),
    /** How long the file stays valid after each build, under a year as RFC 9116 recommends. */
    lifetime: Duration.schema,
    /** The languages reports may use, such as `en`. */
    preferredLanguages: schema.array(schema.string().min(1)).readonly().exactOptional(),
    /** The address of the disclosure policy. */
    policy: schema.string().exactOptional(),
});
/** A site's security contact. */
export type SecurityOptions = schema.Infer<typeof SecurityOptions>;

/** What crawlers may fetch, per the Robots Exclusion Protocol (RFC 9309). */
export const RobotsOptions = schema.object({
    /** The rules, each for the crawlers a user agent names. */
    rules: schema
        .array(
            schema.object({
                /** The crawlers the rule applies to, `*` for every one. */
                userAgent: schema.string().min(1),
                /** The paths they may fetch. */
                allow: schema.array(schema.string()).readonly().exactOptional(),
                /** The paths they may not fetch. */
                disallow: schema.array(schema.string()).readonly().exactOptional(),
            }),
        )
        .readonly(),
});
/** What crawlers may fetch. */
export type RobotsOptions = schema.Infer<typeof RobotsOptions>;

/** The files a site publishes beside its pages for crawlers, readers and tools. */
export const SiteMetadata = schema.object({
    /** The paths sitemap.xml lists. */
    sitemap: schema.array(schema.string().startsWith("/")).readonly().exactOptional(),
    /** The rules robots.txt gives crawlers. */
    robots: RobotsOptions.exactOptional(),
    /** The Atom feed of articles at its path. */
    feed: FeedOptions.exactOptional(),
    /** The site's guide for language models, llms.txt. */
    llms: LlmsOptions.exactOptional(),
    /** The site's web app manifest, manifest.webmanifest. */
    manifest: ManifestOptions.exactOptional(),
    /** The site's security contact, /.well-known/security.txt. */
    security: SecurityOptions.exactOptional(),
});
/** The files a site publishes beside its pages. */
export type SiteMetadata = schema.Infer<typeof SiteMetadata>;

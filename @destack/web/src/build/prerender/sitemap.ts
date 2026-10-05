import { schema } from "@destack/schema";

/** What crawlers may fetch, after Next.js's robots rules. */
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

/** Write the sitemap of a site's pages at their absolute addresses (sitemaps.org 0.9). */
export function writeSitemap(origin: string, routes: readonly string[]): string {
    const entries = routes.map(
        (route) => `  <url><loc>${escape(new URL(route, origin).href)}</loc></url>`,
    );

    return [
        '<?xml version="1.0" encoding="UTF-8"?>',
        '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
        ...entries,
        "</urlset>",
        "",
    ].join("\n");
}

/** Write the rules crawlers read, naming the sitemap when the site has one. */
export function writeRobots(origin: string, options: RobotsOptions, hasSitemap: boolean): string {
    // write each rule's user agent and its paths
    const groups = options.rules.map((rule) =>
        [
            `User-agent: ${rule.userAgent}`,
            ...(rule.allow ?? []).map((path) => `Allow: ${path}`),
            ...(rule.disallow ?? []).map((path) => `Disallow: ${path}`),
        ].join("\n"),
    );

    // point crawlers at the sitemap
    const sitemap = hasSitemap ? [`Sitemap: ${new URL("/sitemap.xml", origin).href}`] : [];

    return `${[...groups, ...sitemap].join("\n\n")}\n`;
}

/** Escape text for an XML element. */
function escape(text: string): string {
    return text
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&apos;");
}

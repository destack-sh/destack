import { schema } from "@destack/schema";
import { FeedOptions, writeFeed } from "./feed.ts";
import { LlmsOptions, writeLlms } from "./llms.ts";
import { ManifestOptions, writeManifest } from "./manifest.ts";
import { SecurityOptions, writeSecurity } from "./security.ts";
import { RobotsOptions, writeRobots, writeSitemap } from "./sitemap.ts";

/** The files a site publishes beside its pages for crawlers, readers and tools. */
export const MetadataOptions = schema.object({
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
export type MetadataOptions = schema.Infer<typeof MetadataOptions>;

/** A file a site publishes at a path of its origin. */
export interface MetadataFile {
    /** The URL path, with a leading slash. */
    readonly path: string;
    /** The media type it is served as. */
    readonly type: string;
    /** The file's text. */
    readonly text: string;
}

/** Write the files a site publishes at its origin, dating the security contact against now. */
export function writeMetadata(site: string, options: MetadataOptions, now: Date): MetadataFile[] {
    // list the pages and the crawler rules naming them
    const files: MetadataFile[] = [];
    if (options.sitemap !== undefined) {
        const text = writeSitemap(site, options.sitemap);
        files.push({ path: "/sitemap.xml", type: "application/xml", text });
    }
    if (options.robots !== undefined) {
        const text = writeRobots(site, options.robots, options.sitemap !== undefined);
        files.push({ path: "/robots.txt", type: "text/plain", text });
    }

    // write the feed, the guide for language models, the manifest and the security contact
    if (options.feed !== undefined) {
        const text = writeFeed(site, options.feed);
        files.push({ path: options.feed.path, type: "application/atom+xml", text });
    }
    if (options.llms !== undefined) {
        files.push({ path: "/llms.txt", type: "text/plain", text: writeLlms(site, options.llms) });
    }
    if (options.manifest !== undefined) {
        const text = writeManifest(options.manifest);
        files.push({ path: "/manifest.webmanifest", type: "application/manifest+json", text });
    }
    if (options.security !== undefined) {
        const text = writeSecurity(site, options.security, now);
        files.push({ path: "/.well-known/security.txt", type: "text/plain", text });
    }

    return files;
}

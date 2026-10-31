import type { Endpoint } from "../../declare/view.ts";
import { writeFeed } from "./feed.ts";
import { writeLlms } from "./llms.ts";
import { writeManifest } from "./manifest.ts";
import { writeSecurity } from "./security.ts";
import type { SiteMetadata } from "./site.ts";
import { writeRobots, writeSitemap } from "./sitemap.ts";

/** Answer the files a site publishes beside its pages, by path, as its pages module's endpoints. */
export function siteEndpoints(metadata: SiteMetadata): Record<string, Endpoint> {
    // list the pages and the crawler rules naming them
    const endpoints: Record<string, Endpoint> = {};
    const { sitemap, robots, feed, llms, manifest, security } = metadata;
    if (sitemap !== undefined) {
        endpoints["/sitemap.xml"] = ({ origin }) =>
            text(writeSitemap(origin, sitemap), "application/xml");
    }
    if (robots !== undefined) {
        const isMapped = sitemap !== undefined;
        endpoints["/robots.txt"] = ({ origin }) =>
            text(writeRobots(origin, robots, isMapped), "text/plain");
    }

    // answer the feed, the guide for language models, the manifest and the security contact, dated by the endpoint's clock
    if (feed !== undefined) {
        endpoints[feed.path] = ({ origin }) =>
            text(writeFeed(origin, feed), "application/atom+xml");
    }
    if (llms !== undefined) {
        endpoints["/llms.txt"] = ({ origin }) => text(writeLlms(origin, llms), "text/plain");
    }
    if (manifest !== undefined) {
        endpoints["/manifest.webmanifest"] = () =>
            text(writeManifest(manifest), "application/manifest+json");
    }
    if (security !== undefined) {
        endpoints["/.well-known/security.txt"] = ({ origin, environment }) =>
            text(writeSecurity(origin, security, new Date(environment.clock.now())), "text/plain");
    }

    return endpoints;
}

/** Answer a text file of a media type. */
function text(body: string, type: string): Response {
    return new Response(body, { headers: { "content-type": `${type}; charset=utf-8` } });
}

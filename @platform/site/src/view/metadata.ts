import type { SiteMetadata } from "@destack/view/static";
import { brand } from "./brand.ts";
import { documents, posts } from "./content/collection.ts";
import { tagline } from "./content/site.ts";

/** The address security reports go to. */
const SECURITY_CONTACT = "mailto:florian@symbol.industries";

/** Write the files the site publishes: its sitemap of pages, crawler rules, feed and more. */
export function metadata(paths: readonly string[]): SiteMetadata {
    return {
        sitemap: paths,
        robots: { rules: [{ userAgent: "*", allow: ["/"] }] },
        feed: {
            path: "/blog/feed.xml",
            home: "/blog/",
            title: "Destack blog",
            subtitle: "Updates around Destack.",
            author: { name: "Florian" },
            entries: posts.map((post) => ({
                title: post.title,
                path: post.route,
                published: post.date,
                summary: post.subtitle,
            })),
        },
        llms: {
            title: "Destack",
            summary: tagline,
            details: "Every page is also published as Markdown: add .md to its address.",
            sections: [
                {
                    title: "Docs",
                    links: documents.map(({ document }) => ({
                        title: document.title,
                        path: document.markdownRoute,
                        description: document.description,
                    })),
                },
                {
                    title: "Blog",
                    links: posts.map((post) => ({
                        title: post.title,
                        path: post.markdownRoute,
                        description: post.subtitle,
                    })),
                },
            ],
        },
        manifest: {
            name: "Destack",
            shortName: "Destack",
            description: tagline,
            display: "standalone",
            backgroundColor: "#0b2029",
            themeColor: "#0b2029",
            icons: [
                { src: brand.icon180, sizes: "180x180", type: "image/png" },
                { src: brand.icon256, sizes: "256x256", type: "image/png" },
                { src: brand.icon512, sizes: "512x512", type: "image/png" },
                { src: brand.favicon, sizes: "any", type: "image/svg+xml" },
            ],
        },
        security: {
            contact: [SECURITY_CONTACT],
            lifetime: { days: 183 },
            preferredLanguages: ["en", "de"],
        },
    };
}

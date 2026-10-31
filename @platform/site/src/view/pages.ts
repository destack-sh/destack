import type { Endpoint } from "@destack/view/declare";
import { siteEndpoints } from "@destack/view/static";
import { documents, pageSources, posts, searchEntries } from "./content/collection.ts";
import { release } from "./content/generated/release.ts";
import { plainTextFor } from "./content/markdown/text.ts";
import { metadata } from "./metadata.ts";

/** Every page: the home, pricing, the blog and each post, and each documentation page. */
export const paths = [
    "/",
    "/pricing/",
    "/blog/",
    ...posts.map((post) => post.route),
    ...documents.map((each) => each.document.route),
];

/** The path rendering the not-found page. */
export const notFound = "/404";

/** The site's files: its metadata, search index, release, and each page's Markdown and text. */
export const endpoints: Readonly<Record<string, Endpoint>> = {
    ...siteEndpoints(metadata(paths)),
    "/search.json": () => Response.json(searchEntries),
    "/release.json": () => Response.json(release),
    ...Object.fromEntries(
        pageSources.flatMap((source): [string, Endpoint][] => [
            [source.markdownRoute, () => text(source.markdown, "text/markdown")],
            [source.textRoute, () => text(plainTextFor(source.markdown), "text/plain")],
        ]),
    ),
};

/** Answer a page source in a text format. */
function text(source: string, type: string): Response {
    return new Response(`${source.trim()}\n`, {
        headers: { "content-type": `${type}; charset=utf-8` },
    });
}

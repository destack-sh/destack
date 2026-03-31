import { formatDocsSlug } from "../../shared/slug.ts";

/** Render one docs page route string. */
export function renderDocsPageRoute(slug: string) {
    const docsSlug = formatDocsSlug(slug);

    return `/docs/${docsSlug}`;
}

/** Format one docs slug string. */
export function formatDocsSlug(slug: string) {
    return slug.replace(/\s+/g, "-").toLowerCase();
}

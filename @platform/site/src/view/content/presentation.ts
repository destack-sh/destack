import { schema } from "@destack/schema";

/** One navigable item, shared by generated indexes and archives. */
export const ContentEntry = schema.object({
    /** The visible title. */
    title: schema.string(),
    /** The destination. */
    href: schema.string(),
    /** The one-line summary. */
    summary: schema.string().exactOptional(),
    /** The publication date, as an ISO date. */
    date: schema.string().exactOptional(),
    /** The cover image, or null for an entry without one. */
    image: schema
        .object({ source: schema.string(), alt: schema.string() })
        .nullable()
        .exactOptional(),
});
/** One linked entry of a content list. */
export type ContentEntry = schema.Infer<typeof ContentEntry>;

/** Display publication dates consistently without shifting calendar days by time zone. */
export function formatDate(date: string): string {
    return new Intl.DateTimeFormat("en-GB", {
        day: "numeric",
        month: "long",
        year: "numeric",
        timeZone: "UTC",
    }).format(new Date(date));
}

/** Estimate reading time at roughly 300 tokens (200 words) per minute. */
export function formatReadTime(tokenCount: number) {
    const minutes = Math.max(1, Math.ceil(tokenCount / 300));

    return `${minutes} min`;
}

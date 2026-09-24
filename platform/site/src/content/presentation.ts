import { escapeAttribute } from "./html.ts";

/** One navigable item, shared by generated indexes, archives, and references. */
export type ContentEntry = {
    title: string;
    href: string;
    // oxlint-disable-next-line destack/no-sludge -- summary is the entry field in the generated content JSON
    summary?: string;
    meta?: string;
    date?: string;
    external?: boolean;
    code?: boolean;
    image?: { source: string; alt: string } | null;
};

/** Display publication dates consistently without shifting calendar days by time zone. */
export function formatDate(date: string): string {
    return new Intl.DateTimeFormat("en-GB", {
        day: "numeric",
        month: "long",
        year: "numeric",
        timeZone: "UTC",
    }).format(new Date(date));
}

/** Render the same semantic entry structure in generated and interactive pages. */
export function renderContentList(entries: readonly ContentEntry[]): string {
    return `<ul class="content-list">${entries
        .map((entry) => {
            // mark external links to open in a new tab
            const relation = entry.external ? ' target="_blank" rel="noopener noreferrer"' : "";
            // omit placeholder descriptions that repeat the link title
            const isRepeatedTitle = entry.summary?.replace(/\.$/, "") === entry.title;
            const description =
                entry.summary && !isRepeatedTitle
                    ? `<span class="content-entry-summary">${escapeAttribute(entry.summary)}</span>`
                    : "";
            const meta = entry.date
                ? `<time class="content-entry-meta" datetime="${escapeAttribute(entry.date)}">${escapeAttribute(
                      entry.meta ?? formatDate(entry.date),
                  )}</time>`
                : entry.meta
                  ? `<span class="content-entry-meta">${escapeAttribute(entry.meta)}</span>`
                  : "";
            const titleTag = entry.code ? "code" : "span";
            const image = entry.image
                ? `<img class="content-entry-image" src="${escapeAttribute(entry.image.source)}" alt="${escapeAttribute(
                      entry.image.alt,
                  )}" loading="lazy">`
                : "";
            const arrow = `<svg class="content-arrow" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M5 12h14m-6-6 6 6-6 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>`;

            return `<li><a class="content-entry"${entry.date ? ' data-dated="true"' : ""}${
                entry.external ? ' data-external="true"' : ""
            } href="${escapeAttribute(
                entry.href,
            )}"${relation}>${image}<${titleTag} class="content-entry-title">${escapeAttribute(
                entry.title,
            )}</${titleTag}>${meta}${arrow}${description}</a></li>`;
        })
        .join("")}</ul>`;
}

/** Estimate reading time at roughly 300 tokens (200 words) per minute. */
export function formatReadTime(tokenCount: number) {
    const minutes = Math.max(1, Math.ceil(tokenCount / 300));

    return `${minutes} min`;
}

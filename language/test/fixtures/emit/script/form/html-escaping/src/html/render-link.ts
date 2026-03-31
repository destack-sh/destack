import { escapeAttr, quoteAttr } from "./escape-attr.ts";
import { escapeHtml } from "./escape-html.ts";

/** Render one escaped docs link. */
export function renderLink(href: string, label: string): string {
    const escapedHref = escapeAttr(href);
    const escapedLabel = escapeHtml(label);
    const hrefAttribute = quoteAttr("href", escapedHref);

    return `<a ${hrefAttribute}>${escapedLabel}</a>`;
}

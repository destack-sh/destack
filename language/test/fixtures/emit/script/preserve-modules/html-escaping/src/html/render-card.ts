import { quoteAttr } from "./escape-attr.ts";
import { escapeHtml } from "./escape-html.ts";
import { renderLink } from "./render-link.ts";

/** Render one escaped docs callout card. */
export function renderCalloutCard(
    title: string,
    summary: string,
    href: string,
) {
    // build the escaped card pieces
    const className = quoteAttr("class", "docs-callout");
    const heading = `<h2>${escapeHtml(title)}</h2>`;
    const body = `<p>${escapeHtml(summary)}</p>`;
    const footer = renderLink(href, "Read guide");

    return `<section ${className}>${heading}${body}${footer}</section>`;
}

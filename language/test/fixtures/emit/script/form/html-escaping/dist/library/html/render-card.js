import { quoteAttr } from "./escape-attr.js";
import { escapeHtml } from "./escape-html.js";
import { renderLink } from "./render-link.js";
export function renderCalloutCard(title, summary, href) {
    const className = quoteAttr("class", "docs-callout");
    const heading = `<h2>${escapeHtml(title)}</h2>`;
    const body = `<p>${escapeHtml(summary)}</p>`;
    const footer = renderLink(href, "Read guide");
    return `<section ${className}>${heading}${body}${footer}</section>`;
}
//# sourceMappingURL=./render-card.map

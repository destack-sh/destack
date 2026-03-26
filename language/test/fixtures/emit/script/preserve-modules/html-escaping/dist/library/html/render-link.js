import { escapeAttr, quoteAttr } from "./escape-attr.js";
import { escapeHtml } from "./escape-html.js";
export function renderLink(href, label) {
    const escapedHref = escapeAttr(href);
    const escapedLabel = escapeHtml(label);
    const hrefAttribute = quoteAttr("href", escapedHref);
    return `<a ${hrefAttribute}>${escapedLabel}</a>`;
}
//# sourceMappingURL=./render-link.map

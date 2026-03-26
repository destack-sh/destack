import { escapeAttr, quoteAttr } from "./html/escape-attr.js";
import { escapeHtml } from "./html/escape-html.js";
import { renderCalloutCard } from "./html/render-card.js";
import { renderLink } from "./html/render-link.js";
export const docsPreview = {
    escapedBodyHtml: escapeHtml("<em>destack</em>"),
    escapedTooltipLabel: escapeAttr(`quote"and'ampersand&`),
    quotedTooltipAttribute: quoteAttr("data-title", `quote"and'ampersand&`),
    renderedGuideLink: renderLink("/docs?chapter=emit", `Emit "Guide"`),
    renderedGuideCard: renderCalloutCard(
        "Emit Guide",
        "Preserve authored html helpers in library output.",
        "/docs?chapter=emit",
    ),
};
//# sourceMappingURL=./app.map

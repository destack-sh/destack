import { escapeAttr, quoteAttr } from "./html/escape-attr.ts";
import { escapeHtml } from "./html/escape-html.ts";
import { renderCalloutCard } from "./html/render-card.ts";
import { renderLink } from "./html/render-link.ts";

/** The escaped docs preview state for the preserve modules entry. */
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

import { escapeAttribute } from "./html.ts";

/** Captions and accessible labels for a highlighted code listing. */
export type ListingOptions = {
    title?: string;
    href?: string;
    detail?: string;
    label?: string;
};

/** Render highlighted code with shared captions, line numbers, and scrolling. */
export function renderListing(
    highlighted: string,
    { title, href, detail, label = "Code" }: ListingOptions = {},
): string {
    // preserve empty lines without including gutters in copied code
    const rows = highlighted
        .split("\n")
        .map(
            (line, index) =>
                `<span class="markdown-code-line" data-publication-line><span class="markdown-code-gutter" data-publication-gutter aria-hidden="true">${
                    index + 1
                }</span><span class="markdown-code-text" data-publication-code>${line || " "}</span></span>`,
        )
        .join("");

    return `<figure class="markdown-code" data-publication-listing>${renderListingCaption({
        title,
        href,
        detail,
    })}<pre data-publication-body tabindex="0" aria-label="${escapeAttribute(
        title ?? label,
    )}"><code class="markdown-code-lines" data-publication-lines>${rows}</code></pre></figure>`;
}

/** Render a diagram with the shared listing frame and caption. */
export function renderDiagram(
    source: string,
    { title, label = "Diagram" }: ListingOptions = {},
): string {
    return `<figure class="markdown-code markdown-mermaid" data-publication-listing>${renderListingCaption(
        { title },
    )}<div data-publication-body data-mermaid tabindex="0" role="region" aria-label="${escapeAttribute(
        title ?? label,
    )}"><pre><code>${escapeAttribute(source)}</code></pre></div></figure>`;
}

/** Render an authored caption or linked source location. */
function renderListingCaption({ title, href, detail }: ListingOptions): string {
    // escape the caption title and link it to its source
    const name = title == undefined ? "" : escapeAttribute(title);
    const heading =
        href == undefined
            ? name
            : `<a href="${escapeAttribute(
                  href,
              )}" rel="external noopener noreferrer" target="_blank">${name}</a>`;
    const caption =
        title == undefined
            ? ""
            : `<figcaption data-publication-caption><span data-publication-caption-title>${heading}</span>${
                  detail == undefined
                      ? ""
                      : `<span data-publication-caption-detail>${escapeAttribute(detail)}</span>`
              }</figcaption>`;

    return caption;
}

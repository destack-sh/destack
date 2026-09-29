import { marked } from "marked";
import { headingsFor, searchSectionsFor } from "./markdown.ts";
import { plainTextFor, tokenEstimateFor } from "./text.ts";

/// A generated section composed into authored Markdown and HTML.
export type DocumentSection = { title: string; markdown: string; html: string; children?: DocumentSection[]; };

/// Append named sections and derive every document outline from the composed Markdown.
export function appendDocumentSections<T extends { markdown: string; html: string; }>(document: T, sections: DocumentSection[]) {
    let markdown = document.markdown.trimEnd();
    const html = document.html + sections.map((section) => renderSection(section, 2)).join("");

    // index authored and generated content together
    markdown += "\n";
    const headings = headingsFor(markdown);
    const searchSections = searchSectionsFor(markdown);

    return {
        ...document,
        html,
        markdown,
        headings,
        searchSections,
        searchText: searchSections.map((section) => section.text).join(" "),
        tableOfContents: headings.filter((heading) => heading.depth > 1),
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };

    /// Derive heading depth from nesting and identifiers from the full document.
    function renderSection(section: DocumentSection, depth: number): string {
        markdown += `\n\n${"#".repeat(depth)} ${section.title}`;
        const heading = headingsFor(markdown).at(-1);
        markdown += `\n\n${section.markdown}`;

        // render the same section tree in HTML
        const children = (section.children ?? []).map((child) => renderSection(child, depth + 1)).join("");
        const title = marked.parseInline(section.title);

        return `<section class="document-section"><h${depth} id="${heading!.id}">${title}</h${depth}>${section.html}${children}</section>`;
    }
}

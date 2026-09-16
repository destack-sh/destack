import { mapDirectives } from "./directives.ts";

/// Normalize Markdown into compact full-text search content.
export function searchTextFor(markdown: string) {
    return mapDirectives(
        withoutComments(markdown),
        (_name, attributes, body) =>
            `${attributes.title ?? attributes.alt ?? ""}\n${attributes.caption ?? body}`,
    )
        .replace(/```[\s\S]*?```/g, (block) => block.replace(/^```[^\n]*|```$/g, ""))
        .replace(/<[^>]+>/g, " ")
        .replace(/\[([^\]]+)]\([^)]+\)/g, "$1")
        .replace(/[`*_#>|~-]/g, " ")
        .replace(/\s+/g, " ")
        .trim();
}

/// Convert Markdown into readable text while preserving code exactly.
export function plainTextFor(markdown: string) {
    const code: string[] = [];
    const protectedMarkdown = withoutComments(markdown)
        .replace(/^```[^\n]*\n([\s\S]*?)^```\s*$/gm, (_, source) => protect(source.trimEnd(), code))
        .replace(/`([^`\n]+)`/g, (_, source) => protect(source, code));
    const text = mapDirectives(protectedMarkdown, (name, attributes, body) => {
        if (name === "figure" || name === "video") {
            const title = attributes.title ?? attributes.alt ?? "";
            const source = protect(attributes.src, code);

            return `${title} (${source})\n${attributes.caption ?? body}`;
        }

        return body;
    })
        .replace(/^```[^\n]*\n/gm, "")
        .replace(/^```\s*$/gm, "")
        .replace(/^:::\w+[^\n]*$/gm, "")
        .replace(/^:::\s*$/gm, "")
        .replace(/^#{1,6}\s+/gm, "")
        .replace(/^>\s?/gm, "")
        .replace(/!\[([^\]]*)]\([^)]+\)/g, "$1")
        .replace(/\[([^\]]+)]\([^)]+\)/g, "$1")
        .replace(/<[^>]+>/g, "")
        .replace(/[*_`~]/g, "")
        .replace(/\n{3,}/g, "\n\n")
        .trim();

    return text.replace(/\u0000(\d+)\u0000/g, (_, index) => code[Number(index)]);
}

/// Remove HTML comments from rendered text.
function withoutComments(markdown: string) {
    return markdown.replace(/```[\s\S]*?```|`[^`\n]*`|<!--[\s\S]*?-->/g, (source) =>
        source.startsWith("<!--") ? "" : source,
    );
}

/// Protect code during prose cleanup.
function protect(source: string, code: string[]) {
    const index = code.length;
    code.push(source);

    return `\u0000${index}\u0000`;
}

/// Estimate language-model tokens from plain-text length.
export function tokenEstimateFor(text: string) {
    return Math.ceil(text.length / 4);
}

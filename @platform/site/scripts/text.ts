/** An image with its description, source, and the attributes braced after it. */
const IMAGE = /!\[([^\]]*)\]\(([^)\s]+)[^)]*\)(?:\{[^}\n]*\})?/gu;

/** Normalize Markdown into compact full-text search content. */
export function searchTextFor(markdown: string) {
    return withoutComments(markdown)
        .replace(IMAGE, "$1")
        .replace(/```[\s\S]*?```/gu, (block) => block.replace(/^```[^\n]*|```$/gu, ""))
        .replace(/<[^>]+>/gu, " ")
        .replace(/\[([^\]]+)\]\([^)]+\)/gu, "$1")
        .replace(/[`*_#>|~-]/gu, " ")
        .replace(/\s+/gu, " ")
        .trim();
}

/** Convert Markdown into readable text while preserving code exactly. */
export function plainTextFor(markdown: string) {
    // protect code spans and fences from the prose cleanup
    const code: string[] = [];
    const protectedMarkdown = withoutComments(markdown)
        .replace(/^```[^\n]*\n([\s\S]*?)^```\s*$/gmu, (_match: string, source: string) =>
            protect(source.trimEnd(), code),
        )
        .replace(/`([^`\n]+)`/gu, (_match: string, source: string) => protect(source, code));
    const text = protectedMarkdown
        .replace(
            IMAGE,
            (_match: string, alt: string, source: string) => `${alt} (${protect(source, code)})`,
        )
        .replace(/^```[^\n]*\n/gmu, "")
        .replace(/^```\s*$/gmu, "")
        .replace(/^#{1,6}\s+/gmu, "")
        .replace(/^>\s?/gmu, "")
        .replace(/\[([^\]]+)\]\([^)]+\)/gu, "$1")
        .replace(/<[^>]+>/gu, "")
        .replace(/[*_`~]/gu, "")
        .replace(/\n{3,}/gu, "\n\n")
        .trim();

    return text.replace(/\u0000(\d+)\u0000/gu, (_match: string, index: string) => {
        const source = code[Number(index)];
        if (source == undefined) {
            throw new Error(`missing protected code ${index}`);
        }

        return source;
    });
}

/** Remove HTML comments from rendered text. */
function withoutComments(markdown: string) {
    return markdown.replace(/```[\s\S]*?```|`[^`\n]*`|<!--[\s\S]*?-->/gu, (source) =>
        source.startsWith("<!--") ? "" : source,
    );
}

/** Protect code during prose cleanup. */
function protect(source: string, code: string[]) {
    const index = code.length;
    code.push(source);

    return `\u0000${index}\u0000`;
}

/** Estimate language-model tokens from plain-text length. */
export function tokenEstimateFor(text: string) {
    return Math.ceil(text.length / 4);
}

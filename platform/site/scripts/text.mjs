/// Normalize Markdown into compact full-text search content.
export function searchTextFor(markdown) {
    return markdown
        .replace(/```[\s\S]*?```/g, (block) => block.replace(/^```[^\n]*|```$/g, ""))
        .replace(/<[^>]+>/g, " ")
        .replace(/\[([^\]]+)]\([^)]+\)/g, "$1")
        .replace(/[`*_#>|~-]/g, " ")
        .replace(/\s+/g, " ")
        .trim();
}

/// Convert Markdown into readable text while preserving code exactly.
export function plainTextFor(markdown) {
    const code = [];
    const protectedMarkdown = markdown
        .replace(/^```[^\n]*\n([\s\S]*?)^```\s*$/gm, (_, source) => protect(source.trimEnd(), code))
        .replace(/`([^`\n]+)`/g, (_, source) => protect(source, code));
    const text = protectedMarkdown
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

/// Replace code with a stable placeholder during prose cleanup.
function protect(source, code) {
    const index = code.length;
    code.push(source);

    return `\u0000${index}\u0000`;
}

/// Estimate language-model tokens from plain-text length.
export function tokenEstimateFor(text) {
    return Math.ceil(text.length / 4);
}

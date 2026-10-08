import { mapDirectives } from "./directives.ts";

/** Normalize Markdown into compact full-text search content. */
export function searchTextFor(markdown: string) {
    return mapDirectives(
        withoutComments(markdown),
        (_name, attributes, body) =>
            `${attributes["title"] ?? attributes["alt"] ?? ""}\n${attributes["caption"] ?? body}`,
    )
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
    const text = mapDirectives(protectedMarkdown, (name, attributes, body) => {
        if (name === "figure" || name === "video") {
            const title = attributes["title"] ?? attributes["alt"] ?? "";
            const source = attributes["src"];
            if (source == undefined) {
                throw new Error(`missing src in ${name} directive`);
            }

            return `${title} (${protect(source, code)})\n${attributes["caption"] ?? body}`;
        }

        return body;
    })
        .replace(/^```[^\n]*\n/gmu, "")
        .replace(/^```\s*$/gmu, "")
        .replace(/^:::\w+[^\n]*$/gmu, "")
        .replace(/^:::\s*$/gmu, "")
        .replace(/^#{1,6}\s+/gmu, "")
        .replace(/^>\s?/gmu, "")
        .replace(/!\[([^\]]*)\]\([^)]+\)/gu, "$1")
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

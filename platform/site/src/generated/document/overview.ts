import type { DocumentContent } from "../documents";

const content = {
    html: resolveAssets("<h1 id=\"overview\">Overview</h1><p>Destack is an absurdly integrated universal computing stack, beginning with its own programming language (affectionately &quot;TypeScript++&quot;) and runtime (<code>.ds</code>) - basically a superset of &quot;strict modern&quot; TypeScript.</p>\n", {  }),
} satisfies DocumentContent;

export default content;

/// Replace build-time asset placeholders with bundled URLs.
function resolveAssets(html: string, assets: Record<string, string>) {
    let resolved = html;

    for (const [placeholder, asset] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, asset);
    }

    return resolved;
}

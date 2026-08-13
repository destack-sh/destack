import type { DocumentContent } from "../documents";

const content = {
    html: resolveAssets("<h1 id=\"runtime\">Runtime</h1><p>The runtime is where Destack (<code>.ds</code>) code actually <em>runs</em>, and it&#39;s the only place where &quot;pure computation&quot; touches the real world via our well defined <code>@binding</code>s.\nThis is nice because it means we get to capture and analyze all effects through a relatively thin well known boundary, which enables great observability and debugging.\nAnd because Destack is a fully integrated stack, the runtime has been co-designed as part of the entire language toolchain, and the standard library takes advantage of this.</p>\n", {  }),
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

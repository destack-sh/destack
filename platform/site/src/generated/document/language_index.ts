import type { DocumentContent } from "../documents";

const content = {
    html: resolveAssets("<h1 id=\"language\">Language</h1><p>Our <code>.ds</code> (&quot;TypeScript++&quot;) is a superset of a &quot;strict modern&quot; subset of <code>.ts</code> (TypeScript), similar <em>in spirit</em> to familiar ecosystem extensions like <code>.svelte</code> or <code>.vue</code>.\nGenerally, existing TypeScript and TSX <em>just works</em> <strong>if</strong> it is sound <em>and</em> uses no exceptions.\nFortunately, strict TypeScript is already a best practice - it&#39;s what you get when enabling the recommended soundness flags in TSC (mostly) - and converting implicit exceptions to explicit results is a trivial (and worthwhile) one-shot transformation.</p>\n<p>There are solid arguments that a language should be minimal like Zig or Go or even C, though programmers 50 years ago would not have called them &quot;minimal&quot; by any stretch.\nUltimately, we do not believe &quot;language minimalism&quot; to be pragmatic for the universal language and toolchain we want: Destack aims to be a <em>complete</em> (and coherent and pragmatic) language, not a <em>minimal</em> language.\nAnd since we needed <em>some</em> additions anyway, we took the opportunity to round out the language with modern ergonomics like patterns, operator overloading, reflection, and comptime.</p>\n", {  }),
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

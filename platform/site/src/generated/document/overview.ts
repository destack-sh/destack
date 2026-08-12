import type { DocumentContent } from "../documents";

const content = {
    html: resolveAssets("<h1 id=\"overview\">Overview</h1><p>The Destack language (<code>.ds</code>) is a superset of &quot;strict modern&quot; TypeScript with support for <code>.ts</code> and <code>.tsx</code> files, true native AOT compilation and a fully integrated toolchain, <em>and</em> it can also &quot;compile&quot; nicely to standard JS/TS targets.\nStrict TypeScript code &quot;just works&quot;, but Destack has absolutely <strong>no JavaScript or NPM interoperability</strong>.</p>\n<p>We believe that the ideal way to build correct, optimal, integrated software systems is to build a fully integrated computing stack, and thus by &quot;language&quot; (&quot;TypeScript++&quot;) we mean much more than &quot;just&quot; the programming language itself: a language, a runtime, a toolchain, plugins, libraries, and ultimately, a way of programming.\nIt&#39;s all connected, and to leave out a part would be to betray the whole, which is why we need to begin with an <em>actual</em> programming language.</p>\n<h2 id=\"universality-and-completeness\">Universality and Completeness</h2><p>We&#39;re very early in software, and we&#39;re still figuring out how to build optimal, correct, and integrated software systems.\nOver 50 years, we have grown more and more layers of software sediment and need ever <em>more</em> tools to get any code out the door, and yet confidence and performance have plummeted.\nWe can do better, but not by adding <em>more</em> and more inscrutable pieces.</p>\n<p>The best possible stack would be fully integrated across the language itself, the toolchain, the runtime, and basically anything that touches the software stack.\nTo be fully integrated, we need a base programming language that can actually run all modern software efficiently across all relevant target platforms.</p>\n<p>Only TypeScript is seriously close to being a universal software foundation: it is the most popular and familiar programming language, it runs <em>directly</em> on the web, and the web is the most ubiquitous software platform.\nThe TypeScript ecosystem has good - if not perfect - conceptions of answers to all modern software needs, from great developer tools to rich interactive frontends to reasonably performant backends.</p>\n<p>Disregarding the legacy JavaScript baggage, modern TypeScript is surprisingly close to a fully statically compilable language - indeed, most browsers retrofit compilation internally already based on this assumption.\nEmbracing TypeScript and &quot;the web ecosystem&quot; lets us build a new toolchain that covers the full stack, is immediately familiar to millions of developers, runs transparently on existing targets, and can be made to run as fast as the machine allows.</p>\n", {  }),
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

import { defineExample } from "@destack/package/declare";
import { Prose } from "./prose.tsx";

/** An article as Markdown renders it: headings, a list, a quotation, code and a table. */
const ARTICLE = `<h1>Release notes</h1>
<p>This release brings <a href="#sync">live sync</a> to every space and a faster <code>destack build</code>.</p>
<h2>What changed</h2>
<ul><li>Spaces sync their objects as they change.</li><li>Builds reuse unchanged packages.</li><li><input type="checkbox" checked disabled> Docs updated</li></ul>
<blockquote><p>The fastest build is the one you skip.</p></blockquote>
<pre><code>destack build --watch</code></pre>
<table><thead><tr><th>Command</th><th>Before</th><th>After</th></tr></thead>
<tbody><tr><td><code>build</code></td><td>12 s</td><td>3 s</td></tr><tr><td><code>sync</code></td><td>manual</td><td>live</td></tr></tbody></table>`;

/** A GitHub alert and a footnote, as GitHub renders them. */
const NOTES = `<p>Sync runs over your space's own database.<sup><a href="#fn-1" id="fnref-1" data-footnote-ref>1</a></sup></p>
<div class="markdown-alert markdown-alert-warning"><p class="markdown-alert-title">Warning</p><p>Leaving a space removes its synced copy.</p></div>
<section data-footnotes><ol><li id="fn-1"><p>One SQLite database per space. <a href="#fnref-1" data-footnote-backref>↩</a></p></li></ol></section>`;

/** An article with headings, a list, a quotation, code and a table. */
export const proseArticle = defineExample({
    of: Prose,
    name: "article",
    description: "an article with headings, a list, a quotation, code and a table",
    render: () => <Prose innerHTML={ARTICLE} />,
});

/** A paragraph with a footnote and a warning alert. */
export const proseNotes = defineExample({
    of: Prose,
    name: "notes",
    description: "a paragraph with a footnote and a warning alert",
    render: () => <Prose innerHTML={NOTES} />,
});

import assert from "node:assert/strict";
import { renderListing } from "../src/content/listing.ts";
import { renderMarkdown } from "./markdown.ts";
import { highlightCode } from "./highlight.ts";

// authored fences and generated examples must produce the same listing
const source = 'import { signal } from "@destack/signals";';
const highlighted = highlightCode(source, "ts");
const rendered = renderMarkdown(`\`\`\`ts\n${source}\n\`\`\``, { assets: [] });
assert.equal(rendered.trim(), renderListing(highlighted, { label: "ts" }));
assert.ok(!rendered.includes("figcaption"));
assert.match(rendered, /data-publication-gutter aria-hidden="true">1</);

// captions escape text and links while highlighted lines retain their markup
const listing = renderListing('<span class="hl-keyword">export</span>\n\nvalue', {
    title: 'src/a<&".ts', href: 'https://example.com/?a=1&b="2"', detail: "L1–3",
});
assert.equal(listing, [
    "<figure class=\"markdown-code\" data-publication-listing>",
    "<figcaption data-publication-caption><span data-publication-caption-title><a href=\"https://example.com/?a=1&amp;b=&quot;2&quot;\" rel=\"external noopener noreferrer\" target=\"_blank\">src/a&lt;&amp;&quot;.ts</a></span><span data-publication-caption-detail>L1–3</span></figcaption>",
    "<pre data-publication-body tabindex=\"0\" aria-label=\"src/a&lt;&amp;&quot;.ts\"><code class=\"markdown-code-lines\" data-publication-lines>",
    "<span class=\"markdown-code-line\" data-publication-line><span class=\"markdown-code-gutter\" data-publication-gutter aria-hidden=\"true\">1</span><span class=\"markdown-code-text\" data-publication-code><span class=\"hl-keyword\">export</span></span></span>",
    "<span class=\"markdown-code-line\" data-publication-line><span class=\"markdown-code-gutter\" data-publication-gutter aria-hidden=\"true\">2</span><span class=\"markdown-code-text\" data-publication-code> </span></span>",
    "<span class=\"markdown-code-line\" data-publication-line><span class=\"markdown-code-gutter\" data-publication-gutter aria-hidden=\"true\">3</span><span class=\"markdown-code-text\" data-publication-code>value</span></span></code></pre></figure>",
].join(""));
console.log("listing tests passed");

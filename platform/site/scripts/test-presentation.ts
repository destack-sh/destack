import assert from "node:assert/strict";
import { renderMarkdown } from "./markdown.ts";
import { renderContentList } from "../src/content/presentation.ts";

// keep generated navigation and browser-rendered archives on the same safe markup
const html = renderContentList([
    { title: 'A < B', href: '/docs/?q="x"', summary: 'One & two' },
    { title: 'Article', href: '/blog/article/', date: '2026-09-21' },
]);
assert.equal(html, [
    "<ul class=\"content-list\">",
    "<li><a class=\"content-entry\" href=\"/docs/?q=&quot;x&quot;\"><span class=\"content-entry-title\">A &lt; B</span><svg class=\"content-arrow\" aria-hidden=\"true\" viewBox=\"0 0 24 24\" fill=\"none\"><path d=\"M5 12h14m-6-6 6 6-6 6\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg><span class=\"content-entry-summary\">One &amp; two</span></a></li>",
    "<li><a class=\"content-entry\" data-dated=\"true\" href=\"/blog/article/\"><span class=\"content-entry-title\">Article</span><time class=\"content-entry-meta\" datetime=\"2026-09-21\">21 September 2026</time><svg class=\"content-arrow\" aria-hidden=\"true\" viewBox=\"0 0 24 24\" fill=\"none\"><path d=\"M5 12h14m-6-6 6 6-6 6\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg></a></li></ul>",
].join(""));

// separate attribution from quoted words without altering ordinary quotes or alerts
const context = { assets: [] };
assert.equal(renderMarkdown(
    "> First paragraph.\n>\n> Second paragraph.\n>\n> — Author, *A Work* (1972)\n", context),
    '<figure class="markdown-quote"><blockquote>\n<p>First paragraph.</p>\n<p>Second paragraph.</p>\n</blockquote><figcaption>— Author, <em>A Work</em> (1972)</figcaption></figure>\n');
assert.equal(renderMarkdown("> An ordinary quote.\n", context), '<blockquote>\n<p>An ordinary quote.</p>\n</blockquote>\n');
assert.match(renderMarkdown("> [!NOTE]\n> A note.\n", context), /^<aside class="markdown-callout"/);

console.log('shared content presentation passed');

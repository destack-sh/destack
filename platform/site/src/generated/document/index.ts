import type { DocumentContent } from "../documents";

const content = {
    html: resolveAssets("<h1 id=\"destack-documentation\">Destack Documentation</h1><p>Destack is an integrated language, toolchain, runtime, and library for building software from\nstrict TypeScript-shaped source.</p>\n<h2 id=\"start-here\">Start Here</h2><ul>\n<li><a href=\"/docs/overview/\">Overview</a>: understand the language, toolchain, runtime, and broader stack.</li>\n<li><a href=\"/docs/comparison/\">Comparison</a>: compare Destack with TypeScript and adjacent systems.</li>\n<li><a href=\"/docs/language/\">Language</a>: learn the language and its relationship to strict TypeScript.</li>\n<li><a href=\"/docs/language/runtime/\">Runtime</a>: understand execution, effects, and host bindings.</li>\n</ul>\n<h2 id=\"install-destack\">Install Destack</h2><p>Install the current release and create a new application:</p>\n<figure class=\"markdown-code\"><figcaption><span class=\"markdown-code__title\">Install Destack</span><span class=\"markdown-code__format\">.sh</span></figcaption><pre tabindex=\"0\"><code class=\"markdown-code-lines\"><span class=\"markdown-code-line\"><span class=\"markdown-code-gutter\">1</span><span class=\"markdown-code-text\">curl -fsSL https://destack.sh/install | sh</span></span><span class=\"markdown-code-line\"><span class=\"markdown-code-gutter\">2</span><span class=\"markdown-code-text\">destack new my-destack-app</span></span></code></pre></figure>", {  }),
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

import engineer_great_software_asset0 from "../content/blog/engineer-great-software/compiler.svg?url";

export type PostStatus = "draft" | "published";

export type Post = {
    author: string;
    date: string;
    html: string;
    route: string;
    slug: string;
    status: PostStatus;
    summary: string;
    tableOfContents: readonly TableOfContentsEntry[];
    tags: readonly string[];
    title: string;
};

export type TableOfContentsEntry = {
    depth: number;
    id: string;
    text: string;
};

export const posts = [
    {
        author: "Florian",
        date: "2026-05-30",
        html: resolveAssets("<p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.\nInteger porta sem at lorem varius, non cursus magna luctus.\nPraesent vitae ipsum sed nulla dictum posuere.</p>\n<aside class=\"blog-callout\" data-kind=\"note\"><strong>note</strong><p>Lorem ipsum dolor sit amet.\nVestibulum ante ipsum primis in faucibus.</p>\n</aside><h2 id=\"placeholder-section\">placeholder section</h2><p>Curabitur luctus, ipsum non tempor feugiat, turpis justo sodales lectus, vitae luctus eros neque ac urna.\nDonec feugiat lorem id risus dictum, et malesuada lorem posuere.\nMorbi laoreet neque at ante interdum, quis facilisis magna dictum.</p>\n<figure class=\"blog-figure\"><img alt=\"Placeholder block diagram with source, middle, and output columns.\" src=\"__BLOG_ASSET_0__\"><figcaption><span>figure 1</span>A deliberately placeholder figure that keeps the final diagram slot visible.</figcaption></figure><h2 id=\"placeholder-code\">placeholder code</h2><p>Sed euismod, lorem vel fermentum posuere, ipsum risus iaculis tortor, non gravida neque sem vitae mi.\nAliquam erat volutpat.</p>\n<figure class=\"blog-code\"><figcaption><span>listing 2</span>ds</figcaption><pre><code><span class=\"hljs-keyword\">type</span> PlaceholderId = <span class=\"hljs-built_in\">uint64</span>;\n\n<span class=\"hljs-keyword\">struct</span> PlaceholderRecord {\n    id: PlaceholderId;\n    label: <span class=\"hljs-built_in\">string</span>;\n    count: uint32;\n}\n\n<span class=\"hljs-keyword\">function</span> renderPlaceholder(record: PlaceholderRecord): <span class=\"hljs-built_in\">string</span> {\n    <span class=\"hljs-keyword\">return</span> <span class=\"hljs-string\">`${record.label}:${record.count}`</span>;\n}</code></pre></figure><h2 id=\"placeholder-diagram\">placeholder diagram</h2><p>Phasellus vitae lorem at ipsum lacinia dictum.\nNunc sed magna non lorem blandit gravida.<sup class=\"blog-footnote-ref\" id=\"fnref-placeholder\"><a href=\"#fn-placeholder\">1</a></sup></p>\n<figure class=\"blog-diagram\"><figcaption><span>figure 3</span>diagram</figcaption><pre><code>placeholder input\n    |\n    v\nplaceholder transform\n    |\n    +------&gt; placeholder branch\n    |\n    v\nplaceholder output</code></pre></figure><h2 id=\"replace-this-later\">replace this later</h2><p>Etiam tempor ipsum sed lorem tincidunt, a porttitor justo sagittis.\nSuspendisse potenti.\nMauris vel lorem sit amet ipsum sodales pretium.</p>\n<section class=\"blog-footnotes\"><h2>notes</h2><ol><li id=\"fn-placeholder\"><span class=\"blog-footnote-number\">1</span><span>Lorem ipsum dolor sit amet, consectetur adipiscing elit. <a class=\"blog-footnote-back\" href=\"#fnref-placeholder\">back</a></span></li></ol></section>", { "__BLOG_ASSET_0__": engineer_great_software_asset0 }),
        route: "/blog/engineer-great-software/",
        slug: "engineer-great-software",
        status: "draft",
        summary: "Temporary placeholder copy for exercising blog prose, diagrams, code blocks, figures, footnotes, and archive rendering.",
        tableOfContents: [{"depth":2,"id":"placeholder-section","text":"placeholder section"},{"depth":2,"id":"placeholder-code","text":"placeholder code"},{"depth":2,"id":"placeholder-diagram","text":"placeholder diagram"},{"depth":2,"id":"replace-this-later","text":"replace this later"}],
        tags: ["placeholder","blog","layout"],
        title: "placeholder blog post",
    }
] as const satisfies readonly Post[];

export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

function resolveAssets(html: string, assets: Record<string, string>) {
    let resolved = html;

    for (const [placeholder, asset] of Object.entries(assets)) {
        resolved = resolved.replaceAll(placeholder, asset);
    }

    return resolved;
}

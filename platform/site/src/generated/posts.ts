import { loadContent, type RenderedContent } from "../content/load";

export type Post = {
    /// The post author.
    author: string;
    /// The static rendered HTML route.
    contentRoute: string;
    /// The publication date.
    date: string;
    /// The authored Markdown route.
    markdownRoute: string;
    /// The canonical browser route.
    route: string;
    /// The canonical post slug.
    slug: string;
    /// The post subtitle.
    subtitle: string;
    /// The rendered heading tree.
    tableOfContents: readonly TableOfContentsEntry[];
    /// The plain text route.
    textRoute: string;
    /// The post title.
    title: string;
    /// The approximate token count.
    tokens: number;
};

/// One rendered post body.
export type PostContent = RenderedContent;

/// One rendered post heading.
export type TableOfContentsEntry = {
    /// The heading depth.
    depth: number;
    /// The heading fragment identifier.
    id: string;
    /// The heading text.
    text: string;
};

/// The generated blog posts.
export const posts = [
    {
        author: "Florian",
        contentRoute: "/_content/html/4d43ba66351722520c7d9f9c63ef341a2ed690def10224168a83cbf736dbb25f.html",
        date: "2026-09-14",
        markdownRoute: "/blog/introducing-typescriptpp.md",
        route: "/blog/introducing-typescriptpp/",
        slug: "introducing-typescriptpp",
        subtitle: "Evolving TypeScript into the Last Programming Language",
        tableOfContents: [{"depth":1,"id":"higher-order-programming","text":"Higher Order Programming"},{"depth":1,"id":"long-live-code","text":"Long Live Code"},{"depth":1,"id":"systems-and-scaffolding","text":"Systems and Scaffolding"},{"depth":1,"id":"boring-software","text":"Boring Software"},{"depth":1,"id":"typescript","text":"TypeScript++"}],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "Introducing TypeScript++",
        tokens: 3357,
    }
] as const satisfies readonly Post[];

/// Blog posts indexed by slug.
export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

/// Load one rendered post body by slug.
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    const post = postBySlug.get(slug);

    return post == undefined ? undefined : loadContent(post.contentRoute);
}

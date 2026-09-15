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
        contentRoute: "/_content/html/de65c0b91d08de75abf2666aad8c83c6f8e06b820c9f6ceea0f955dd14325f28.html",
        date: "2026-09-21",
        markdownRoute: "/blog/introducing-destack.md",
        route: "/blog/introducing-destack/",
        slug: "introducing-destack",
        subtitle: "TypeScript++, the last programming language, and the final stack.",
        tableOfContents: [{"depth":1,"id":"the-software-we-were-promised","text":"The Software We Were Promised"},{"depth":1,"id":"higher-order-programming","text":"Higher Order Programming"},{"depth":1,"id":"the-system-and-the-meta-system","text":"The System and The Meta System"},{"depth":1,"id":"destack","text":"Destack"},{"depth":1,"id":"typescript","text":"TypeScript++"}],
        textRoute: "/blog/introducing-destack.txt",
        title: "Introducing Destack",
        tokens: 3489,
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

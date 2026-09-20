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

/// Portable formats for the blog directory.
export const blogIndex = {"markdownRoute":"/blog/index.md","textRoute":"/blog/index.txt","tokens":24};

/// The generated blog posts.
export const posts = [
    {
        author: "Florian",
        contentRoute: "/_content/html/b4615386771bd8cd0132ec7a1a5a6c7ec1f51e3d5c7ff501fbf794a9fce787dc.html",
        date: "2026-09-21",
        markdownRoute: "/blog/introducing-destack.md",
        route: "/blog/introducing-destack/",
        slug: "introducing-destack",
        subtitle: "TypeScript, the final stack, and software you can own.",
        tableOfContents: [{"depth":1,"id":"the-software-that-could-be","text":"The Software That Could Be"},{"depth":1,"id":"higher-order-programming","text":"Higher Order Programming"},{"depth":1,"id":"the-system-and-the-scaffolding","text":"The System and The Scaffolding"},{"depth":1,"id":"destack","text":"Destack"}],
        textRoute: "/blog/introducing-destack.txt",
        title: "Introducing Destack",
        tokens: 3379,
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

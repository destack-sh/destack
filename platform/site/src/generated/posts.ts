import { loadContent, type RenderedContent } from "../content/load";

export type Post = {
    /// The post author.
    author: string;
    /// The static rendered HTML route.
    contentRoute: string;
    /// The post's first figure image, shown on its directory entry.
    cover: { src: string; alt: string } | null;
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
export const blogIndex = {
    markdownRoute: "/blog/index.md",
    textRoute: "/blog/index.txt",
    tokens: 24,
};

/// The generated blog posts.
export const posts = [
    {
        author: "Florian",
        contentRoute:
            "/_content/html/acca948815a7912f8d74b72420c89cbd275527b3dffa1a7fe90f41a8b8aaf1ea.html",
        cover: {
            src: "/_content/assets/aade6fe26c510ace.jpg",
            alt: "Illustration of Cambrian marine life, with Opabinia swimming above trilobites, spiny animals, and sponges.",
        },
        date: "2026-09-21",
        markdownRoute: "/blog/introducing-destack.md",
        route: "/blog/introducing-destack/",
        slug: "introducing-destack",
        subtitle: "TypeScript, the final stack, and software you can own.",
        tableOfContents: [
            { depth: 1, id: "the-software-that-could-be", text: "The Software That Could Be" },
            { depth: 1, id: "higher-order-programming", text: "Higher Order Programming" },
            {
                depth: 1,
                id: "the-system-and-the-scaffolding",
                text: "The System and The Scaffolding",
            },
            { depth: 1, id: "destack", text: "Destack" },
        ],
        textRoute: "/blog/introducing-destack.txt",
        title: "Introducing Destack",
        tokens: 3353,
    },
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

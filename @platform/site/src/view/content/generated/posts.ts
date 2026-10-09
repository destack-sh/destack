import { loadContent, type RenderedContent } from "../load";

/** One generated blog post record. */
export type Post = {
    /** The post author. */
    author: string;
    /** The static rendered HTML route. */
    contentRoute: string;
    /** The post's first figure image, shown on its directory entry. */
    cover: { source: string; alt: string } | null;
    /** The publication date. */
    date: string;
    /** The authored Markdown route. */
    markdownRoute: string;
    /** The canonical browser route. */
    route: string;
    /** The canonical post slug. */
    slug: string;
    /** The post subtitle. */
    subtitle: string;
    /** The rendered heading tree. */
    tableOfContents: readonly TableOfContentsEntry[];
    /** The plain text route. */
    textRoute: string;
    /** The post title. */
    title: string;
    /** The approximate token count. */
    tokens: number;
};

/** One rendered post body. */
export type PostContent = RenderedContent;

/** One rendered post heading. */
export type TableOfContentsEntry = {
    /** The heading depth. */
    depth: number;
    /** The heading fragment identifier. */
    id: string;
    /** The heading text. */
    text: string;
};

/** Portable formats for the blog directory. */
export const blogIndex = {
    markdownRoute: "/blog/index.md",
    textRoute: "/blog/index.txt",
    tokens: 25,
};

/** The generated blog posts. */
export const posts = [
    {
        author: "Florian",
        contentRoute:
            "/_content/tree/1821d251cf091e2b1edcad5e2e9d5d666a1fcf88b2cbb3b21c8666c734840145.json",
        cover: {
            source: "/_content/assets/aade6fe26c510ace.jpg",
            alt: "Illustration of Cambrian marine life, with Opabinia swimming above trilobites, spiny animals, and sponges.",
        },
        date: "2026-10-14",
        markdownRoute: "/blog/introducing-destack.md",
        route: "/blog/introducing-destack/",
        slug: "introducing-destack",
        subtitle: "The final stack for personal software (for real this time).",
        tableOfContents: [
            { depth: 1, id: "higher-order-programming", text: "Higher Order Programming" },
            { depth: 1, id: "the-software-engine", text: "The Software Engine" },
            { depth: 1, id: "the-destack", text: "The Destack" },
        ],
        textRoute: "/blog/introducing-destack.txt",
        title: "Introducing Destack",
        tokens: 2500,
    },
] as const satisfies readonly Post[];

/** Blog posts indexed by slug. */
export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

/** Load one rendered post body by slug. */
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    const post = postBySlug.get(slug);

    return post == undefined ? undefined : loadContent(post.contentRoute);
}

import type { ImageMetadata } from "@destack/view/media";
import { loadContent, type RenderedContent } from "../load";

/** One generated blog post record. */
export type Post = {
    /** The post author. */
    author: string;
    /** The static rendered HTML route. */
    contentRoute: string;
    /** The post's first figure image, shown on its directory entry. */
    cover: { image: ImageMetadata; alt: string } | null;
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
            "/_content/tree/bdde7211423cf80bc014bd13de537552d89c631d6e9656dd91fcef92ded8802c.json",
        cover: {
            image: {
                src: "/_content/assets/aade6fe26c510ace.jpg",
                width: 673,
                height: 600,
                format: "jpeg",
                srcSet: [
                    {
                        width: 640,
                        format: "avif",
                        src: "/_content/assets/aade6fe26c510ace-640.avif",
                    },
                    {
                        width: 640,
                        format: "webp",
                        src: "/_content/assets/aade6fe26c510ace-640.webp",
                    },
                    {
                        width: 640,
                        format: "jpeg",
                        src: "/_content/assets/aade6fe26c510ace-640.jpeg",
                    },
                    {
                        width: 673,
                        format: "avif",
                        src: "/_content/assets/aade6fe26c510ace-673.avif",
                    },
                    {
                        width: 673,
                        format: "webp",
                        src: "/_content/assets/aade6fe26c510ace-673.webp",
                    },
                    { width: 673, format: "jpeg", src: "/_content/assets/aade6fe26c510ace.jpg" },
                ],
            },
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
        tokens: 2499,
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

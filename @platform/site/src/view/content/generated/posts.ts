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
            "/_content/tree/e28dec7ff2d8b4493b8008f14ec2db0faad84afd238c2f7634d6f507e6734976.json",
        cover: {
            image: {
                src: "/_content/assets/9f1ff7f1d24e2f62.jpg",
                width: 2560,
                height: 1170,
                format: "jpeg",
                srcSet: [
                    {
                        width: 640,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-640.avif",
                    },
                    {
                        width: 640,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-640.webp",
                    },
                    {
                        width: 640,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-640.jpeg",
                    },
                    {
                        width: 750,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-750.avif",
                    },
                    {
                        width: 750,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-750.webp",
                    },
                    {
                        width: 750,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-750.jpeg",
                    },
                    {
                        width: 828,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-828.avif",
                    },
                    {
                        width: 828,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-828.webp",
                    },
                    {
                        width: 828,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-828.jpeg",
                    },
                    {
                        width: 1080,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1080.avif",
                    },
                    {
                        width: 1080,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1080.webp",
                    },
                    {
                        width: 1080,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1080.jpeg",
                    },
                    {
                        width: 1200,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1200.avif",
                    },
                    {
                        width: 1200,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1200.webp",
                    },
                    {
                        width: 1200,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1200.jpeg",
                    },
                    {
                        width: 1920,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1920.avif",
                    },
                    {
                        width: 1920,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1920.webp",
                    },
                    {
                        width: 1920,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-1920.jpeg",
                    },
                    {
                        width: 2048,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-2048.avif",
                    },
                    {
                        width: 2048,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-2048.webp",
                    },
                    {
                        width: 2048,
                        format: "jpeg",
                        src: "/_content/assets/9f1ff7f1d24e2f62-2048.jpeg",
                    },
                    {
                        width: 2560,
                        format: "avif",
                        src: "/_content/assets/9f1ff7f1d24e2f62-2560.avif",
                    },
                    {
                        width: 2560,
                        format: "webp",
                        src: "/_content/assets/9f1ff7f1d24e2f62-2560.webp",
                    },
                    { width: 2560, format: "jpeg", src: "/_content/assets/9f1ff7f1d24e2f62.jpg" },
                ],
            },
            alt: "Illustration of a Cambrian ecosystem, with Kylinxia, trilobites, worms, and colourful sponges beneath deep blue water.",
        },
        date: "2026-10-19",
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
        tokens: 2609,
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

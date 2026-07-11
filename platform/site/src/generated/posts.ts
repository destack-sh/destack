export type Post = {
    author: string;
    date: string;
    markdownRoute: string;
    route: string;
    slug: string;
    subtitle: string;
    tableOfContents: readonly TableOfContentsEntry[];
    tags: readonly string[];
    textRoute: string;
    title: string;
    tokens: number;
};

export type PostContent = {
    html: string;
};

export type TableOfContentsEntry = {
    depth: number;
    id: string;
    text: string;
};

export const posts = [
    {
        author: "Florian",
        date: "2026-06-18",
        markdownRoute: "/blog/introducing-destack.md",
        route: "/blog/introducing-destack/",
        slug: "introducing-destack",
        subtitle: "the absurdly integrated stack for humans building correct, optimal, integrated software systems on TypeScript++",
        tableOfContents: [{"depth":2,"id":"higher-order-programming","text":"higher-order programming"},{"depth":2,"id":"the-system-is-the-specification","text":"the system is the specification"},{"depth":2,"id":"correctness-alignment-visibility","text":"correctness = alignment + visibility"},{"depth":2,"id":"one-language-one-toolchain-one-stack","text":"one language, one toolchain, one stack"},{"depth":2,"id":"human-first-design","text":"human-first design"},{"depth":2,"id":"typescript","text":"typescript++"},{"depth":2,"id":"optimality-requires-expressivity","text":"optimality requires expressivity"},{"depth":2,"id":"homoiconicity-hackability","text":"homoiconicity -> hackability"},{"depth":2,"id":"bootstrapping-an-ecosystem","text":"bootstrapping an ecosystem"}],
        tags: ["language","runtime","platform"],
        textRoute: "/blog/introducing-destack.txt",
        title: "introducing destack",
        tokens: 3355,
    }
] as const satisfies readonly Post[];

export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

const postLoaders: Record<string, () => Promise<{ default: PostContent }>> = {
    "introducing-destack": () => import("./post/introducing-destack"),
};

/// Load one rendered post body by slug.
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    return postLoaders[slug]?.().then((module) => module.default);
}

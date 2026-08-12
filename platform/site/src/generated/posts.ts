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
        date: "2026-08-18",
        markdownRoute: "/blog/introducing-typescriptpp.md",
        route: "/blog/introducing-typescriptpp/",
        slug: "introducing-typescriptpp",
        subtitle: "evolving TypeScript into the last programming language",
        tableOfContents: [{"depth":1,"id":"introduction","text":"introduction"},{"depth":1,"id":"why","text":"why"},{"depth":2,"id":"why-even-bother-with-programming-languages","text":"why even bother with programming languages"},{"depth":2,"id":"why-do-we-need-a-universal-language","text":"why do we need a universal language"},{"depth":2,"id":"why-combine-typescript-and-rust","text":"why combine typescript and rust"},{"depth":2,"id":"why-stay-within-the-lines","text":"why stay within the lines"},{"depth":2,"id":"why-now","text":"why now"},{"depth":1,"id":"how","text":"how"},{"depth":2,"id":"no-backward-compatibility","text":"no backward compatibility"},{"depth":2,"id":"obviously-no-soundness-hole","text":"obviously, no <soundness hole>"},{"depth":2,"id":"strictest-ts","text":"strictest TS"},{"depth":2,"id":"esm-modules","text":"ESM modules"},{"depth":2,"id":"tsx-too-of-course","text":"TSX, too, of course"},{"depth":2,"id":"constrained-dynamic","text":"constrained dynamic (?)"},{"depth":2,"id":"proper-primitives","text":"proper primitives"},{"depth":2,"id":"no-exceptions-results-only","text":"no exceptions, results only"},{"depth":2,"id":"patterns","text":"patterns"},{"depth":2,"id":"decorators","text":"decorators"},{"depth":2,"id":"nominality","text":"nominality"},{"depth":2,"id":"extensions","text":"extensions"},{"depth":2,"id":"operator-overloading","text":"operator overloading"},{"depth":2,"id":"objects-as-types","text":"objects as types"},{"depth":2,"id":"classes-yes-but-which-ones","text":"classes, yes, but which ones"},{"depth":2,"id":"dynamic-and-structural-interfaces","text":"Dynamic and structural interfaces"},{"depth":2,"id":"generics-and-variance","text":"generics and variance"},{"depth":2,"id":"structs-and-value-types","text":"structs and value types"},{"depth":2,"id":"ownership","text":"ownership"},{"depth":2,"id":"borrowing","text":"borrowing"},{"depth":2,"id":"lifetimes","text":"lifetimes"},{"depth":2,"id":"access-mutability-exclusive","text":"access, mutability, exclusive"},{"depth":2,"id":"local-and-shared-memory-spaces","text":"local and shared memory spaces"},{"depth":2,"id":"async-promise-tasks","text":"async, promise, tasks"},{"depth":2,"id":"panics-traps","text":"panics, traps"}],
        tags: ["language","runtime","platform"],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "introducing typescript++",
        tokens: 3050,
    }
] as const satisfies readonly Post[];

export const postBySlug: ReadonlyMap<string, Post> = new Map(
    posts.map((post): [string, Post] => [post.slug, post]),
);

const postLoaders: Record<string, () => Promise<{ default: PostContent }>> = {
    "introducing-typescriptpp": () => import("./post/introducing-typescriptpp"),
};

/// Load one rendered post body by slug.
export async function loadPost(slug: string): Promise<PostContent | undefined> {
    return postLoaders[slug]?.().then((module) => module.default);
}

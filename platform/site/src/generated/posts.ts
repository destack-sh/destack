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
        subtitle: "Evolving TypeScript into the Last Programming Language",
        tableOfContents: [{"depth":1,"id":"introduction","text":"Introduction"},{"depth":1,"id":"why","text":"Why"},{"depth":2,"id":"why-human-first-design","text":"Why Human First Design"},{"depth":2,"id":"why-does-it-need-to-be-universal","text":"Why Does It Need to Be \"Universal\""},{"depth":2,"id":"why-combine-typescript-and-rust","text":"Why \"Combine\" TypeScript and Rust"},{"depth":2,"id":"why-stay-within-the-lines","text":"Why Stay Within the Lines"},{"depth":1,"id":"how","text":"How"},{"depth":2,"id":"no-backward-compatibility","text":"No Backward Compatibility"},{"depth":2,"id":"destack-json","text":"destack.json"},{"depth":2,"id":"obviously-no-soundness-holes","text":"Obviously, No Soundness Holes"},{"depth":2,"id":"strictest-ts","text":"Strictest TS"},{"depth":2,"id":"esm-modules","text":"ESM Modules"},{"depth":2,"id":"proper-primitives","text":"Proper Primitives"},{"depth":2,"id":"nominality","text":"Nominality"},{"depth":2,"id":"tsx","text":"TSX"},{"depth":2,"id":"patterns-and-match","text":"Patterns and Match"},{"depth":2,"id":"decorators","text":"Decorators"},{"depth":2,"id":"no-exceptions-results-only","text":"No Exceptions, Results Only"},{"depth":2,"id":"structural-interfaces-dynamic","text":"Structural Interfaces, Dynamic (?)"},{"depth":2,"id":"extensions","text":"Extensions"},{"depth":2,"id":"operator-overloading","text":"Operator Overloading"},{"depth":2,"id":"objects-as-types","text":"Objects as Types"},{"depth":2,"id":"classes-yes-but-which-ones","text":"Classes, Yes, but Which Ones"},{"depth":2,"id":"dynamic-and-structural-interfaces","text":"Dynamic and Structural Interfaces"},{"depth":2,"id":"generics-and-variance","text":"Generics and Variance"},{"depth":2,"id":"structs-and-value-types","text":"Structs and Value Types"},{"depth":2,"id":"ownership","text":"Ownership"},{"depth":2,"id":"borrowing","text":"Borrowing"},{"depth":2,"id":"lifetimes","text":"Lifetimes"},{"depth":2,"id":"access-mutability-exclusive","text":"Access, Mutability, Exclusive"},{"depth":2,"id":"local-and-shared-memory-spaces","text":"Local and Shared Memory Spaces"},{"depth":2,"id":"async-promise-tasks","text":"Async, Promise, Tasks"},{"depth":2,"id":"panics-traps","text":"Panics, Traps"},{"depth":2,"id":"automatic-implicit-effects","text":"Automatic, Implicit Effects"},{"depth":2,"id":"durability","text":"Durability"},{"depth":2,"id":"generalied-module","text":"Generalied Module"}],
        tags: ["language","runtime"],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "Introducing TypeScript++",
        tokens: 3084,
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

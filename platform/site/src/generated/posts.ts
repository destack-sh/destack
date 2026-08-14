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
        date: "2026-08-21",
        markdownRoute: "/blog/introducing-typescriptpp.md",
        route: "/blog/introducing-typescriptpp/",
        slug: "introducing-typescriptpp",
        subtitle: "Making TypeScript the Last Programming Language",
        tableOfContents: [{"depth":1,"id":"why","text":"Why"},{"depth":2,"id":"why-care-about-code","text":"Why Care About Code"},{"depth":2,"id":"why-human-first","text":"Why \"Human-First\""},{"depth":2,"id":"why-universal","text":"Why \"Universal\""},{"depth":2,"id":"why-not-correct-all-ills","text":"Why Not Correct All Ills"},{"depth":1,"id":"how","text":"How"},{"depth":2,"id":"types","text":"Types"},{"depth":3,"id":"obviously-no-soundness-holes","text":"Obviously, No Soundness Holes"},{"depth":3,"id":"strictest-ts","text":"Strictest TS"},{"depth":3,"id":"proper-primitives","text":"Proper Primitives"},{"depth":3,"id":"arrays-slices-and-tuples","text":"Arrays, Slices and Tuples"},{"depth":3,"id":"classes","text":"Classes"},{"depth":3,"id":"enums","text":"Enums"},{"depth":3,"id":"this","text":"This"},{"depth":3,"id":"nominality","text":"Nominality"},{"depth":3,"id":"narrowing-instanceof-typeof-is","text":"Narrowing, instanceof, typeof, is"},{"depth":3,"id":"readonly","text":"Readonly"},{"depth":3,"id":"type-aliases-and-algebra","text":"Type Aliases and ALgebra"},{"depth":3,"id":"unions","text":"Unions"},{"depth":3,"id":"structural-interfaces","text":"Structural Interfaces"},{"depth":3,"id":"funky-signatures","text":"Funky Signatures"},{"depth":3,"id":"generics-and-variance","text":"Generics and Variance"},{"depth":2,"id":"expressions","text":"Expressions"},{"depth":3,"id":"tsx","text":"TSX"},{"depth":3,"id":"patterns-and-match","text":"Patterns and Match"},{"depth":3,"id":"decorators","text":"Decorators"},{"depth":3,"id":"no-exceptions-only-results","text":"No Exceptions, Only Results"},{"depth":3,"id":"ranges","text":"Ranges"},{"depth":3,"id":"extensions","text":"Extensions"},{"depth":3,"id":"operator-overloading","text":"Operator Overloading"},{"depth":3,"id":"const-evaluation","text":"Const Evaluation"},{"depth":3,"id":"functions-and-captures","text":"Functions and Captures"},{"depth":3,"id":"async-promise-tasks","text":"Async, Promise, Tasks"},{"depth":3,"id":"panics-traps","text":"Panics, Traps"},{"depth":3,"id":"context-contextvars","text":"Context, ContextVars"},{"depth":2,"id":"memory","text":"Memory"},{"depth":3,"id":"representation","text":"Representation"},{"depth":3,"id":"local-and-shared-memory-spaces","text":"Local and Shared Memory Spaces"},{"depth":3,"id":"structs-and-value-types","text":"Structs and Value Types"},{"depth":3,"id":"ownership","text":"Ownership"},{"depth":3,"id":"references-and-pointers","text":"References and Pointers"},{"depth":3,"id":"borrowing","text":"Borrowing"},{"depth":3,"id":"lifetimes","text":"Lifetimes"},{"depth":3,"id":"access-mutability-exclusive","text":"Access, Mutability, Exclusive"},{"depth":3,"id":"memory-type-algebra","text":"Memory Type Algebra"},{"depth":2,"id":"runtime","text":"Runtime"},{"depth":3,"id":"burning-the-boats","text":"Burning the Boats"},{"depth":3,"id":"destack-json","text":"destack.json"},{"depth":3,"id":"esm-modules","text":"ESM Modules"},{"depth":3,"id":"effects-bindings","text":"Effects / Bindings"},{"depth":3,"id":"durability","text":"Durability"},{"depth":3,"id":"policy","text":"Policy"},{"depth":3,"id":"import-meta","text":"import.meta"},{"depth":3,"id":"conditions","text":"Conditions"}],
        tags: ["language","runtime"],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "Introducing TypeScript++",
        tokens: 4149,
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

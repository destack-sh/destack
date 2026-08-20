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
        contentRoute: "/_content/blog/introducing-typescriptpp/index.html",
        date: "2026-08-21",
        markdownRoute: "/blog/introducing-typescriptpp.md",
        route: "/blog/introducing-typescriptpp/",
        slug: "introducing-typescriptpp",
        subtitle: "Evolving TypeScript into the Last Programming Language",
        tableOfContents: [{"depth":1,"id":"why","text":"Why"},{"depth":2,"id":"why-care-about-code","text":"Why Care About Code"},{"depth":2,"id":"why-design-human-first","text":"Why Design \"Human-First\""},{"depth":2,"id":"why-not-reinvent-everything","text":"Why Not Reinvent _Everything_"},{"depth":1,"id":"how","text":"How"},{"depth":2,"id":"types","text":"Types"},{"depth":3,"id":"soundness","text":"Soundness"},{"depth":3,"id":"primitives","text":"Primitives"},{"depth":3,"id":"enums","text":"Enums"},{"depth":3,"id":"arrays-slices-and-tuples","text":"Arrays, Slices and Tuples"},{"depth":3,"id":"classes","text":"Classes"},{"depth":3,"id":"this","text":"This"},{"depth":3,"id":"nominality","text":"Nominality"},{"depth":3,"id":"readonly","text":"Readonly"},{"depth":3,"id":"visibility","text":"Visibility"},{"depth":3,"id":"algebra","text":"Algebra"},{"depth":3,"id":"unions","text":"Unions"},{"depth":3,"id":"interfaces","text":"Interfaces"},{"depth":3,"id":"funky-signatures","text":"Funky Signatures"},{"depth":3,"id":"generics","text":"Generics"},{"depth":3,"id":"narrowing","text":"Narrowing"},{"depth":2,"id":"expressions","text":"Expressions"},{"depth":3,"id":"tsx","text":"TSX"},{"depth":3,"id":"patterns-and-match","text":"Patterns and Match"},{"depth":3,"id":"decorators","text":"Decorators"},{"depth":3,"id":"no-exceptions-only-results","text":"No Exceptions, Only Results"},{"depth":3,"id":"try-catch-finally","text":"Try-Catch-Finally"},{"depth":3,"id":"using","text":"Using"},{"depth":3,"id":"extensions","text":"Extensions"},{"depth":3,"id":"operator-overloading","text":"Operator Overloading"},{"depth":3,"id":"const-evaluation","text":"Const Evaluation"},{"depth":3,"id":"functions-lambdas-and-captures","text":"Functions, Lambdas and Captures"},{"depth":3,"id":"async","text":"Async"},{"depth":3,"id":"context-contextvars","text":"Context, ContextVars"},{"depth":3,"id":"panics-traps","text":"Panics, Traps"},{"depth":2,"id":"memory","text":"Memory"},{"depth":3,"id":"representation","text":"Representation"},{"depth":3,"id":"local-and-shared-memory","text":"Local and Shared Memory"},{"depth":3,"id":"structs","text":"Structs"},{"depth":3,"id":"ownership","text":"Ownership"},{"depth":3,"id":"borrowing","text":"Borrowing"},{"depth":3,"id":"lifetimes","text":"Lifetimes"},{"depth":3,"id":"mutability","text":"Mutability"},{"depth":2,"id":"runtime","text":"Runtime"},{"depth":3,"id":"write-once-run-everywhere","text":"\"Write Once, Run Everywhere\""},{"depth":3,"id":"destack-json","text":"destack.json"},{"depth":3,"id":"esm-modules","text":"ESM Modules"},{"depth":3,"id":"worker-first","text":"Worker-first"},{"depth":3,"id":"effects-bindings","text":"Effects / Bindings"},{"depth":3,"id":"import-meta","text":"import.meta"},{"depth":3,"id":"conditions","text":"Conditions"},{"depth":3,"id":"style","text":"Style"},{"depth":3,"id":"documentation","text":"Documentation"},{"depth":3,"id":"topology","text":"Topology"},{"depth":2,"id":"so","text":"So"}],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "Introducing TypeScript++",
        tokens: 6693,
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

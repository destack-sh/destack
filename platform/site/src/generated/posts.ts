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
        contentRoute: "/_content/html/67c24650cfa8fde15cd82452dbee35e5bc03ddd64e4eb81aca66d72e022ac9da.html",
        date: "2026-09-14",
        markdownRoute: "/blog/introducing-typescriptpp.md",
        route: "/blog/introducing-typescriptpp/",
        slug: "introducing-typescriptpp",
        subtitle: "Evolving TypeScript into the Last Programming Language",
        tableOfContents: [{"depth":1,"id":"why","text":"Why"},{"depth":2,"id":"why-code-first","text":"Why \"Code-First\""},{"depth":2,"id":"why-human-first","text":"Why \"Human-First\""},{"depth":2,"id":"why-universal","text":"Why \"Universal\""},{"depth":1,"id":"how","text":"How"},{"depth":2,"id":"types","text":"Types"},{"depth":3,"id":"soundness","text":"Soundness"},{"depth":3,"id":"primitives","text":"Primitives"},{"depth":3,"id":"sequences","text":"Sequences"},{"depth":3,"id":"enums","text":"Enums"},{"depth":3,"id":"classes","text":"Classes"},{"depth":3,"id":"structs","text":"Structs"},{"depth":3,"id":"newtypes","text":"Newtypes"},{"depth":3,"id":"extensions","text":"Extensions"},{"depth":3,"id":"algebra","text":"Algebra"},{"depth":3,"id":"unions","text":"Unions"},{"depth":3,"id":"interfaces","text":"Interfaces"},{"depth":3,"id":"generics","text":"Generics"},{"depth":2,"id":"expressions","text":"Expressions"},{"depth":3,"id":"patterns-and-match","text":"Patterns and Match"},{"depth":3,"id":"decorators","text":"Decorators"},{"depth":3,"id":"tsx","text":"TSX"},{"depth":3,"id":"result-and-try","text":"Result and Try"},{"depth":3,"id":"using","text":"Using"},{"depth":3,"id":"narrowing","text":"Narrowing"},{"depth":3,"id":"overloading","text":"Overloading"},{"depth":3,"id":"static-and-const","text":"Static and Const"},{"depth":3,"id":"functions-lambdas-and-captures","text":"Functions, Lambdas and Captures"},{"depth":3,"id":"async-and-promise","text":"Async and Promise"},{"depth":3,"id":"panic","text":"Panic"},{"depth":2,"id":"memory","text":"Memory"},{"depth":3,"id":"local-and-shared","text":"Local and Shared"},{"depth":3,"id":"layout","text":"Layout"},{"depth":3,"id":"ownership","text":"Ownership"},{"depth":3,"id":"borrowing","text":"Borrowing"},{"depth":3,"id":"mutability","text":"Mutability"},{"depth":3,"id":"drop","text":"Drop"},{"depth":2,"id":"runtime","text":"Runtime"},{"depth":3,"id":"esm-modules","text":"ESM Modules"},{"depth":3,"id":"documentation","text":"Documentation"},{"depth":3,"id":"destack-json","text":"destack.json"},{"depth":3,"id":"import-meta","text":"import.meta"},{"depth":3,"id":"workers","text":"Workers"},{"depth":3,"id":"bindings","text":"Bindings"},{"depth":3,"id":"policy","text":"Policy"},{"depth":3,"id":"testing","text":"Testing"}],
        textRoute: "/blog/introducing-typescriptpp.txt",
        title: "Introducing TypeScript++",
        tokens: 12036,
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

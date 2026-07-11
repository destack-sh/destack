export type Document = {
    description: string;
    markdownRoute: string;
    order: number;
    path: string;
    route: string;
    tableOfContents: readonly TableOfContentsEntry[];
    textRoute: string;
    title: string;
    tokens: number;
};

export type DocumentContent = {
    html: string;
};

export type TableOfContentsEntry = {
    depth: number;
    id: string;
    text: string;
};

export const documents = [
    {
        description: "The premise, ambitions, and open questions behind Destack.",
        markdownRoute: "/docs/index.md",
        order: 0,
        path: "index.md",
        route: "/docs/",
        tableOfContents: [{"depth":2,"id":"why-you-should-not-use-destack","text":"Why You Should Not Use Destack"},{"depth":2,"id":"some-questions-worth-considering","text":"Some Questions Worth Considering"}],
        textRoute: "/docs/index.txt",
        title: "Higher-Order Programming",
        tokens: 2465,
    },
    {
        description: "The language, toolchain, runtime, and the case for an integrated computing stack.",
        markdownRoute: "/docs/overview.md",
        order: 10,
        path: "overview.md",
        route: "/docs/overview/",
        tableOfContents: [{"depth":2,"id":"universality-and-completeness","text":"Universality and Completeness"}],
        textRoute: "/docs/overview.txt",
        title: "Overview",
        tokens: 606,
    },
    {
        description: "How Destack relates to adjacent languages, runtimes, and programming systems.",
        markdownRoute: "/docs/comparison.md",
        order: 11,
        path: "comparison.md",
        route: "/docs/comparison/",
        tableOfContents: [{"depth":2,"id":"typescript","text":"TypeScript"},{"depth":3,"id":"modules","text":"Modules"},{"depth":3,"id":"values","text":"Values"},{"depth":3,"id":"primitives","text":"Primitives"},{"depth":4,"id":"characters","text":"Characters"},{"depth":3,"id":"arithmetic","text":"Arithmetic"},{"depth":4,"id":"conversions","text":"Conversions"},{"depth":3,"id":"unknown","text":"Unknown"},{"depth":4,"id":"any-and-unknown","text":"Any and Unknown"},{"depth":3,"id":"shapes","text":"Shapes"},{"depth":4,"id":"type-vs-interface","text":"Type vs Interface"},{"depth":4,"id":"freshness","text":"Freshness"},{"depth":3,"id":"classes","text":"Classes"},{"depth":4,"id":"nominality","text":"Nominality"},{"depth":3,"id":"enums","text":"Enums"},{"depth":3,"id":"generics","text":"Generics"},{"depth":3,"id":"variance","text":"Variance"},{"depth":4,"id":"arrays","text":"Arrays"},{"depth":3,"id":"guards","text":"Guards"},{"depth":4,"id":"truthiness","text":"Truthiness"},{"depth":4,"id":"is","text":"Is"},{"depth":3,"id":"operators","text":"Operators"},{"depth":4,"id":"equality","text":"Equality"},{"depth":3,"id":"closures","text":"Closures"},{"depth":3,"id":"loops","text":"Loops"},{"depth":3,"id":"trees","text":"Trees"},{"depth":3,"id":"errors","text":"Errors"},{"depth":4,"id":"exceptions","text":"Exceptions"},{"depth":4,"id":"promises","text":"Promises"},{"depth":3,"id":"decorators","text":"Decorators"},{"depth":3,"id":"comptime","text":"Comptime"},{"depth":2,"id":"rust","text":"Rust"},{"depth":3,"id":"ownership","text":"Ownership"},{"depth":4,"id":"defaults","text":"Defaults"},{"depth":4,"id":"borrowing","text":"Borrowing"},{"depth":4,"id":"exclusivity","text":"Exclusivity"},{"depth":4,"id":"moves","text":"Moves"},{"depth":4,"id":"drop","text":"Drop"},{"depth":3,"id":"lifetimes","text":"Lifetimes"},{"depth":4,"id":"inference","text":"Inference"},{"depth":4,"id":"structs","text":"Structs"},{"depth":4,"id":"declarations","text":"Declarations"},{"depth":3,"id":"traits","text":"Traits"},{"depth":4,"id":"interfaces","text":"Interfaces"},{"depth":4,"id":"implementations","text":"Implementations"},{"depth":4,"id":"blankets","text":"Blankets"},{"depth":4,"id":"associated-types","text":"Associated Types"},{"depth":3,"id":"data","text":"Data"},{"depth":4,"id":"enums-2","text":"Enums"},{"depth":4,"id":"optionality","text":"Optionality"},{"depth":3,"id":"errors-2","text":"Errors"},{"depth":4,"id":"results","text":"Results"},{"depth":4,"id":"panics","text":"Panics"},{"depth":3,"id":"concurrency","text":"Concurrency"},{"depth":4,"id":"send-and-sync","text":"Send and Sync"},{"depth":4,"id":"tasks","text":"Tasks"},{"depth":4,"id":"workers","text":"Workers"},{"depth":3,"id":"generics-2","text":"Generics"},{"depth":4,"id":"monomorphization","text":"Monomorphization"},{"depth":4,"id":"constants","text":"Constants"},{"depth":4,"id":"erasure","text":"Erasure"},{"depth":4,"id":"variance-2","text":"Variance"},{"depth":3,"id":"unsafe","text":"Unsafe"},{"depth":3,"id":"comptime-2","text":"Comptime"},{"depth":4,"id":"macros","text":"Macros"},{"depth":4,"id":"const-functions","text":"Const Functions"},{"depth":2,"id":"flow","text":"Flow"},{"depth":3,"id":"objects","text":"Objects"},{"depth":4,"id":"exactness","text":"Exactness"},{"depth":3,"id":"nominality-2","text":"Nominality"},{"depth":4,"id":"classes-2","text":"Classes"},{"depth":4,"id":"opaque-types","text":"Opaque Types"},{"depth":3,"id":"variance-3","text":"Variance"},{"depth":4,"id":"sigils","text":"Sigils"},{"depth":3,"id":"predicates","text":"Predicates"},{"depth":4,"id":"guards-2","text":"Guards"},{"depth":2,"id":"assemblyscript","text":"AssemblyScript"},{"depth":3,"id":"numbers","text":"Numbers"},{"depth":4,"id":"naming","text":"Naming"},{"depth":4,"id":"overflow","text":"Overflow"},{"depth":3,"id":"expressiveness","text":"Expressiveness"},{"depth":4,"id":"closures-2","text":"Closures"},{"depth":4,"id":"unions","text":"Unions"},{"depth":3,"id":"memory","text":"Memory"},{"depth":4,"id":"values-2","text":"Values"},{"depth":4,"id":"equality-2","text":"Equality"},{"depth":2,"id":"c","text":"C#"},{"depth":3,"id":"values-3","text":"Values"},{"depth":4,"id":"structs-and-classes","text":"Structs and Classes"},{"depth":4,"id":"records","text":"Records"},{"depth":4,"id":"properties","text":"Properties"},{"depth":3,"id":"generics-3","text":"Generics"},{"depth":4,"id":"variance-4","text":"Variance"},{"depth":4,"id":"constraints","text":"Constraints"},{"depth":3,"id":"errors-3","text":"Errors"},{"depth":4,"id":"exceptions-2","text":"Exceptions"},{"depth":4,"id":"nullability","text":"Nullability"},{"depth":3,"id":"concurrency-2","text":"Concurrency"},{"depth":4,"id":"async","text":"Async"}],
        textRoute: "/docs/comparison.txt",
        title: "Comparison",
        tokens: 7702,
    },
    {
        description: "The Destack language and its relationship to strict modern TypeScript.",
        markdownRoute: "/docs/language/index.md",
        order: 20,
        path: "language/index.md",
        route: "/docs/language/",
        tableOfContents: [],
        textRoute: "/docs/language/index.txt",
        title: "Language",
        tokens: 269,
    },
    {
        description: "Primitives, nominal and value types, constraints, representation, and reflection.",
        markdownRoute: "/docs/language/types.md",
        order: 21,
        path: "language/types.md",
        route: "/docs/language/types/",
        tableOfContents: [{"depth":2,"id":"primitives","text":"Primitives"},{"depth":2,"id":"unknown","text":"Unknown"},{"depth":2,"id":"string","text":"String"},{"depth":2,"id":"intervals","text":"Intervals"},{"depth":2,"id":"newtypes","text":"Newtypes"},{"depth":2,"id":"newtype-interfaces","text":"Newtype Interfaces"},{"depth":2,"id":"extensions","text":"Extensions"},{"depth":3,"id":"blankets","text":"Blankets"},{"depth":2,"id":"enums","text":"Enums"},{"depth":2,"id":"tagged-unions","text":"Tagged Unions"},{"depth":2,"id":"structs","text":"Structs"},{"depth":2,"id":"classes","text":"Classes"},{"depth":2,"id":"arrays-slices-and-tuples","text":"Arrays, Slices and Tuples"},{"depth":2,"id":"readonly","text":"Readonly"},{"depth":2,"id":"generics","text":"Generics"},{"depth":2,"id":"variance","text":"Variance"},{"depth":2,"id":"static","text":"Static"},{"depth":2,"id":"associated-types-and-constants","text":"Associated Types and Constants"},{"depth":2,"id":"constraints","text":"Constraints"},{"depth":2,"id":"shapes","text":"Shapes"},{"depth":2,"id":"representation","text":"Representation"},{"depth":2,"id":"dynamic","text":"Dynamic"},{"depth":2,"id":"layout","text":"Layout"},{"depth":2,"id":"reflection","text":"Reflection"}],
        textRoute: "/docs/language/types.txt",
        title: "Types",
        tokens: 10906,
    },
    {
        description: "Values, control flow, dispatch, errors, metaprogramming, and modules.",
        markdownRoute: "/docs/language/expressions.md",
        order: 22,
        path: "language/expressions.md",
        route: "/docs/language/expressions/",
        tableOfContents: [{"depth":2,"id":"values","text":"Values"},{"depth":2,"id":"closures","text":"Closures"},{"depth":2,"id":"continuations","text":"Continuations"},{"depth":2,"id":"tasks","text":"Tasks"},{"depth":2,"id":"patterns","text":"Patterns"},{"depth":2,"id":"guards","text":"Guards"},{"depth":2,"id":"loops","text":"Loops"},{"depth":2,"id":"using","text":"Using"},{"depth":2,"id":"operators","text":"Operators"},{"depth":2,"id":"arithmetic","text":"Arithmetic"},{"depth":2,"id":"ranges","text":"Ranges"},{"depth":2,"id":"dispatch","text":"Dispatch"},{"depth":3,"id":"overloads","text":"Overloads"},{"depth":3,"id":"operators-2","text":"Operators"},{"depth":3,"id":"interfaces","text":"Interfaces"},{"depth":3,"id":"index-signatures","text":"Index Signatures"},{"depth":3,"id":"unions","text":"Unions"},{"depth":3,"id":"coherence","text":"Coherence"},{"depth":2,"id":"errors","text":"Errors"},{"depth":3,"id":"error","text":"Error"},{"depth":3,"id":"result","text":"Result"},{"depth":3,"id":"maybe-must-and-coalesce","text":"Maybe, Must and Coalesce"},{"depth":3,"id":"try","text":"Try"},{"depth":3,"id":"try-catch-finally","text":"Try-Catch-Finally"},{"depth":3,"id":"panics","text":"Panics"},{"depth":2,"id":"trees-tsx","text":"Trees (TSX)"},{"depth":2,"id":"decorators","text":"Decorators"},{"depth":3,"id":"diagnostics","text":"Diagnostics"},{"depth":3,"id":"restrictions","text":"Restrictions"},{"depth":3,"id":"taint","text":"Taint"},{"depth":3,"id":"derive","text":"Derive"},{"depth":3,"id":"static-if","text":"Static If"},{"depth":2,"id":"module","text":"Module"},{"depth":2,"id":"globals","text":"Globals"},{"depth":2,"id":"comptime","text":"Comptime"},{"depth":3,"id":"dynamic-code","text":"Dynamic Code"},{"depth":2,"id":"macros","text":"Macros"}],
        textRoute: "/docs/language/expressions.txt",
        title: "Expressions",
        tokens: 13061,
    },
    {
        description: "Ownership, borrowing, lifetimes, allocation, capabilities, and synchronization.",
        markdownRoute: "/docs/language/memory.md",
        order: 23,
        path: "language/memory.md",
        route: "/docs/language/memory/",
        tableOfContents: [{"depth":2,"id":"ownership","text":"Ownership"},{"depth":2,"id":"borrowing","text":"Borrowing"},{"depth":3,"id":"stability","text":"Stability"},{"depth":3,"id":"rooting","text":"Rooting"},{"depth":3,"id":"definite-assignment","text":"Definite Assignment"},{"depth":2,"id":"lifetimes","text":"Lifetimes"},{"depth":2,"id":"suspension","text":"Suspension"},{"depth":2,"id":"drop","text":"Drop"},{"depth":2,"id":"allocation","text":"Allocation"},{"depth":2,"id":"space","text":"Space"},{"depth":3,"id":"shared-space","text":"Shared Space"},{"depth":3,"id":"static-space","text":"Static Space"},{"depth":2,"id":"conversions","text":"Conversions"},{"depth":2,"id":"unsafe","text":"Unsafe"},{"depth":2,"id":"algebra","text":"Algebra"},{"depth":2,"id":"polymorphism","text":"Polymorphism"},{"depth":2,"id":"capabilities","text":"Capabilities"},{"depth":2,"id":"synchronization","text":"Synchronization"}],
        textRoute: "/docs/language/memory.txt",
        title: "Memory",
        tokens: 8967,
    },
    {
        description: "Execution, effects, observability, and the integrated Destack runtime.",
        markdownRoute: "/docs/runtime/index.md",
        order: 30,
        path: "runtime/index.md",
        route: "/docs/runtime/",
        tableOfContents: [],
        textRoute: "/docs/runtime/index.txt",
        title: "Runtime",
        tokens: 121,
    },
    {
        description: "Conditional sources, typed modules, metadata, and import attributes.",
        markdownRoute: "/docs/runtime/modules.md",
        order: 31,
        path: "runtime/modules.md",
        route: "/docs/runtime/modules/",
        tableOfContents: [{"depth":2,"id":"conditions","text":"Conditions"},{"depth":2,"id":"import-meta","text":"Import Meta"},{"depth":2,"id":"data-modules","text":"Data Modules"},{"depth":2,"id":"text-modules","text":"Text Modules"},{"depth":2,"id":"binary-modules","text":"Binary Modules"},{"depth":2,"id":"import-attributes","text":"Import Attributes"}],
        textRoute: "/docs/runtime/modules.txt",
        title: "Modules",
        tokens: 1689,
    },
    {
        description: "Runtime policy and controlled interaction with the host.",
        markdownRoute: "/docs/runtime/policy.md",
        order: 32,
        path: "runtime/policy.md",
        route: "/docs/runtime/policy/",
        tableOfContents: [],
        textRoute: "/docs/runtime/policy.txt",
        title: "Policy",
        tokens: 226,
    }
] as const satisfies readonly Document[];

export const documentByRoute: ReadonlyMap<string, Document> = new Map(
    documents.map((document): [string, Document] => [document.route, document]),
);

const documentLoaders: Record<string, () => Promise<{ default: DocumentContent }>> = {
    "/docs/": () => import("./document/index"),
    "/docs/overview/": () => import("./document/overview"),
    "/docs/comparison/": () => import("./document/comparison"),
    "/docs/language/": () => import("./document/language_index"),
    "/docs/language/types/": () => import("./document/language_types"),
    "/docs/language/expressions/": () => import("./document/language_expressions"),
    "/docs/language/memory/": () => import("./document/language_memory"),
    "/docs/runtime/": () => import("./document/runtime_index"),
    "/docs/runtime/modules/": () => import("./document/runtime_modules"),
    "/docs/runtime/policy/": () => import("./document/runtime_policy"),
};

/// Load one rendered document body by canonical route.
export async function loadDocument(route: string): Promise<DocumentContent | undefined> {
    return documentLoaders[route]?.().then((module) => module.default);
}

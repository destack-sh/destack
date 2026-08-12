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
        description: "The language, toolchain, runtime, and the case for an integrated computing stack.",
        markdownRoute: "/docs/overview.md",
        order: 10,
        path: "overview.md",
        route: "/docs/overview/",
        tableOfContents: [{"depth":2,"id":"universality-and-completeness","text":"Universality and Completeness"}],
        textRoute: "/docs/overview.txt",
        title: "Overview",
        tokens: 604,
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
        tableOfContents: [{"depth":2,"id":"primitives","text":"Primitives"},{"depth":2,"id":"unknown","text":"Unknown"},{"depth":2,"id":"string","text":"String"},{"depth":2,"id":"intervals","text":"Intervals"},{"depth":2,"id":"newtypes","text":"Newtypes"},{"depth":2,"id":"newtype-interfaces","text":"Newtype Interfaces"},{"depth":2,"id":"extensions","text":"Extensions"},{"depth":3,"id":"blankets","text":"Blankets"},{"depth":2,"id":"enums","text":"Enums"},{"depth":2,"id":"tagged-unions","text":"Tagged Unions"},{"depth":2,"id":"structs","text":"Structs"},{"depth":2,"id":"classes","text":"Classes"},{"depth":2,"id":"arrays-slices-and-tuples","text":"Arrays, Slices and Tuples"},{"depth":2,"id":"readonly","text":"Readonly"},{"depth":2,"id":"generics","text":"Generics"},{"depth":2,"id":"boundaries","text":"Boundaries"},{"depth":2,"id":"variance","text":"Variance"},{"depth":2,"id":"static","text":"Static"},{"depth":2,"id":"associated-types-and-constants","text":"Associated Types and Constants"},{"depth":2,"id":"constraints","text":"Constraints"},{"depth":2,"id":"representation","text":"Representation"},{"depth":2,"id":"layout","text":"Layout"},{"depth":2,"id":"reflection","text":"Reflection"}],
        textRoute: "/docs/language/types.txt",
        title: "Types",
        tokens: 9589,
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
        tokens: 13447,
    },
    {
        description: "Ownership, borrowing, lifetimes, allocation, capabilities, and synchronization.",
        markdownRoute: "/docs/language/memory.md",
        order: 23,
        path: "language/memory.md",
        route: "/docs/language/memory/",
        tableOfContents: [{"depth":2,"id":"ownership","text":"Ownership"},{"depth":2,"id":"borrowing","text":"Borrowing"},{"depth":3,"id":"stability","text":"Stability"},{"depth":3,"id":"rooting","text":"Rooting"},{"depth":3,"id":"definite-assignment","text":"Definite Assignment"},{"depth":2,"id":"lifetimes","text":"Lifetimes"},{"depth":2,"id":"suspension","text":"Suspension"},{"depth":2,"id":"drop","text":"Drop"},{"depth":2,"id":"allocation","text":"Allocation"},{"depth":2,"id":"space","text":"Space"},{"depth":3,"id":"shared-space","text":"Shared Space"},{"depth":3,"id":"shared-module-bindings","text":"Shared Module Bindings"},{"depth":2,"id":"conversions","text":"Conversions"},{"depth":2,"id":"unsafe","text":"Unsafe"},{"depth":2,"id":"algebra","text":"Algebra"},{"depth":2,"id":"polymorphism","text":"Polymorphism"},{"depth":2,"id":"capabilities","text":"Capabilities"},{"depth":2,"id":"synchronization","text":"Synchronization"}],
        textRoute: "/docs/language/memory.txt",
        title: "Memory",
        tokens: 9043,
    },
    {
        description: "Execution, effects, observability, and the integrated Destack runtime.",
        markdownRoute: "/docs/language/runtime/index.md",
        order: 30,
        path: "language/runtime/index.md",
        route: "/docs/language/runtime/",
        tableOfContents: [],
        textRoute: "/docs/language/runtime/index.txt",
        title: "Runtime",
        tokens: 121,
    },
    {
        description: "Conditional sources, typed modules, metadata, and import attributes.",
        markdownRoute: "/docs/language/runtime/modules.md",
        order: 31,
        path: "language/runtime/modules.md",
        route: "/docs/language/runtime/modules/",
        tableOfContents: [{"depth":2,"id":"conditions","text":"Conditions"},{"depth":2,"id":"import-meta","text":"Import Meta"},{"depth":2,"id":"data-modules","text":"Data Modules"},{"depth":2,"id":"text-modules","text":"Text Modules"},{"depth":2,"id":"binary-modules","text":"Binary Modules"},{"depth":2,"id":"import-attributes","text":"Import Attributes"}],
        textRoute: "/docs/language/runtime/modules.txt",
        title: "Modules",
        tokens: 1691,
    },
    {
        description: "Runtime policy and controlled interaction with the host.",
        markdownRoute: "/docs/language/runtime/policy.md",
        order: 32,
        path: "language/runtime/policy.md",
        route: "/docs/language/runtime/policy/",
        tableOfContents: [],
        textRoute: "/docs/language/runtime/policy.txt",
        title: "Policy",
        tokens: 226,
    }
] as const satisfies readonly Document[];

export const documentByRoute: ReadonlyMap<string, Document> = new Map(
    documents.map((document): [string, Document] => [document.route, document]),
);

const documentLoaders: Record<string, () => Promise<{ default: DocumentContent }>> = {
    "/docs/overview/": () => import("./document/overview"),
    "/docs/language/": () => import("./document/language_index"),
    "/docs/language/types/": () => import("./document/language_types"),
    "/docs/language/expressions/": () => import("./document/language_expressions"),
    "/docs/language/memory/": () => import("./document/language_memory"),
    "/docs/language/runtime/": () => import("./document/language_runtime_index"),
    "/docs/language/runtime/modules/": () => import("./document/language_runtime_modules"),
    "/docs/language/runtime/policy/": () => import("./document/language_runtime_policy"),
};

/// Load one rendered document body by canonical route.
export async function loadDocument(route: string): Promise<DocumentContent | undefined> {
    return documentLoaders[route]?.().then((module) => module.default);
}

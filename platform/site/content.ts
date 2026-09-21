/// A published collection, its sources, and its optional header shortcut.
export type Collection = {
    /// The visible collection name.
    title: string;
    /// Whether this collection appears in navigation and discovery.
    isListed?: boolean;
    /// The canonical collection root.
    route: string;
    /// The header keyboard shortcut.
    shortcut?: string;
    /// The repository directories published in this collection.
    sources: readonly {
        directory: string;
        readme?: boolean;
        path: string;
        hierarchy: readonly number[];
    }[];
};

/// The collections published by the site.
export const collections: readonly Collection[] = [
    {
        title: "Docs",
        route: "/docs/",
        shortcut: "d",
        sources: [{ directory: "docs", path: "", hierarchy: [] }],
    },
    {
        title: "Language",
        isListed: false,
        route: "/docs/language/",
        shortcut: "l",
        sources: [
            { directory: "language/docs", path: "language", hierarchy: [10] },
            {
                directory: "language/library",
                readme: true,
                path: "language/standard-library",
                hierarchy: [10, 30],
            },
        ],
    },
    {
        title: "Libraries",
        route: "/docs/library/",
        sources: [{ directory: "library/docs", path: "library", hierarchy: [20] }],
    },
    {
        title: "Templates",
        route: "/docs/template/",
        shortcut: "t",
        sources: [
            { directory: "template/docs", path: "template", hierarchy: [50] },
            { directory: "template/stack/docs", path: "template/stack", hierarchy: [50, 10] },
            { directory: "template/blank/docs", path: "template/blank", hierarchy: [50, 20] },
        ],
    },
    {
        title: "Blog",
        route: "/blog/",
        shortcut: "b",
        sources: [{ directory: "blog", path: "", hierarchy: [] }],
    },
];

/// Find the most specific collection containing a route.
export function collectionAt(route: string): Collection | undefined {
    let match: Collection | undefined;

    // prefer the deepest matching collection
    for (const collection of collections) {
        if (
            route.startsWith(collection.route) &&
            (match == undefined || collection.route.length > match.route.length)
        ) {
            match = collection;
        }
    }

    return match;
}

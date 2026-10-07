/** A published collection and its sources. */
export type Collection = {
    /** The visible collection name. */
    title: string;
    /** The canonical collection root. */
    route: string;
    /** The repository directories published in this collection. */
    sources: readonly {
        directory: string;
        path: string;
        hierarchy: readonly number[];
    }[];
};

/** The collections published by the site. */
export const collections: readonly Collection[] = [
    {
        title: "Documentation",
        route: "/docs/",
        sources: [{ directory: "@platform/docs", path: "", hierarchy: [] }],
    },
    {
        title: "Blog",
        route: "/blog/",
        sources: [{ directory: "@platform/blog", path: "", hierarchy: [] }],
    },
];

/** Find the most specific collection containing a route. */
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

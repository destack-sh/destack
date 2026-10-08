import type { NavigationPage } from "./page.ts";
import { collectionAt, type Collection } from "../content.ts";

/** Generate chapter navigation and ancestry. */
export function buildNavigation(documents: NavigationPage[]) {
    // index pages by route and select the chapters
    const byRoute = new Map(documents.map((page) => [page.route, page]));
    const chapters = documents.filter((page) => page.kind === "chapter");

    // resolve each parent from published routes
    for (const page of documents) {
        const parentRoute = page.route.replace(/[^/]+\/$/u, "");
        const parent = page.route === "/docs/" ? undefined : byRoute.get(parentRoute);
        if (page.route !== "/docs/" && parent == undefined) {
            throw new Error(`missing parent ${parentRoute} for ${page.route}`);
        }
        if (parent != undefined && (parent === page || !page.route.startsWith(parent.route))) {
            throw new Error(`invalid parent ${parent.route} for ${page.route}`);
        }
        if (parent != undefined) {
            page.parent = parent;
        }
    }

    // resolve ancestry once before selecting navigation entries
    const ancestry = new Map<
        NavigationPage,
        { ancestors: NavigationPage[]; collection: Collection }
    >();
    for (const page of documents) {
        const ancestors: NavigationPage[] = [];
        let parent = page.parent;
        while (parent != undefined) {
            ancestors.unshift(parent);
            parent = parent.parent;
        }
        const collection = collectionAt(page.route);
        if (collection == undefined || !byRoute.has(collection.route)) {
            throw new Error(`missing collection for ${page.route}`);
        }
        ancestry.set(page, { ancestors, collection });
    }

    /** Read the resolved ancestry of one document. */
    function ancestryOf(page: NavigationPage) {
        const entry = ancestry.get(page);
        if (entry == undefined) {
            throw new Error(`missing ancestry for ${page.route}`);
        }

        return entry;
    }

    // index document children in their published order
    const children = new Map<NavigationPage | undefined, NavigationPage[]>();
    for (const document of documents) {
        const siblings = children.get(document.parent) ?? [];
        siblings.push(document);
        children.set(document.parent, siblings);
    }

    // expand every document beneath the active section
    for (const page of documents) {
        const { ancestors, collection } = ancestryOf(page);
        const root = byRoute.get(collection.route);
        if (root == undefined) {
            throw new Error(`missing collection root ${collection.route} for ${page.route}`);
        }
        const rootDepth = ancestryOf(root).ancestors.length;
        const chain = [...ancestors, page];
        const section = chain[chain.indexOf(root) + 1];
        const visible: NavigationPage[] = [];
        function visit(document: NavigationPage) {
            visible.push(document);
            if (
                document === section ||
                (section != undefined && ancestryOf(document).ancestors.includes(section))
            ) {
                for (const child of children.get(document) ?? []) {
                    visit(child);
                }
            }
        }
        for (const document of children.get(root) ?? []) {
            visit(document);
        }

        // keep pagination within the authored collection
        const sequence = chapters.filter(
            (chapter) => ancestryOf(chapter).collection === collection,
        );
        const index = sequence.indexOf(page);
        const previous = index > 0 ? sequence[index - 1] : undefined;
        const next = index >= 0 ? sequence[index + 1] : undefined;
        page.navigation = {
            root: link(root),
            ancestors: ancestors.map(link),
            entries: visible.map((chapter) => ({
                ...link(chapter),
                depth: ancestryOf(chapter).ancestors.length - rootDepth - 1,
            })),
            ...(previous == undefined ? {} : { previous: link(previous) }),
            ...(next == undefined ? {} : { next: link(next) }),
        };
    }
}

/** Keep the title and route needed by a navigation link. */
function link(page: NavigationPage) {
    return { title: page.title, route: page.route };
}

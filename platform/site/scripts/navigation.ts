import type { NavigationPage } from "./page";
import { collectionAt } from "../content.ts";

/// Generate chapter navigation and reference ancestry.
export function buildNavigation(documents: NavigationPage[], references: NavigationPage[]) {
    const pages = [...documents, ...references];
    const byRoute = new Map(pages.map((page) => [page.route, page]));
    const chapters = documents.filter((page) => page.kind === "chapter");

    // resolve each parent from published routes, including symbol module parents
    for (const page of pages) {
        const parentRoute =
            page.parentRoute ?? page.moduleRoute ?? page.route.replace(/[^/]+\/$/, "");
        const parent =
            page.route === "/docs/" ? undefined : byRoute.get(parentRoute);
        if (page.route !== "/docs/" && parent == undefined) {
            throw new Error(`missing parent ${parentRoute} for ${page.route}`);
        }
        if (parent != undefined && (parent === page || !page.route.startsWith(parent.route))) {
            throw new Error(`invalid parent ${parent.route} for ${page.route}`);
        }
        page.parent = parent;
    }

    // resolve ancestry once before selecting navigation entries
    for (const page of pages) {
        page.ancestors = [];
        let parent = page.parent;
        while (parent != undefined) {
            page.ancestors.unshift(parent);
            parent = parent.parent;
        }
        page.collection = collectionAt(page.route);
        if (
            page.collection == undefined ||
            !byRoute.has(page.collection.route)
        ) {
            throw new Error(`missing collection for ${page.route}`);
        }
    }

    // index document children in their published order
    const children = new Map<NavigationPage | undefined, NavigationPage[]>();
    for (const document of documents) {
        if (!children.has(document.parent)) children.set(document.parent, []);
        children.get(document.parent)!.push(document);
    }

    // expand every document beneath the active section
    for (const page of pages) {
        const collection = page.collection!;
        const root = byRoute.get(collection.route)!;
        const chain = [...page.ancestors!, page];
        const section = chain[chain.indexOf(root) + 1];
        const visible: NavigationPage[] = [];
        function visit(document: NavigationPage) {
            if (document.collection?.isListed === false && document.collection !== collection) return;
            visible.push(document);
            if (document === section || document.ancestors!.includes(section)) {
                for (const child of children.get(document) ?? []) visit(child);
            }
        }
        for (const document of children.get(root) ?? []) visit(document);

        // keep pagination within the authored collection
        const sequence = chapters.filter(
            (chapter) => chapter.collection === collection,
        );
        const index = sequence.indexOf(page);
        page.navigation = {
            root: link(root),
            ancestors: page.ancestors!.map(link),
            entries: visible.map((chapter) => ({
                ...link(chapter),
                depth: chapter.ancestors!.length - root.ancestors!.length - 1,
            })),
            previous: index > 0 ? link(sequence[index - 1]) : undefined,
            next:
                index >= 0 && index + 1 < sequence.length
                    ? link(sequence[index + 1])
                    : undefined,
        };
    }
}

/// Keep the title and route needed by a navigation link.
function link(page: NavigationPage) {
    return { title: page.title, route: page.route };
}

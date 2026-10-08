import type { Root } from "hast";
import { loadAsset } from "./asset.ts";

/** One rendered content body. */
export type RenderedContent = {
    /** The body's tree, as remark and rehype built it from its Markdown. */
    tree: Root;
};

/** Load one rendered content body from its static route. */
export async function loadContent(route: string): Promise<RenderedContent> {
    if (!route.startsWith("/_content/") || route.includes("..")) {
        throw new Error(`invalid rendered content route: ${route}`);
    }

    // load the body's tree, failing on a missing or malformed asset
    const source = await loadAsset(route);
    if (source === undefined) {
        throw new Error(`missing rendered content: ${route}`);
    }
    const tree: unknown = JSON.parse(source);
    if (!isRoot(tree)) {
        throw new Error(`rendered content is no tree: ${route}`);
    }

    return { tree };
}

/** Check that a parsed body is a tree's root, which the build writes for every page. */
function isRoot(value: unknown): value is Root {
    return (
        typeof value === "object" &&
        value !== null &&
        "type" in value &&
        value.type === "root" &&
        "children" in value &&
        Array.isArray(value.children)
    );
}

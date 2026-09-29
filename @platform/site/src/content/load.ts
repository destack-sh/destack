import { loadAsset } from "./asset.ts";

/** One rendered content body. */
export type RenderedContent = {
    /** The rendered HTML. */
    html: string;
};

/** Load one rendered content body from its static route. */
export async function loadContent(route: string): Promise<RenderedContent> {
    if (!route.startsWith("/_content/") || route.includes("..")) {
        throw new Error(`invalid rendered content route: ${route}`);
    }

    // load the rendered HTML, failing on a missing asset
    const html = await loadAsset(route);
    if (html === undefined) {
        throw new Error(`Missing rendered content: ${route}`);
    }

    return { html };
}

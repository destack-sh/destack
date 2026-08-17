/// One rendered content body.
export type RenderedContent = {
    /// The rendered HTML.
    html: string;
};

/// Load one rendered content body from its static route.
export async function loadContent(route: string): Promise<RenderedContent> {
    if (!route.startsWith("/_content/") || route.includes("..")) {
        throw new Error(`invalid rendered content route: ${route}`);
    }

    // read the same public artifact directly while rendering on the server
    if (import.meta.env.SSR) {
        const [{ readFile }, { join }] = await Promise.all([
            import("node:fs/promises"),
            import("node:path"),
        ]);
        const file = join(process.cwd(), "public", route.slice(1));

        return { html: await readFile(file, "utf8") };
    }

    // fetch the independently cached artifact during browser navigation
    const response = await fetch(route);
    if (!response.ok) {
        throw new Error(`cannot load rendered content (${response.status}): ${route}`);
    }

    return { html: await response.text() };
}

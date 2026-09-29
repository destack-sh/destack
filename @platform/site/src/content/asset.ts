/** Read generated content from the server bundle or its public browser URL. */
export async function loadAsset(route: string): Promise<string | undefined> {
    if (!route.startsWith("/_content/") || route.includes("..")) {
        throw new Error(`Invalid content route: ${route}`);
    }

    // retain generated content in server builds without a filesystem dependency
    if (import.meta.env.SSR) {
        const { assets } = await import("../generated/assets.ts");
        const load = assets[route];

        return load ? await load() : undefined;
    }

    // fetch the independently cached public artifact
    const response = await fetch(route);
    if (response.status === 404) {
        return undefined;
    }
    if (!response.ok) {
        throw new Error(`Cannot load content (${response.status}): ${route}`);
    }

    return response.text();
}

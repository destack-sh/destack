import { loadContent } from "./load";
import type { Document, DocumentContent } from "../generated/documents";

const libraryRoute = "/docs/language/library/";

/// One generated library item and its rendered body.
export type LibraryItem = {
    /// The generated documentation metadata.
    document: Document;

    /// The rendered item body.
    content: DocumentContent;
};

/// Load one generated library item from its static artifacts.
export async function loadLibraryItem(route: string): Promise<LibraryItem | undefined> {
    if (!route.startsWith(libraryRoute) || route.includes("..")) {
        return undefined;
    }
    const metadataRoute = `/_content${route}index.json`;
    const document = await loadLibraryItemMetadata(metadataRoute);
    if (document == undefined) {
        return undefined;
    }
    if (document.route !== route) {
        throw new Error(`invalid library item route: ${document.route}`);
    }
    const content = await loadContent(document.contentRoute);

    return { content, document };
}

/// Load and validate one generated library item metadata record.
async function loadLibraryItemMetadata(route: string): Promise<Document | undefined> {
    let value: unknown;

    // read the same public artifact directly while rendering on the server
    if (import.meta.env.SSR) {
        const [{ readFile }, { join }] = await Promise.all([
            import("node:fs/promises"),
            import("node:path"),
        ]);
        const file = join(process.cwd(), "public", route.slice(1));
        try {
            value = JSON.parse(await readFile(file, "utf8"));
        } catch (error) {
            if (isMissingFileError(error)) {
                return undefined;
            }
            throw error;
        }
    }
    // fetch the independently cached artifact during browser navigation
    else {
        const response = await fetch(route);
        if (response.status === 404) {
            return undefined;
        }
        if (!response.ok) {
            throw new Error(`cannot load library item (${response.status}): ${route}`);
        }
        value = await response.json();
    }

    if (!isDocument(value)) {
        throw new Error(`invalid library item metadata: ${route}`);
    }

    return value;
}

/// Return whether one error reports a missing filesystem path.
function isMissingFileError(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}

/// Return whether one value is complete generated document metadata.
function isDocument(value: unknown): value is Document {
    if (typeof value !== "object" || value == null) {
        return false;
    }
    const document = value as Record<string, unknown>;
    const hasModule = (
        document.moduleRoute == undefined && document.moduleTitle == undefined
    ) || (
        typeof document.moduleRoute === "string" && typeof document.moduleTitle === "string"
    );

    return (
        typeof document.contentRoute === "string" &&
        typeof document.description === "string" &&
        (document.lead == undefined || typeof document.lead === "string") &&
        typeof document.markdownRoute === "string" &&
        hasModule &&
        typeof document.order === "number" &&
        typeof document.path === "string" &&
        typeof document.route === "string" &&
        Array.isArray(document.tableOfContents) &&
        typeof document.textRoute === "string" &&
        typeof document.title === "string" &&
        typeof document.tokens === "number"
    );
}

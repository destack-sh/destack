import type { ContentEntry } from "./presentation.ts";
import { loadContent, type RenderedContent } from "./load.ts";
import { loadAsset } from "./asset.ts";

/** One published documentation page. */
export type Document = {
    /** Immediate collection entries. */
    entries?: readonly ContentEntry[];
    /** The static rendered HTML route. */
    contentRoute: string;
    /** The concise chapter description. */
    description: string;
    /** The optional introductory sentence. */
    lead?: string;
    /** The authored Markdown route. */
    markdownRoute: string;
    /** The document category. */
    kind: "chapter" | "catalog";
    /** The generated chapter links. */
    navigation: {
        root: DocumentLink;
        ancestors: readonly DocumentLink[];
        entries: readonly (DocumentLink & { depth: number })[];
        previous?: DocumentLink;
        next?: DocumentLink;
    };
    /** The canonical browser route. */
    route: string;
    /** The rendered heading tree. */
    tableOfContents: readonly TableOfContentsEntry[];
    /** The plain text route. */
    textRoute: string;
    /** The chapter title. */
    title: string;
    /** The approximate token count. */
    tokens: number;
};

/** A published document destination. */
export type DocumentLink = {
    /** The visible title. */
    title: string;
    /** The canonical route. */
    route: string;
};

/** One rendered document body. */
export type DocumentContent = RenderedContent;

/** One rendered document heading. */
export type TableOfContentsEntry = {
    /** The heading depth. */
    depth: number;
    /** The heading fragment identifier. */
    id: string;
    /** The heading text. */
    text: string;
};

/** One document and its rendered body. */
export type LoadedDocument = {
    /** The generated documentation metadata. */
    document: Document;

    /** The rendered item body. */
    content: DocumentContent;
};

/** Load one published document and its rendered body. */
export async function loadDocument(route: string): Promise<LoadedDocument | undefined> {
    if (!route.startsWith("/docs/") || route.includes("..")) {
        return undefined;
    }
    const metadataRoute = `/_content${route}index.json`;
    const document = await loadDocumentMetadata(metadataRoute);
    if (document == undefined) {
        return undefined;
    }
    if (document.route !== route) {
        throw new Error(`invalid document route: ${document.route}`);
    }
    const content = await loadContent(document.contentRoute);

    return { content, document };
}

/** Load and validate one document metadata record. */
async function loadDocumentMetadata(route: string): Promise<Document | undefined> {
    // load the metadata, or none for a missing route
    const text = await loadAsset(route);
    if (text === undefined) {
        return undefined;
    }
    const value: unknown = JSON.parse(text);

    // reject metadata of the wrong shape
    if (!isDocument(value)) {
        throw new Error(`invalid document metadata: ${route}`);
    }

    return value;
}

/** Return whether one value is complete generated document metadata. */
function isDocument(value: unknown): value is Document {
    if (typeof value !== "object" || value == null) {
        return false;
    }
    const document = value as Record<string, unknown>;

    return (
        typeof document.contentRoute === "string" &&
        typeof document.description === "string" &&
        (document.lead == undefined || typeof document.lead === "string") &&
        typeof document.markdownRoute === "string" &&
        (document.kind === "chapter" || document.kind === "catalog") &&
        typeof document.navigation === "object" &&
        document.navigation != null &&
        typeof document.route === "string" &&
        Array.isArray(document.tableOfContents) &&
        typeof document.textRoute === "string" &&
        typeof document.title === "string" &&
        typeof document.tokens === "number"
    );
}

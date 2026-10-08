import { schema } from "@destack/schema";

import { ContentEntry } from "./presentation.ts";
import { loadContent, type RenderedContent } from "./load.ts";
import { loadAsset } from "./asset.ts";

/** A published document destination. */
export const DocumentLink = schema.object({
    /** The visible title. */
    title: schema.string(),
    /** The canonical route. */
    route: schema.string(),
});
/** A published document destination. */
export type DocumentLink = schema.Infer<typeof DocumentLink>;

/** One rendered document heading. */
export const TableOfContentsEntry = schema.object({
    /** The heading depth. */
    depth: schema.number(),
    /** The heading fragment identifier. */
    id: schema.string(),
    /** The heading text. */
    text: schema.string(),
});
/** One rendered document heading. */
export type TableOfContentsEntry = schema.Infer<typeof TableOfContentsEntry>;

/** One published documentation page. */
export const Document = schema.object({
    /** Immediate collection entries. */
    entries: schema.array(ContentEntry).exactOptional(),
    /** The static rendered HTML route. */
    contentRoute: schema.string(),
    /** The concise chapter description. */
    description: schema.string(),
    /** The optional introductory sentence. */
    lead: schema.string().exactOptional(),
    /** The authored Markdown route. */
    markdownRoute: schema.string(),
    /** The document category. */
    kind: schema.enum(["chapter", "catalog"]),
    /** The generated chapter links. */
    navigation: schema.object({
        /** The collection root. */
        root: DocumentLink,
        /** The chapters above this one, from the root down. */
        ancestors: schema.array(DocumentLink),
        /** The collection's chapters, each at its depth below the root. */
        entries: schema.array(DocumentLink.extend({ depth: schema.number() })),
        /** The chapter before this one. */
        previous: DocumentLink.exactOptional(),
        /** The chapter after this one. */
        next: DocumentLink.exactOptional(),
    }),
    /** The canonical browser route. */
    route: schema.string(),
    /** The rendered heading tree. */
    tableOfContents: schema.array(TableOfContentsEntry),
    /** The plain text route. */
    textRoute: schema.string(),
    /** The chapter title. */
    title: schema.string(),
    /** The approximate token count. */
    tokens: schema.number(),
});
/** One published documentation page. */
export type Document = schema.Infer<typeof Document>;

/** One rendered document body. */
export type DocumentContent = RenderedContent;

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

    // reject metadata of the wrong shape
    return Document.parse(JSON.parse(text));
}

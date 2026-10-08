import type { Root } from "hast";
import type { ContentEntry } from "../src/view/content/presentation.ts";
import type { Document } from "../src/view/content/document.ts";
import type { ContentAsset, headingsFor, searchSectionsFor } from "./markdown.ts";

/** A destination while its collection navigation is being assembled. */
export type NavigationPage = {
    route: string;
    title: string;
    kind?: string;
    parent?: NavigationPage;
    navigation?: Document["navigation"];
};

/** Common rendered content consumed by publication and search. */
export type RenderedPage = NavigationPage & {
    tree: Root;
    markdown: string;
    markdownRoute: string;
    textRoute: string;
    assets: ContentAsset[];
    file: string;
    tokens: number;
    headings: ReturnType<typeof headingsFor>;
    tableOfContents: ReturnType<typeof headingsFor>;
    searchSections: ReturnType<typeof searchSectionsFor>;
    searchText: string;
};

/** An authored documentation page. */
export type DocumentationPage = RenderedPage & {
    description: string;
    lead?: string;
    path: string;
    order: number;
    entries?: ContentEntry[];
    directory?: string;
};

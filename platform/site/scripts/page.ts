import type { Collection } from "../content";
import type { ContentEntry } from "../src/content/presentation";
import type { Document } from "../src/content/document";
import type { ContentAsset, headingsFor, searchSectionsFor } from "./markdown";

/// A destination while its collection navigation is being assembled.
export type NavigationPage = {
    route: string;
    title: string;
    kind?: string;
    /// A warning inherited by descendant pages.
    warning?: string;
    parentRoute?: string;
    moduleRoute?: string;
    parent?: NavigationPage;
    ancestors?: NavigationPage[];
    collection?: Collection;
    navigation?: Document["navigation"];
};

/// Common rendered content consumed by publication and search.
export type RenderedPage = NavigationPage & {
    html: string;
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
    searchKind?: string;
    searchContext?: string;
};

/// A documentation page, including generated package and rule references.
export type DocumentationPage = RenderedPage & {
    description: string;
    lead?: string;
    path: string;
    order: number;
    entries?: ContentEntry[];
    directory?: string;
};

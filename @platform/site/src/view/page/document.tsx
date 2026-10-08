import { createMemo, Show } from "@destack/view";
import { query, type RoutePreloadFuncArgs, type RouteSectionProps } from "@destack/view/router";
import { Metadata } from "@destack/view/document";

import { type LoadedDocument, loadDocument } from "../content/document";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "./missing";

/** Load a document and its rendered body once per route, for the preload and the page alike. */
const document = query(loadDocument, "document");

/** Load the requested document while the route is preloading. */
export function preload(arguments_: RoutePreloadFuncArgs): Promise<LoadedDocument | undefined> {
    return document(routeOf(arguments_.params["path"]));
}

/** Render a documentation chapter or index, or the missing document page for an unknown path. */
export function Document(properties: RouteSectionProps<Promise<LoadedDocument | undefined>>) {
    const loaded = createMemo(() => properties.data);

    return (
        <Show
            keyed
            when={loaded()}
            fallback={
                <MissingPage
                    backHref="/docs/"
                    backLabel="back to docs"
                    description="This documentation page does not exist."
                    label="missing document"
                    title="this page does not exist"
                />
            }
        >
            {(found) => (
                <>
                    <Metadata
                        title={found.document.title}
                        description={found.document.description}
                        alternates={{
                            canonical: found.document.route,
                            types: {
                                "text/markdown": found.document.markdownRoute,
                                "text/plain": found.document.textRoute,
                            },
                        }}
                    />
                    <DocumentArticle content={found.content} document={found.document} />
                </>
            )}
        </Show>
    );
}

/** Read the canonical document route of a path below `/docs`, the overview for none. */
function routeOf(path: string | undefined): string {
    const trimmed = path?.replace(/^\/+|\/+$/gu, "") ?? "";

    return trimmed === "" ? "/docs/" : `/docs/${trimmed}/`;
}

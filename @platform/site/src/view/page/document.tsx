import { Show } from "@destack/view";
import type { RouteSectionProps } from "@destack/view/router";
import { Metadata } from "@destack/view/document";

import { documentAt } from "../content/document";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "./missing";

/** Render a documentation chapter or index, or the missing document page for an unknown path. */
export function Document(properties: RouteSectionProps) {
    const loaded = () => documentAt(routeOf(properties.params["path"]));

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

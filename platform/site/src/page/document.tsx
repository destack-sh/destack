import { createResource, Show, Suspense } from "solid-js";

import { loadDocument } from "../content/document";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/// Properties for one documentation page.
type DocumentPageProps = {
    /// The canonical document route.
    route: string;
};

/// Render an authored chapter or generated reference.
export function DocumentPage(props: DocumentPageProps) {
    const [item] = createResource(() => props.route, loadDocument);

    return (
        <Shell>
            <Suspense>
                <Show
                    keyed
                    when={item()}
                    fallback={
                        <Show when={!item.loading}>
                            <MissingPage
                                backHref="/docs/"
                                backLabel="back to docs"
                                description="This documentation page does not exist."
                                label="missing document"
                                title="this page does not exist"
                            />
                        </Show>
                    }
                >
                    {(item) => (
                        <>
                            <Seo
                                description={item.document.description}
                                markdownRoute={item.document.markdownRoute}
                                path={item.document.route}
                                textRoute={item.document.textRoute}
                                title={item.document.title}
                            />
                            <DocumentArticle
                                content={item.content}
                                document={item.document}
                            />
                        </>
                    )}
                </Show>
            </Suspense>
        </Shell>
    );
}

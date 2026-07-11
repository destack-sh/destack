import { createResource, Show, Suspense } from "solid-js";

import { documentByRoute, loadDocument } from "../generated/documents";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

type DocumentPageProps = {
    /// The canonical document route.
    route: string;
};

/// Render one generated documentation page.
export function DocumentPage(props: DocumentPageProps) {
    const document = () => documentByRoute.get(props.route);
    const [content] = createResource(() => props.route, loadDocument);

    return (
        <Shell>
            <Show keyed fallback={<MissingDocument />} when={document()}>
                {(document) => (
                    <>
                        <Seo
                            description={document.description}
                            markdownRoute={document.markdownRoute}
                            path={document.route}
                            textRoute={document.textRoute}
                            title={document.title}
                        />

                        <Suspense>
                            <Show when={content()}>
                                {(content) => (
                                    <DocumentArticle content={content()} document={document} />
                                )}
                            </Show>
                        </Suspense>
                    </>
                )}
            </Show>
        </Shell>
    );
}

/// Render an unknown documentation route.
function MissingDocument() {
    return (
        <MissingPage
            backHref="/docs/"
            backLabel="back to docs"
            description="This documentation chapter does not exist."
            label="missing document"
            title="this chapter does not exist"
        />
    );
}

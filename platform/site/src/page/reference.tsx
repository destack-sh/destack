import { createResource, Show, Suspense } from "solid-js";

import { loadGeneratedReference } from "../content/reference";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

type GeneratedReferencePageProps = {
    /// The canonical generated reference route.
    route: string;
};

/// Render one generated reference.
export function GeneratedReferencePage(props: GeneratedReferencePageProps) {
    const [item] = createResource(() => props.route, loadGeneratedReference);

    return (
        <Shell>
            <Suspense>
                <Show
                    fallback={(
                        <Show when={!item.loading}>
                            <MissingReference />
                        </Show>
                    )}
                    keyed
                    when={item()}
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

/// Render an unknown generated reference.
function MissingReference() {
    return (
        <MissingPage
            backHref="/docs/"
            backLabel="back to docs"
            description="This reference does not exist."
            label="missing reference"
            title="this reference does not exist"
        />
    );
}

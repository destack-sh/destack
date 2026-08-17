import { createResource, Show, Suspense } from "solid-js";

import { loadLibraryItem } from "../content/reference";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

type LibraryItemPageProps = {
    /// The canonical library item route.
    route: string;
};

/// Render one generated standard library item.
export function LibraryItemPage(props: LibraryItemPageProps) {
    const [item] = createResource(() => props.route, loadLibraryItem);

    return (
        <Shell>
            <Suspense>
                <Show
                    fallback={(
                        <Show when={!item.loading}>
                            <MissingLibraryItem />
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

/// Render an unknown generated library item.
function MissingLibraryItem() {
    return (
        <MissingPage
            backHref="/docs/language/library/"
            backLabel="back to standard library"
            description="This standard library item does not exist."
            label="missing library item"
            title="this library item does not exist"
        />
    );
}

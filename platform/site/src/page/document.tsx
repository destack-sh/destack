import { createMemo, Loading, Show } from "@destack/view";

import { loadDocument } from "../content/document";
import { DocumentArticle } from "../reader/document";
import { MissingPage } from "../site/missing";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";

/** Properties for one documentation page. */
type DocumentPageProperties = {
    /** The canonical document route. */
    route: string;
};

/** Render an authored chapter or generated reference. */
export function DocumentPage(properties: DocumentPageProperties) {
    const item = createMemo(() => loadDocument(properties.route));

    return (
        <Shell>
            <Loading>
                <Show
                    keyed
                    when={item()}
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
                    {(item) => (
                        <>
                            <Seo
                                description={item.document.description}
                                markdownRoute={item.document.markdownRoute}
                                path={item.document.route}
                                textRoute={item.document.textRoute}
                                title={item.document.title}
                            />
                            <DocumentArticle content={item.content} document={item.document} />
                        </>
                    )}
                </Show>
            </Loading>
        </Shell>
    );
}

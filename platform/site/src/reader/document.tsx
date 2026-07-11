import { A } from "@solidjs/router";
import { For, Show } from "solid-js";

import { commandEvents } from "../command/command";
import { type Document, type DocumentContent, documents } from "../generated/documents";
import { Breadcrumbs, type Breadcrumb } from "./breadcrumbs";
import { Reader } from "./reader";

type DocumentArticleProps = {
    /// The rendered document body.
    content: DocumentContent;

    /// The current document.
    document: Document;
};

/// Render one manual chapter with book and heading navigation.
export function DocumentArticle(props: DocumentArticleProps) {
    return (
        <Reader
            contents={props.document.tableOfContents}
            location={<DocumentLocation document={props.document} />}
            navigation={<DocumentNavigation current={props.document} />}
            source={props.document}
        >
            <div class="markdown" innerHTML={props.content.html} />
            <DocumentPagination current={props.document} />
        </Reader>
    );
}

type DocumentNavigationProps = {
    /// The current document.
    current: Document;
};

/// Render the ordered manual chapters and local search.
function DocumentNavigation(props: DocumentNavigationProps) {
    return (
        <nav aria-label="manual" class="docs-book">
            <A class="docs-book__title" href="/docs/">
                [docs]
            </A>

            <button
                class="docs-search"
                onClick={() => document.dispatchEvent(new CustomEvent(commandEvents.open))}
                type="button"
            >
                <span aria-hidden="true">/</span>
                <span>search everything</span>
            </button>

            <ol>
                <For each={documents}>
                    {(document) => (
                        <li style={{ "--docs-depth": documentDepth(document) }}>
                            <A
                                activeClass="docs-book__active"
                                end
                                href={document.route}
                            >
                                <span>{String(document.order).padStart(2, "0")}</span>
                                {document.title}
                            </A>
                        </li>
                    )}
                </For>
            </ol>
        </nav>
    );
}

/// Return the visual nesting of one manual chapter.
function documentDepth(document: Document) {
    const parts = document.path.split("/");

    return parts.length > 1 && parts.at(-1) !== "index.md" ? "1" : "0";
}

type DocumentLocationProps = {
    /// The current document.
    document: Document;
};

/// Render the navigable chapter path.
function DocumentLocation(props: DocumentLocationProps) {
    const items = (): readonly Breadcrumb[] => {
        const segments = props.document.route.split("/").filter(Boolean).slice(1);
        const breadcrumbs: Breadcrumb[] = [{ href: "/docs/", label: "docs" }];

        for (let index = 0; index < segments.length; index += 1) {
            const isCurrent = index === segments.length - 1;
            const label = isCurrent ? props.document.title : segments[index];
            const href = isCurrent ? undefined : `/docs/${segments.slice(0, index + 1).join("/")}/`;
            breadcrumbs.push({ href, label });
        }

        return breadcrumbs;
    };

    return <Breadcrumbs items={items()} />;
}

type DocumentPaginationProps = {
    /// The current document.
    current: Document;
};

/// Link to the adjacent chapters in manual order.
function DocumentPagination(props: DocumentPaginationProps) {
    const index = () => documents.findIndex((document) => document.route === props.current.route);
    const previous = () => documents[index() - 1];
    const next = () => documents[index() + 1];

    return (
        <nav aria-label="chapter navigation" class="docs-pagination">
            <Show when={previous()}>
                {(document) => <A href={document().route}>[previous] {document().title}</A>}
            </Show>

            <Show when={next()}>
                {(document) => <A href={document().route}>[next] {document().title}</A>}
            </Show>
        </nav>
    );
}

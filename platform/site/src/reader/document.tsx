import { A } from "@solidjs/router";
import { For, Show } from "solid-js";

import { commandEvents } from "../command/command";
import { type Document, type DocumentContent, documents } from "../generated/documents";
import { Breadcrumbs, type Breadcrumb } from "./breadcrumbs";
import { Reader } from "./reader";
import "../style/manual.css";

/// Properties for one rendered manual chapter.
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
            location={() => <DocumentLocation document={props.document} />}
            navigation={() => <DocumentNavigation current={props.document} />}
            publication="manual"
            source={props.document}
        >
            <header class="manual-folio">
                <span>
                    {String(props.document.order).padStart(2, "0")} / technical field manual
                </span>
            </header>
            <div class="markdown" innerHTML={props.content.html} />
            <DocumentPagination current={props.document} />
        </Reader>
    );
}

/// Properties for the manual chapter navigation.
type DocumentNavigationProps = {
    /// The current document.
    current: Document;
};

/// Render the ordered manual chapters and local search.
function DocumentNavigation(props: DocumentNavigationProps) {
    return (
        <nav aria-label="manual" class="docs-book">
            <A class="docs-book__title" href="/docs/">
                field manual
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
    // omit each collection index from its visual depth
    const segments = document.path.split("/");
    const isDirectoryIndex = segments.at(-1) === "index.md";
    const depth = segments.length - (isDirectoryIndex ? 2 : 1);

    return String(Math.max(0, depth));
}

/// Properties for the manual chapter location.
type DocumentLocationProps = {
    /// The current document.
    document: Document;
};

/// Render the navigable chapter path.
function DocumentLocation(props: DocumentLocationProps) {
    const items = (): readonly Breadcrumb[] => {
        // begin every chapter path at the manual root
        const segments = props.document.route.split("/").filter(Boolean).slice(1);
        const breadcrumbs: Breadcrumb[] = [{ href: "/docs/", label: "docs" }];

        // link each ancestor while leaving the current chapter inert
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

/// Properties for the adjacent chapter navigation.
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
                {(document) => <A href={document().route}>← {document().title}</A>}
            </Show>

            <Show when={next()}>
                {(document) => <A href={document().route}>{document().title} →</A>}
            </Show>
        </nav>
    );
}

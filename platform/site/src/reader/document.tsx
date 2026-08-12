import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { commandEvents } from "../command/command";
import { type Document, type DocumentContent, documents } from "../generated/documents";
import { Breadcrumbs, type Breadcrumb } from "./breadcrumbs";
import { tokens } from "../style/tokens.stylex";
import { Reader } from "./reader";

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
            <header {...stylex.attrs(styles.folio)}>
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
        <nav aria-label="manual" {...stylex.attrs(styles.book)}>
            <A {...stylex.attrs(styles.bookTitle)} href="/docs/">
                field manual
            </A>

            <button
                {...stylex.attrs(styles.search)}
                onClick={() => document.dispatchEvent(new CustomEvent(commandEvents.open))}
                type="button"
            >
                <span aria-hidden="true" {...stylex.attrs(styles.searchPrompt)}>/</span>
                <span>search everything</span>
            </button>

            <ol {...stylex.attrs(styles.bookList)}>
                <For each={documents}>
                    {(document) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    styles.bookLink,
                                    styles.depth(documentDepth(document)),
                                    document.route === props.current.route && styles.active,
                                )}
                                end
                                href={document.route}
                            >
                                <span {...stylex.attrs(styles.bookNumber)}>
                                    {String(document.order).padStart(2, "0")}
                                </span>
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

    return Math.max(0, depth);
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
        <nav aria-label="chapter navigation" {...stylex.attrs(styles.pagination)}>
            <Show when={previous()}>
                {(document) => <A href={document().route}>← {document().title}</A>}
            </Show>

            <Show when={next()}>
                {(document) => <A href={document().route}>{document().title} →</A>}
            </Show>
        </nav>
    );
}

/// Manual navigation styles.
const styles = stylex.create({
    active: {
        color: tokens.ink,
        fontWeight: 600,
    },
    book: {
        alignContent: "start",
        color: tokens.ink,
        display: "grid",
        gap: 0,
    },
    bookLink: {
        color: tokens.soft,
        display: "grid",
        fontSize: "0.78rem",
        gap: "0.4rem",
        gridTemplateColumns: "1.8rem minmax(0, 1fr)",
        lineHeight: 1.3,
        paddingBlock: "0.28rem",
        ":hover": {
            color: tokens.ink,
        },
    },
    bookList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    bookNumber: {
        color: tokens.orange,
        fontFamily: tokens.monoFont,
        fontSize: "0.68rem",
    },
    bookTitle: {
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        paddingBottom: "0.75rem",
        textTransform: "uppercase",
        ":hover": {
            color: tokens.orange,
        },
    },
    depth: (depth: number) => ({
        paddingLeft: `${depth * 0.55}rem`,
    }),
    folio: {
        color: tokens.orange,
        fontSize: "0.7rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        margin: "0.9rem 0 0",
        textTransform: "uppercase",
    },
    pagination: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        marginTop: "3rem",
        paddingTop: "1rem",
    },
    search: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: tokens.soft,
        display: "grid",
        font: "inherit",
        fontSize: "0.78rem",
        gap: "0.4rem",
        gridTemplateColumns: "1rem minmax(0, 1fr)",
        minHeight: tokens.siteControlHeight,
        padding: "0.35rem 0",
        textAlign: "left",
        ":hover": {
            color: tokens.text,
        },
    },
    searchPrompt: {
        color: tokens.accent,
        fontFamily: tokens.monoFont,
    },
});

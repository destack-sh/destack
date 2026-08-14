import { A } from "@solidjs/router";
import { type Accessor, For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { type Document, type DocumentContent, documents } from "../generated/documents";
import { Breadcrumbs, type Breadcrumb } from "./breadcrumbs";
import { ContentsTree, type ContentsEntry } from "./contents";
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
            navigation={(activeHeading) => (
                <DocumentNavigation
                    activeHeading={activeHeading}
                    contents={props.document.tableOfContents}
                    current={props.document}
                />
            )}
            publication="manual"
            source={props.document}
        >
            <header {...stylex.attrs(styles.articleHeader)}>
                <h1 {...stylex.attrs(styles.articleTitle)}>{props.document.title}</h1>
                <p {...stylex.attrs(styles.articleDescription)}>{props.document.description}</p>
            </header>
            <div class="markdown" innerHTML={props.content.html} />
            <DocumentPagination current={props.document} />
        </Reader>
    );
}

/// Properties for the manual chapter navigation.
type DocumentNavigationProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The headings in the current document.
    contents: readonly ContentsEntry[];

    /// The current document.
    current: Document;
};

/// Render the manual chapters and current article headings.
function DocumentNavigation(props: DocumentNavigationProps) {
    return (
        <nav aria-label="manual" {...stylex.attrs(styles.book)}>
            <A {...stylex.attrs(styles.bookTitle)} href="/docs/">
                <span>technical field manual</span>
            </A>

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
                                {document.title}
                            </A>

                            <Show when={document.route === props.current.route}>
                                <ContentsTree
                                    activeId={props.activeHeading}
                                    entries={props.contents}
                                    isNested
                                />
                            </Show>
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
    articleDescription: {
        color: tokens.soft,
        fontFamily: tokens.textFont,
        fontSize: "var(--size-page-description)",
        lineHeight: 1.4,
        margin: `calc(${tokens.publicationSpace} * 2) 0 0`,
        maxWidth: "42rem",
    },
    articleHeader: {
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: 0,
        paddingBottom: `calc(${tokens.publicationSpace} * 4)`,
    },
    articleTitle: {
        fontFamily: tokens.textFont,
        fontSize: "var(--size-page-title)",
        fontWeight: 300,
        letterSpacing: "-0.035em",
        lineHeight: 1,
        margin: `calc(${tokens.publicationSpace} * 4) 0 0`,
    },
    book: {
        alignContent: "start",
        color: tokens.ink,
        display: "grid",
        gap: 0,
    },
    bookLink: {
        color: tokens.soft,
        display: "block",
        fontSize: "var(--size-navigation)",
        lineHeight: 1.3,
        paddingBlock: "0.25rem",
        ":hover": {
            color: tokens.ink,
        },
    },
    bookList: {
        display: "grid",
        gap: 0,
        listStyle: "none",
        margin: 0,
        padding: `calc(${tokens.publicationSpace} * 5) 0 0`,
    },
    bookTitle: {
        alignItems: "center",
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        color: tokens.ink,
        display: "flex",
        fontFamily: tokens.monoFont,
        fontSize: "var(--size-label)",
        fontWeight: 600,
        gap: "0.75rem",
        letterSpacing: "0.02em",
        minHeight: tokens.publicationRow,
        ":hover": {
            color: tokens.orange,
        },
    },
    depth: (depth: number) => ({
        paddingLeft: `${depth * 0.55}rem`,
    }),
    pagination: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        marginTop: `calc(${tokens.publicationSpace} * 4)`,
        paddingTop: `calc(${tokens.publicationSpace} * 2)`,
    },
});

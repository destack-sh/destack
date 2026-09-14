import { A } from "@solidjs/router";
import { type Accessor, For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { publicationStyles } from "./publication.stylex";

import { type Document, type DocumentContent } from "../content/document";
import { Breadcrumbs } from "./breadcrumbs";
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
    const tokenCount =
        props.document.kind === "chapter" ? props.document.tokens : undefined;

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
            tokenCount={tokenCount}
        >
            <header {...stylex.attrs(publicationStyles.header)}>
                <h1
                    {...stylex.attrs(
                        publicationStyles.title,
                        props.document.kind === "rule" && styles.lintRuleTitle,
                    )}
                >
                    {props.document.title}
                </h1>
                <Show when={props.document.lead}>
                    {(lead) => (
                        <p {...stylex.attrs(publicationStyles.description)}>
                            {lead()}
                        </p>
                    )}
                </Show>
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
    const navigation = () => props.current.navigation;

    return (
        <nav aria-label="manual" {...stylex.attrs(styles.book)}>
            <A
                {...stylex.attrs(publicationStyles.collectionTitle)}
                href={navigation().root.route}
            >
                {navigation().root.title}
            </A>
            <ol {...stylex.attrs(publicationStyles.collectionList)}>
                <For each={navigation().entries}>
                    {(entry) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    styles.bookLink,
                                    documentIndent(entry.depth),
                                    entry.depth === 0 && styles.section,
                                    entry.route === props.current.route &&
                                        publicationStyles.active,
                                )}
                                end
                                href={entry.route}
                            >
                                {entry.title}
                            </A>
                            <Show
                                when={
                                    entry.route === props.current.route &&
                                    props.contents.length > 0
                                }
                            >
                                <div
                                    {...stylex.attrs(
                                        documentIndent(entry.depth),
                                    )}
                                >
                                    <ContentsTree
                                        activeId={props.activeHeading}
                                        entries={props.contents}
                                        isNested
                                    />
                                </div>
                            </Show>
                        </li>
                    )}
                </For>
            </ol>
        </nav>
    );
}

/// Return the indentation of one generated navigation entry.
function documentIndent(depth: number) {
    return [styles.depth0, styles.depth1, styles.depth2, styles.depth3][
        Math.min(depth, 3)
    ];
}

/// Render the generated document ancestors.
function DocumentLocation(props: { document: Document }) {
    return (
        <Breadcrumbs
            items={props.document.navigation.ancestors.map((link) => ({
                href: link.route,
                label: link.title,
            }))}
        />
    );
}

/// Render the generated adjacent chapter links.
function DocumentPagination(props: { current: Document }) {
    const previous = () => props.current.navigation.previous;
    const next = () => props.current.navigation.next;

    return (
        <Show when={previous() || next()}>
            <nav
                aria-label="chapter navigation"
                {...stylex.attrs(styles.pagination)}
            >
                <Show when={previous()}>
                    {(link) => (
                        <A
                            {...stylex.attrs(styles.paginationLink)}
                            href={link().route}
                        >
                            ← {link().title}
                        </A>
                    )}
                </Show>
                <Show when={next()}>
                    {(link) => (
                        <A
                            {...stylex.attrs(styles.paginationLink)}
                            href={link().route}
                        >
                            {link().title} →
                        </A>
                    )}
                </Show>
            </nav>
        </Show>
    );
}

/// Manual navigation styles.
const styles = stylex.create({
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
            color: tokens.accent,
        },
    },

    depth0: {
        paddingLeft: 0,
    },
    depth1: {
        paddingLeft: "1rem",
    },
    depth2: {
        paddingLeft: "2rem",
    },
    depth3: {
        paddingLeft: "3rem",
    },
    pagination: {
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        gap: "1rem 2rem",
        justifyContent: "space-between",
        marginTop: `calc(${tokens.publicationSpace} * 4)`,
        paddingTop: `calc(${tokens.publicationSpace} * 2)`,
    },
    paginationLink: {
        textTransform: "lowercase",
        color: tokens.ink,
        ":hover": {
            color: tokens.accent,
        },
    },
    section: {
        color: tokens.ink,
        fontWeight: 500,
        paddingTop: `calc(${tokens.publicationSpace} * 1.5)`,
    },
    lintRuleTitle: {
        fontSize: "clamp(2rem, 3.5vw, 2.75rem)",
        overflowWrap: "anywhere",
    },
});

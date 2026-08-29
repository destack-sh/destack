import { A } from "@solidjs/router";
import { type Accessor, For, Show } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import {
    type Document,
    type DocumentContent,
    documents,
} from "../generated/documents";
import { Breadcrumbs, type Breadcrumb } from "./breadcrumbs";
import { ContentsTree, type ContentsEntry } from "./contents";
import { tokens } from "../style/tokens.stylex";
import { Reader } from "./reader";

const mobile = "@media (max-width: 767px)";

const guideIndex = "language/index.md";
const standardLibraryPath = "language/library/";
const standardLibraryIndex = `${standardLibraryPath}index.md`;

/// Properties for one rendered manual chapter.
type DocumentArticleProps = {
    /// The rendered document body.
    content: DocumentContent;

    /// The current document.
    document: Document;
};

/// Render one manual chapter with book and heading navigation.
export function DocumentArticle(props: DocumentArticleProps) {
    // show reading size for authored manual chapters
    const tokenCount = props.document.path.startsWith(standardLibraryPath)
        ? undefined
        : props.document.tokens;

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
            <header {...stylex.attrs(styles.articleHeader)}>
                <h1 {...stylex.attrs(styles.articleTitle)}>
                    {props.document.title}
                </h1>
                <Show when={props.document.lead}>
                    {(lead) => (
                        <p {...stylex.attrs(styles.articleDescription)}>
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
    // show guide roots and the current section
    const isStandardLibrary =
        props.current.path.startsWith(standardLibraryPath);
    const currentSection = props.current.path.split("/")[1];
    const chapters = documents.filter((document) => {
        const isGeneratedLibrary =
            document.path.startsWith(standardLibraryPath) &&
            document.path !== standardLibraryIndex;
        if (document.path === guideIndex || isGeneratedLibrary) {
            return false;
        }

        const section = document.path.split("/")[1];

        return documentDepth(document) === 1 || section === currentSection;
    });

    return (
        <nav aria-label="manual" {...stylex.attrs(styles.book)}>
            <A {...stylex.attrs(styles.bookTitle)} href="/docs/">
                <span>guide</span>
            </A>

            <ol {...stylex.attrs(styles.bookList)}>
                <For each={chapters}>
                    {(document) => (
                        <li>
                            <A
                                {...stylex.attrs(
                                    styles.bookLink,
                                    documentIndent(document),
                                    documentDepth(document) === 0 &&
                                        styles.section,
                                    (document.route === props.current.route ||
                                        (document.path ===
                                            standardLibraryIndex &&
                                            isStandardLibrary)) &&
                                        styles.active,
                                )}
                                end
                                href={document.route}
                            >
                                {document.title}
                            </A>

                            <Show when={document.route === props.current.route}>
                                <div
                                    {...stylex.attrs(
                                        styles.documentContents,
                                        documentIndent(document),
                                    )}
                                >
                                    <ContentsTree
                                        activeId={props.activeHeading}
                                        entries={props.contents}
                                        isNested
                                    />
                                </div>
                            </Show>

                            <Show
                                when={
                                    document.path === standardLibraryIndex &&
                                    props.current.path !==
                                        standardLibraryIndex &&
                                    isStandardLibrary
                                }
                            >
                                <Show
                                    fallback={
                                        <>
                                            <A
                                                {...stylex.attrs(
                                                    styles.bookLink,
                                                    styles.depth2,
                                                    styles.active,
                                                )}
                                                end
                                                href={props.current.route}
                                            >
                                                {props.current.title}
                                            </A>
                                            <div
                                                {...stylex.attrs(
                                                    styles.documentContents,
                                                    styles.depth2,
                                                )}
                                            >
                                                <ContentsTree
                                                    activeId={
                                                        props.activeHeading
                                                    }
                                                    entries={props.contents}
                                                    isNested
                                                />
                                            </div>
                                        </>
                                    }
                                    when={props.current.moduleRoute}
                                >
                                    {(moduleRoute) => (
                                        <>
                                            <A
                                                {...stylex.attrs(
                                                    styles.bookLink,
                                                    styles.depth2,
                                                )}
                                                end
                                                href={moduleRoute()}
                                            >
                                                {props.current.moduleTitle}
                                            </A>
                                            <A
                                                {...stylex.attrs(
                                                    styles.bookLink,
                                                    styles.depth3,
                                                    styles.active,
                                                )}
                                                end
                                                href={props.current.route}
                                            >
                                                {props.current.title}
                                            </A>
                                            <div
                                                {...stylex.attrs(
                                                    styles.documentContents,
                                                    styles.depth3,
                                                )}
                                            >
                                                <ContentsTree
                                                    activeId={
                                                        props.activeHeading
                                                    }
                                                    entries={props.contents}
                                                    isNested
                                                />
                                            </div>
                                        </>
                                    )}
                                </Show>
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

/// Return the fixed indentation for one manual chapter.
function documentIndent(document: Document) {
    switch (documentDepth(document)) {
        // top-level area
        case 0:
            return styles.depth0;

        // direct child
        case 1:
            return styles.depth1;

        // grandchild
        case 2:
            return styles.depth2;

        // deeper descendants
        default:
            return styles.depth3;
    }
}

/// Properties for the manual chapter location.
type DocumentLocationProps = {
    /// The current document.
    document: Document;
};

/// Render the navigable chapter path.
function DocumentLocation(props: DocumentLocationProps) {
    const items = (): readonly Breadcrumb[] => {
        // describe generated items through their public module
        if (
            props.document.moduleRoute != undefined &&
            props.document.moduleTitle != undefined
        ) {
            return [
                { href: "/docs/language/", label: "language" },
                { href: "/docs/language/library/", label: "library" },
                {
                    href: props.document.moduleRoute,
                    label: props.document.moduleTitle,
                },
            ];
        }

        // retain only the navigable ancestors
        const segments = props.document.route
            .split("/")
            .filter(Boolean)
            .slice(1, -1);
        const breadcrumbs: Breadcrumb[] = [];

        // link each ancestor from the documentation root
        for (let index = 0; index < segments.length; index += 1) {
            const label = segments[index];
            const href = `/docs/${segments.slice(0, index + 1).join("/")}/`;
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
    const index = () =>
        documents.findIndex(
            (document) => document.route === props.current.route,
        );
    const previous = () => documents[index() - 1];
    const next = () => documents[index() + 1];

    return (
        <Show when={index() >= 0}>
            <nav
                aria-label="chapter navigation"
                {...stylex.attrs(styles.pagination)}
            >
                <Show when={previous()}>
                    {(document) => (
                        <A href={document().route}>← {document().title}</A>
                    )}
                </Show>

                <Show when={next()}>
                    {(document) => (
                        <A href={document().route}>{document().title} →</A>
                    )}
                </Show>
            </nav>
        </Show>
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
        margin: 0,
        maxWidth: "42rem",
    },
    articleHeader: {
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        display: "grid",
        gap: tokens.publicationSpace,
        paddingBlock: `calc(${tokens.publicationSpace} * 3)`,
        [mobile]: {
            gap: tokens.publicationSpace,
            paddingBlock: "1rem",
        },
    },
    articleTitle: {
        fontFamily: tokens.displayFont,
        fontSize: "var(--size-page-title)",
        fontWeight: 400,
        letterSpacing: "-0.03em",
        lineHeight: 1,
        margin: 0,
        textIndent: "-0.04em",
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
        padding: `calc(${tokens.publicationSpace} * 4) 0 0`,
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
    documentContents: {
        paddingBottom: `calc(${tokens.publicationSpace} * 1.5)`,
        paddingTop: `calc(${tokens.publicationSpace} * 0.5)`,
    },
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
    section: {
        color: tokens.ink,
        fontWeight: 600,
        paddingTop: `calc(${tokens.publicationSpace} * 1.5)`,
    },
});

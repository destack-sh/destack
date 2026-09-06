import { type Accessor, type JSX, onCleanup, onMount } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import type { PageSource } from "../content/source";
import { type ContentsEntry, trackActiveHeading } from "./contents";
import {
    createPageSourceCommands,
    type PageSourceCommands,
    SourceActions,
} from "./source";
import { tokens } from "../style/tokens.stylex";

/// Properties for the shared reading frame.
type ReaderProps = {
    /// The rendered article.
    children: JSX.Element;

    /// The current article headings.
    contents: readonly ContentsEntry[];

    /// The publication treatment applied to the reader.
    publication: "journal" | "manual";

    /// The collection navigation shown beside the article.
    navigation: (activeHeading: Accessor<string>) => JSX.Element;

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The portable source files for the current page.
    source: PageSource;

    /// The approximate token count shown for authored pages.
    tokenCount?: number;
};

/// Properties for one responsive reader toolbar.
type ReaderToolbarProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The collection navigation shown beside the article.
    navigation: (activeHeading: Accessor<string>) => JSX.Element;

    /// The shared page source commands.
    sourceCommands: PageSourceCommands;

    /// The approximate token count shown for authored pages.
    tokenCount?: number;
};

/// Render the common blog and documentation reading frame.
export function Reader(props: ReaderProps) {
    const activeHeading = trackActiveHeading(props.contents);
    const sourceCommands = createPageSourceCommands(props.source);

    // align direct links after responsive layout and webfonts settle
    onMount(() => {
        const scrollToHeading = () => {
            const identifier = decodeURIComponent(
                window.location.hash.slice(1),
            );
            if (identifier === "") {
                return;
            }

            document
                .getElementById(identifier)
                ?.scrollIntoView({ block: "start" });
        };
        const scrollAfterLayout = () => {
            window.requestAnimationFrame(scrollToHeading);
        };

        void document.fonts.ready.then(scrollAfterLayout);
        window.addEventListener("hashchange", scrollAfterLayout);

        // detach the route listener with the reader
        onCleanup(() => {
            window.removeEventListener("hashchange", scrollAfterLayout);
        });
    });

    return (
        <div
            {...stylex.attrs(styles.reader)}
            data-publication={props.publication}
        >
            <aside {...stylex.attrs(styles.sidebar)}>
                {props.navigation(activeHeading)}
            </aside>

            <article
                {...stylex.attrs(styles.article)}
                data-markdown-route={props.source.markdownRoute}
                data-page-source
                data-text-route={props.source.textRoute}
            >
                <ReaderToolbar
                    activeHeading={activeHeading}
                    location={props.location}
                    navigation={props.navigation}
                    sourceCommands={sourceCommands}
                    tokenCount={props.tokenCount}
                />

                {props.children}
            </article>
        </div>
    );
}

/// Render one toolbar at its responsive DOM position.
function ReaderToolbar(props: ReaderToolbarProps) {
    return (
        <header {...stylex.attrs(styles.toolbar, styles.toolbarPublication)}>
            <details {...stylex.attrs(styles.menu)} name="reader-tools">
                <summary {...stylex.attrs(styles.menuSummary)}>menu</summary>
                <div {...stylex.attrs(styles.menuBody)}>
                    {props.navigation(props.activeHeading)}
                </div>
            </details>

            <div {...stylex.attrs(styles.location)}>{props.location()}</div>
            <div {...stylex.attrs(styles.toolbarTools)}>
                {props.tokenCount !== undefined && (
                    <span {...stylex.attrs(styles.tokenCount)}>
                        {formatTokenCount(props.tokenCount)}
                    </span>
                )}
                <SourceActions commands={props.sourceCommands} />
            </div>
        </header>
    );
}

/// Format an approximate token count for the compact article toolbar.
function formatTokenCount(tokenCount: number) {
    // keep exact counts legible for short pages
    if (tokenCount < 1_000) {
        return `${tokenCount} tokens`;
    }

    // retain one useful decimal without trailing zeroes
    const thousands = Math.round(tokenCount / 100) / 10;

    return `${thousands}k tokens`;
}

const narrow = "@media (width < 60rem)";
const compact = "@media (width < 52rem)";
const mobile = "@media (max-width: 767px)";

/// Shared reader styles.
const styles = stylex.create({
    article: {
        alignContent: "start",
        color: tokens.ink,
        display: "grid",
        gridColumn: 2,
        maxWidth: "100%",
        minWidth: 0,
        width: "100%",
        [narrow]: {
            gridColumn: "auto",
        },
    },
    location: {
        minWidth: 0,
        [narrow]: {
            gridColumn: "1 / -1",
            gridRow: 2,
        },
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 60rem)": {
            display: "none",
        },
        [narrow]: {
            gridColumn: "1 / -1",
            gridRow: 1,
        },
    },
    menuBody: {
        alignContent: "start",
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        gap: "2rem",
        maxHeight: "min(32rem, calc(100svh - 10rem))",
        overflowY: "auto",
        padding: "1rem 0 0.5rem",
    },
    menuSummary: {
        alignItems: "center",
        color: tokens.text,
        cursor: "pointer",
        display: "flex",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
        fontWeight: 500,
        gap: "0.75rem",
        justifyContent: "flex-start",
        listStyle: "none",
        minHeight: tokens.siteControlHeight,
    },
    reader: {
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-body)",
        columnGap: "4rem",
        gridTemplateColumns: "15rem minmax(0, 1fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `1.5rem ${tokens.gutterRight} 4rem ${tokens.gutterLeft}`,
        width: "100%",
        [narrow]: {
            display: "block",
            maxWidth: "48rem",
            padding: `1rem ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingTop: "0.75rem",
            paddingBottom: "2rem",
        },
    },
    sidebar: {
        alignSelf: "start",
        alignContent: "start",
        display: "none",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        gridColumn: 1,
        gap: "2rem",
        "@media (min-width: 60rem)": {
            display: "grid",
            position: "sticky",
            top: "1.5rem",
        },
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: 0,
        borderTopColor: tokens.line,
        borderTopStyle: "solid",
        borderTopWidth: 0,
        color: tokens.ink,
        display: "flex",
        flexWrap: "wrap",
        fontSize: tokens.siteFontSize,
        gap: "0.5rem",
        justifyContent: "space-between",
        minHeight: tokens.publicationRow,
        paddingBlock: 0,
        [narrow]: {
            alignItems: "start",
            display: "grid",
            gap: "0.25rem 0.75rem",
            gridTemplateColumns: "minmax(0, 1fr) auto",
            justifyContent: "stretch",
        },
    },
    toolbarPublication: {
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
    },
    toolbarTools: {
        alignItems: "center",
        display: "flex",
        gap: "0.75rem",
        minHeight: tokens.publicationRow,
        minWidth: 0,
        [narrow]: {
            gridColumn: 2,
            gridRow: 1,
        },
    },
    tokenCount: {
        color: tokens.ink,
        whiteSpace: "nowrap",
        "::after": {
            content: "·",
            marginLeft: "0.75rem",
        },
        [compact]: {
            display: "none",
        },
    },
});

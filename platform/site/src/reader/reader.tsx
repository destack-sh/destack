import type { Accessor, JSX } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import type { PageSource } from "../content/source";
import { Contents, type ContentsEntry, trackActiveHeading } from "./contents";
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
    navigation: () => JSX.Element;

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The portable source files for the current page.
    source: PageSource;
};

/// Properties for one responsive reader toolbar.
type ReaderToolbarProps = {
    /// The currently active heading identifier.
    activeHeading: Accessor<string>;

    /// The toolbar placement.
    placement: "bottom" | "top";

    /// The current article headings.
    contents: readonly ContentsEntry[];

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The collection navigation shown beside the article.
    navigation: () => JSX.Element;

    /// The shared page source commands.
    sourceCommands: PageSourceCommands;
};

/// Render the common blog and documentation reading frame.
export function Reader(props: ReaderProps) {
    const activeHeading = trackActiveHeading(props.contents);
    const sourceCommands = createPageSourceCommands(props.source);

    return (
        <div
            {...stylex.attrs(
                styles.reader,
                styles.readerPublication,
                props.publication === "journal" && styles.readerJournal,
            )}
            data-publication={props.publication}
        >
            <aside
                {...stylex.attrs(
                    styles.sidebar,
                    props.publication === "journal" && styles.sidebarJournal,
                )}
            >
                {props.navigation()}
                <Contents activeId={activeHeading} entries={props.contents} />
            </aside>

            <article
                {...stylex.attrs(
                    styles.article,
                    styles.articlePublication,
                    props.publication === "journal" && styles.articleJournal,
                )}
                data-markdown-route={props.source.markdownRoute}
                data-page-source
                data-text-route={props.source.textRoute}
            >
                <ReaderToolbar
                    activeHeading={activeHeading}
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    placement="top"
                    sourceCommands={sourceCommands}
                />

                {props.children}

                <ReaderToolbar
                    activeHeading={activeHeading}
                    contents={props.contents}
                    location={props.location}
                    navigation={props.navigation}
                    placement="bottom"
                    sourceCommands={sourceCommands}
                />
            </article>
        </div>
    );
}

/// Render one toolbar at its responsive DOM position.
function ReaderToolbar(props: ReaderToolbarProps) {
    return (
        <header
            {...stylex.attrs(
                styles.toolbar,
                styles.toolbarPublication,
                props.placement === "top" ? styles.toolbarTop : styles.toolbarBottom,
            )}
        >
            <details {...stylex.attrs(styles.menu)} name="reader-tools">
                <summary {...stylex.attrs(styles.menuSummary)}>menu</summary>
                <div {...stylex.attrs(styles.menuBody)}>
                    {props.navigation()}
                    <Contents activeId={props.activeHeading} entries={props.contents} isMenu />
                </div>
            </details>

            <div
                {...stylex.attrs(
                    styles.location,
                    props.placement === "bottom" && styles.locationBottom,
                )}
            >
                {props.location()}
            </div>
            <SourceActions commands={props.sourceCommands} />
        </header>
    );
}

const narrow = "@media (width < 60rem)";
const mobile = "@media (max-width: 767px)";

/// Shared reader styles.
const styles = stylex.create({
    article: {
        alignContent: "start",
        display: "grid",
        gap: "2rem",
        maxWidth: "100%",
        minWidth: 0,
        width: "100%",
        [narrow]: {
            gap: "2rem",
        },
    },
    articlePublication: {
        color: tokens.ink,
        gap: 0,
    },
    articleJournal: {
        gridColumn: 2,
        gridRow: "1 / span 5",
        gridTemplateRows: "subgrid",
        [narrow]: {
            gridColumn: "auto",
            gridRow: "auto",
            gridTemplateRows: "none",
        },
    },
    location: {
        minWidth: 0,
        [narrow]: {
            gridColumn: "1 / -1",
            gridRow: 2,
        },
    },
    locationBottom: {
        [mobile]: {
            display: "none",
        },
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 60rem)": {
            display: "none",
        },
        [narrow]: {
            gridColumn: 1,
            gridRow: 1,
        },
    },
    menuBody: {
        alignContent: "start",
        display: "grid",
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
        fontWeight: 600,
        gap: "0.75rem",
        justifyContent: "flex-start",
        listStyle: "none",
        minHeight: tokens.siteControlHeight,
    },
    reader: {
        display: "grid",
        fontSize: "1rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        justifyContent: "center",
        marginInline: "auto",
        maxWidth: "45rem",
        padding: `3rem ${tokens.gutterRight} 6rem ${tokens.gutterLeft}`,
        width: "100%",
        "@media (min-width: 60rem)": {
            gap: "3rem",
            gridTemplateColumns: "16rem minmax(0, 42rem)",
            justifyContent: "start",
            maxWidth: tokens.siteWidth,
        },
        [narrow]: {
            padding: `1.25rem ${tokens.gutterRight} 4rem ${tokens.gutterLeft}`,
        },
        [mobile]: {
            paddingBottom: "2rem",
        },
    },
    readerPublication: {
        fontFamily: tokens.textFont,
        fontSize: "1rem",
        gap: "3rem",
        gridTemplateColumns: "13rem minmax(0, 48rem)",
        justifyContent: "start",
        maxWidth: tokens.siteWidth,
        paddingTop: "3rem",
        [narrow]: {
            display: "block",
            maxWidth: "46rem",
            paddingTop: "2rem",
        },
        "@media (max-width: 600px)": {
            paddingTop: "1rem",
        },
    },
    readerJournal: {
        columnGap: "3rem",
        gridTemplateRows: "auto auto auto auto auto",
        rowGap: 0,
    },
    sidebar: {
        alignContent: "start",
        display: "none",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        gap: "2rem",
        "@media (min-width: 60rem)": {
            display: "grid",
            maxHeight: "calc(100svh - 8rem)",
            overflow: "auto",
            padding: "0 0.25rem 1rem 0",
            position: "sticky",
            top: "5rem",
        },
    },
    sidebarJournal: {
        gap: 0,
        gridRow: "1 / span 3",
        gridTemplateRows: "subgrid",
        maxHeight: "none",
        overflow: "visible",
        position: "static",
        [narrow]: {
            gridRow: "auto",
            gridTemplateRows: "none",
        },
    },
    toolbar: {
        alignItems: "baseline",
        color: tokens.soft,
        display: "flex",
        flexWrap: "wrap",
        fontSize: tokens.siteFontSize,
        gap: "0.5rem",
        justifyContent: "space-between",
        paddingBottom: "0.75rem",
        [narrow]: {
            alignItems: "start",
            display: "grid",
            gap: "0.25rem 0.75rem",
            gridTemplateColumns: "minmax(0, 1fr) auto",
            justifyContent: "stretch",
        },
    },
    toolbarBottom: {
        display: "none",
        [mobile]: {
            borderTopColor: tokens.line,
            borderTopStyle: "solid",
            borderTopWidth: "1px",
            display: "grid",
            marginTop: "1rem",
            paddingBottom: 0,
            paddingTop: "0.75rem",
        },
    },
    toolbarPublication: {
        fontFamily: tokens.monoFont,
        fontSize: "0.7rem",
    },
    toolbarTop: {
        [mobile]: {
            display: "none",
        },
    },
});

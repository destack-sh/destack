import { formatReadTime } from "../content/presentation";
import { type JSX, onCleanup, onMount } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import type { PageSource } from "../content/source";
import {
    createPageSourceCommands,
    type PageSourceCommands,
    SourceActions,
} from "./source";
import { tokens } from "../style/tokens.stylex";
import { playVideo } from "./media";
import { publicationStyles } from "./publication.stylex";

/// Properties for the shared reading frame.
type ReaderProps = {
    /// The rendered article.
    children: JSX.Element;

    /// The publication treatment applied to the reader.
    publication: "journal" | "manual";

    /// The collection navigation shown beside the article.
    navigation: () => JSX.Element;

    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The portable source files for the current page.
    source: PageSource;

    /// Publication metadata displayed above the page title.
    metadata?: () => JSX.Element;

    /// The approximate token count shown for authored pages.
    tokenCount?: number;
};

/// Properties for one responsive reader toolbar.
type ReaderToolbarProps = {
    /// The current location rendered in the article toolbar.
    location: () => JSX.Element;

    /// The collection navigation shown beside the article.
    navigation: () => JSX.Element;

    /// The shared page source commands.
    sourceCommands: PageSourceCommands;

    /// Publication metadata displayed above the page title.
    metadata?: () => JSX.Element;

    /// The approximate token count shown for authored pages.
    tokenCount?: number;
};

/// Render the common blog and documentation reading frame.
export function Reader(props: ReaderProps) {
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
            {...stylex.attrs(publicationStyles.layout)}
            data-publication={props.publication}
        >
            <aside {...stylex.attrs(publicationStyles.sidebar)}>
                {props.navigation()}
            </aside>

            <article
                {...stylex.attrs(publicationStyles.article)}
                data-markdown-route={props.source.markdownRoute}
                data-page-source
                data-text-route={props.source.textRoute}
                onClick={playVideo}
            >
                <ReaderToolbar
                    metadata={props.metadata}
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
        <header {...stylex.attrs(styles.toolbar, styles.toolbarPublication, props.metadata !== undefined && styles.toolbarWithMetadata)}>
            <details
                {...stylex.attrs(styles.menu)}
                name="reader-tools"
                onKeyDown={(event) => {
                    if (event.key === "Escape") {
                        event.currentTarget.open = false;
                        event.currentTarget.querySelector("summary")?.focus();
                    }
                }}
            >
                <summary aria-label="Contents" title="Contents" {...stylex.attrs(styles.menuSummary)}>
                    Contents
                    <svg aria-hidden="true" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="m6 9 6 6 6-6" /></svg>
                </summary>
                <div
                    {...stylex.attrs(styles.menuBody)}
                    onClick={(event) => {
                        if (event.target instanceof Element && event.target.closest("a")) {
                            event.currentTarget.closest("details")?.removeAttribute("open");
                        }
                    }}
                >
                    {props.navigation()}
                </div>
            </details>

            <div {...stylex.attrs(styles.location)}>{props.location()}</div>
            {props.metadata && <div {...stylex.attrs(styles.metadata)}>{props.metadata()}</div>}
            <div {...stylex.attrs(styles.toolbarTools)}>
                {props.tokenCount !== undefined && (
                    <>
                        <span
                            {...stylex.attrs(styles.statistic)}
                            title="Estimated reading time"
                        >
                            {formatReadTime(props.tokenCount)}
                        </span>
                        <span
                            {...stylex.attrs(
                                styles.statistic,
                                styles.tokenCount,
                            )}
                        >
                            {formatTokenCount(props.tokenCount)}
                        </span>
                    </>
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

const narrow = "@media (width < 80rem)";
const compact = "@media (width < 52rem)";

/// Shared reader styles.
const styles = stylex.create({
    location: {
        minWidth: 0,
        overflow: "hidden",
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 80rem)": { display: "none" },
    },
    menuBody: {
        backgroundColor: tokens.page,
        borderColor: tokens.line,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 12px 32px rgb(0 0 0 / 12%)",
        display: "grid",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-navigation)",
        gap: "1.25rem",
        left: 0,
        right: 0,
        maxHeight: "min(32rem, 65svh)",
        overflowY: "auto",
        overscrollBehavior: "contain",
        padding: "1rem",
        position: "absolute",
        top: "100%",
        zIndex: 20,
    },
    menuSummary: {
        alignItems: "center",
        color: tokens.text,
        cursor: "pointer",
        display: "flex",
        justifyContent: "center",
        listStyle: "none",
        height: "2.75rem",
        width: "auto",
        gap: "0.375rem",
        fontSize: "var(--size-label)",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: tokens.line,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: tokens.ink,
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: "minmax(0, 1fr) auto",
        minHeight: "var(--content-context-height)",
        position: "relative",
        [narrow]: { gridTemplateColumns: "auto minmax(0, 1fr) auto" },
    },
    toolbarWithMetadata: {
        gridTemplateColumns: "minmax(0, 1fr) auto auto",
        [narrow]: { gridTemplateColumns: "auto minmax(0, 1fr) auto auto" },
        "@media (max-width: 600px)": { gridTemplateColumns: "auto minmax(0, 1fr) auto", rowGap: 0 },
    },
    metadata: {
        display: "flex",
        alignItems: "baseline",
        flexWrap: "wrap",
        gap: "0.5rem 1rem",
        color: tokens.soft,
        "@media (max-width: 600px)": { gridColumn: "1 / -1", gridRow: 2, paddingBottom: "0.75rem" },
    },
    toolbarPublication: {
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
    },
    toolbarTools: {
        gridColumn: "-2 / -1",
        gridRow: 1,
        alignItems: "center",
        display: "flex",
        gap: "0.75rem",
        minWidth: 0,
    },
    statistic: {
        color: tokens.ink,
        whiteSpace: "nowrap",
        "::after": {
            content: "·",
            marginLeft: "0.75rem",
        },
    },
    tokenCount: {
        [compact]: {
            display: "none",
        },
    },
});

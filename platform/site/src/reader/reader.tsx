import { color, fontFamily } from "@destack/theme/tokens.stylex";
import { formatReadTime } from "../content/presentation";
import { type JSX, onSettled } from "@destack/view";
import * as stylex from "@destack/style";

import type { PageSource } from "../content/source";
import { createPageSourceCommands, type PageSourceCommands, SourceActions } from "./source";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { playVideo } from "./media";
import { renderDiagrams } from "./diagram";
import { publicationStyles } from "./publication.stylex";

/** Properties for the shared reading frame. */
type ReaderProperties = {
    /** The rendered article. */
    children: JSX.Element;

    /** The publication treatment applied to the reader. */
    publication: "journal" | "manual";

    /** The collection navigation shown beside the article. */
    navigation: () => JSX.Element;

    /** The current location rendered in the article toolbar. */
    location: () => JSX.Element;

    /** The portable source files for the current page. */
    source: PageSource;

    /** The approximate token count shown for authored pages. */
    tokenCount?: number;

    /** The adjacent page links shown below the article. */
    pagination?: () => JSX.Element;
};

/** Properties for one responsive reader toolbar. */
type ReaderToolbarProperties = {
    /** The current location rendered in the article toolbar. */
    location: () => JSX.Element;

    /** The collection navigation shown beside the article. */
    navigation: () => JSX.Element;

    /** The shared page source commands. */
    sourceCommands: PageSourceCommands;

    /** The approximate token count shown for authored pages. */
    tokenCount?: number;
};

/** Render the common blog and documentation reading frame. */
export function Reader(properties: ReaderProperties) {
    // hold the article and its source commands
    let article!: HTMLElement;
    const sourceCommands = createPageSourceCommands(properties.source);

    // render diagrams after the article mounts
    onSettled(() => renderDiagrams(article));

    // align direct links after responsive layout and webfonts settle
    onSettled(() => {
        // scroll to the heading the URL names
        const scrollToHeading = () => {
            const identifier = decodeURIComponent(window.location.hash.slice(1));
            if (identifier === "") {
                return;
            }

            document.getElementById(identifier)?.scrollIntoView({ block: "start" });
        };
        const scrollAfterLayout = () => {
            window.requestAnimationFrame(scrollToHeading);
        };

        // scroll once the fonts load and on each hash change
        void document.fonts.ready.then(scrollAfterLayout);
        window.addEventListener("hashchange", scrollAfterLayout);

        return () => {
            // detach the route listener with the reader
            window.removeEventListener("hashchange", scrollAfterLayout);
        };
    });

    return (
        <div
            {...stylex.attrs(lattice.frame, lattice.ruleBottom, publicationStyles.layout)}
            data-publication={properties.publication}
        >
            <aside {...stylex.attrs(lattice.ruleRight, publicationStyles.sidebar)}>
                <div {...stylex.attrs(publicationStyles.sidebarContent)}>
                    {properties.navigation()}
                </div>
            </aside>

            <article
                ref={article}
                {...stylex.attrs(publicationStyles.article)}
                data-markdown-route={properties.source.markdownRoute}
                data-page-source
                data-text-route={properties.source.textRoute}
                onClick={playVideo}
            >
                <ReaderToolbar
                    location={properties.location}
                    navigation={properties.navigation}
                    sourceCommands={sourceCommands}
                    tokenCount={properties.tokenCount}
                />

                <div {...stylex.attrs(publicationStyles.body)}>{properties.children}</div>
                {properties.pagination?.()}
            </article>
        </div>
    );
}

/** Render one toolbar at its responsive DOM position. */
function ReaderToolbar(properties: ReaderToolbarProperties) {
    return (
        <header {...stylex.attrs(styles.toolbar)}>
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
                <summary
                    aria-label="Contents"
                    title="Contents"
                    {...stylex.attrs(styles.menuSummary)}
                >
                    Contents
                    <svg
                        aria-hidden="true"
                        width="12"
                        height="12"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                    >
                        <path d="m6 9 6 6 6-6" />
                    </svg>
                </summary>
                <div
                    {...stylex.attrs(styles.menuBody)}
                    onClick={(event) => {
                        if (event.target instanceof Element && event.target.closest("a")) {
                            event.currentTarget.closest("details")?.removeAttribute("open");
                        }
                    }}
                >
                    {properties.navigation()}
                </div>
            </details>

            <div {...stylex.attrs(styles.location)}>{properties.location()}</div>
            <div {...stylex.attrs(styles.toolbarTools)}>
                {properties.tokenCount !== undefined && (
                    <>
                        <span {...stylex.attrs(styles.statistic)} title="Estimated reading time">
                            {formatReadTime(properties.tokenCount)}
                        </span>
                        <span {...stylex.attrs(styles.statistic, styles.tokenCount)}>
                            {formatTokenCount(properties.tokenCount)}
                        </span>
                    </>
                )}
                <SourceActions commands={properties.sourceCommands} />
            </div>
        </header>
    );
}

/** Format an approximate token count for the compact article toolbar. */
function formatTokenCount(tokenCount: number) {
    // keep exact counts legible for short pages
    if (tokenCount < 1_000) {
        return `${tokenCount} tokens`;
    }

    // retain one useful decimal without trailing zeroes
    const thousands = Math.round(tokenCount / 100) / 10;

    return `${thousands}k tokens`;
}

/** The media query for narrow screens that stack the sidebar. */
const narrow = "@media (width < 60rem)";
/** The media query for compact screens that hide the toolbar labels. */
const compact = "@media (width < 52rem)";

/** Shared reader styles. */
const styles = stylex.create({
    location: {
        minWidth: 0,
        overflow: "hidden",
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 60rem)": { display: "none" },
    },
    menuBody: {
        backgroundColor: color.background,
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        boxShadow: "0 12px 32px rgb(0 0 0 / 12%)",
        display: "grid",
        fontFamily: fontFamily.default,
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
        color: color.foreground,
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
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        color: color.foreground,
        display: "grid",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-label)",
        gap: "0.75rem",
        gridTemplateColumns: "minmax(0, 1fr) auto",
        height: tokens.bar,
        paddingInline: tokens.inset,
        position: "relative",
        [narrow]: { gridTemplateColumns: "auto minmax(0, 1fr) auto" },
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
        color: color.foreground,
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

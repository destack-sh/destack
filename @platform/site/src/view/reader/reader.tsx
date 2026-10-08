import { frame } from "../layout/frame.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { formatReadTime } from "../content/presentation";
import { Icon } from "@destack/icon";
import { text } from "@destack/theme/text";
import caretDown from "@destack/icon/phosphor/caret-down";
import { Popover, PopoverContent, PopoverTrigger } from "@destack/ui/popover";
import { createSignal, type JSX, onSettled } from "@destack/view";
import * as style from "@destack/style";

import type { PageSource } from "../content/source";
import { createPageSourceCommands, type PageSourceCommands, SourceActions } from "./source";
import { lattice } from "../layout/lattice.stylex";
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
    tokenCount?: number | undefined;

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
    tokenCount?: number | undefined;
};

/** Render the common blog and documentation reading frame. */
export function Reader(properties: ReaderProperties) {
    // hold the article and its source commands
    let article: HTMLElement | undefined;
    const sourceCommands = createPageSourceCommands(properties.source);

    // render diagrams after the article mounts
    onSettled(() => {
        if (!article) {
            throw new TypeError("the reader rendered without its article");
        }

        return renderDiagrams(article);
    });

    // align direct links after responsive layout and webfonts settle
    onSettled(() => {
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
            {...style.attrs(lattice.frame, lattice.ruleBottom, publicationStyles.layout)}
            data-publication={properties.publication}
        >
            <aside {...style.attrs(text.subheadline, lattice.ruleRight, publicationStyles.sidebar)}>
                <div {...style.attrs(publicationStyles.sidebarContent)}>
                    {properties.navigation()}
                </div>
            </aside>

            <article
                ref={article}
                {...style.attrs(publicationStyles.article)}
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

                <div {...style.attrs(publicationStyles.body)}>{properties.children}</div>
                {properties.pagination?.()}
            </article>
        </div>
    );
}

/** Render one toolbar at its responsive DOM position. */
function ReaderToolbar(properties: ReaderToolbarProperties) {
    // hold whether the narrow contents menu is open
    const [isMenuOpen, setIsMenuOpen] = createSignal(false);

    return (
        <header {...style.attrs(text.subheadline, styles.toolbar)}>
            <Popover open={isMenuOpen()} onOpenChange={setIsMenuOpen}>
                <PopoverTrigger variant="ghost" size="sm" xstyle={styles.menu}>
                    Contents
                    <Icon icon={caretDown} />
                </PopoverTrigger>
                <PopoverContent side="bottom" align="start" xstyle={text.subheadline}>
                    <div
                        {...style.attrs(styles.menuBody)}
                        onClick={(event) => {
                            // close once a link in the contents is followed
                            if (event.target instanceof Element && event.target.closest("a")) {
                                setIsMenuOpen(false);
                            }
                        }}
                    >
                        {properties.navigation()}
                    </div>
                </PopoverContent>
            </Popover>

            <div {...style.attrs(styles.location)}>{properties.location()}</div>
            <div {...style.attrs(styles.toolbarTools)}>
                {properties.tokenCount !== undefined && (
                    <>
                        <span
                            {...style.attrs(styles.statistic, styles.readTime)}
                            title="Estimated reading time"
                        >
                            {formatReadTime(properties.tokenCount)}
                        </span>
                        <span {...style.attrs(styles.statistic, styles.tokenCount)}>
                            {formatTokenCount(properties.tokenCount)}
                        </span>
                    </>
                )}
                <SourceActions commands={properties.sourceCommands} />
            </div>
        </header>
    );
}

/** Scroll to the heading the URL names once layout settles. */
function scrollAfterLayout() {
    window.requestAnimationFrame(scrollToHeading);
}

/** Scroll to the heading the URL names. */
function scrollToHeading() {
    const identifier = decodeURIComponent(window.location.hash.slice(1));
    if (identifier === "") {
        return;
    }

    document.getElementById(identifier)?.scrollIntoView({ block: "start" });
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
/** The media query for the smallest phones, which keep only the location and the source actions. */
const tiny = "@media (width < 24rem)";

/** Shared reader styles. */
const styles = style.create({
    location: {
        minWidth: 0,
        overflow: "hidden",
    },
    menu: {
        minWidth: 0,
        "@media (min-width: 60rem)": { display: "none" },
    },
    menuBody: {
        display: "grid",
        gap: "1.25rem",
        maxHeight: "min(32rem, 65svh)",
        overflowY: "auto",
        overscrollBehavior: "contain",
        width: "min(22rem, calc(100vw - 2rem))",
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.foreground,
        display: "grid",
        fontFamily: font.text,
        gap: "0.75rem",
        gridTemplateColumns: "minmax(0, 1fr) auto",
        height: frame.bar,
        paddingInline: frame.inset,
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
    },
    tokenCount: {
        "::before": {
            content: "·",
            marginInlineEnd: "0.75rem",
        },
        [compact]: {
            display: "none",
        },
    },
    readTime: {
        [tiny]: {
            display: "none",
        },
    },
});

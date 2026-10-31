import { frame } from "../layout/frame.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { formatReadTime } from "../content/presentation";
import { text } from "@destack/theme/text";
import type { JSX } from "@destack/view";
import Contents from "../islands/contents";
import ReaderBehavior from "../islands/reader";
import Source from "../islands/source";
import type { BrowserEnvironment } from "@destack/environment/browser";
import * as style from "@destack/style";

import type { PageSource } from "../content/source";
import { lattice } from "../layout/lattice.stylex";
import { publicationStyles } from "./publication.stylex";
import { screen } from "../layout/screen.stylex";
import { requestFrame } from "@destack/view/primitives/raf";

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

    /** The portable source files for the current page. */
    source: PageSource;

    /** The approximate token count shown for authored pages. */
    tokenCount?: number | undefined;
};

/** Render the common blog and documentation reading frame. */
export function Reader(properties: ReaderProperties) {
    return (
        <div
            {...style.attrs(lattice.frame, lattice.ruleBottom, publicationStyles.layout)}
            data-publication={properties.publication}
        >
            <ReaderBehavior />
            <aside {...style.attrs(text.subheadline, lattice.ruleRight, publicationStyles.sidebar)}>
                <div {...style.attrs(publicationStyles.sidebarContent)}>
                    {properties.navigation()}
                </div>
            </aside>

            <article
                {...style.attrs(publicationStyles.article)}
                data-markdown-route={properties.source.markdownRoute}
                data-page-source
                data-text-route={properties.source.textRoute}
            >
                <ReaderToolbar
                    location={properties.location}
                    navigation={properties.navigation}
                    source={properties.source}
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
    return (
        <header {...style.attrs(text.subheadline, styles.toolbar)}>
            <Contents>{properties.navigation()}</Contents>

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
                <Source source={properties.source} />
            </div>
        </header>
    );
}

/** Scroll to the heading the URL names once layout settles, on the display's next frame. */
export function scrollAfterLayout(environment: Pick<BrowserEnvironment, "display" | "navigation">) {
    requestFrame(() => scrollToHeading(environment.navigation), environment.display);
}

/** Scroll to the heading the URL names. */
function scrollToHeading(navigation: BrowserEnvironment["navigation"]) {
    // read the heading from the address, none without a fragment
    const { hash } = new URL(navigation.currentEntry.url);
    const identifier = decodeURIComponent(hash.slice(1));
    if (identifier === "") {
        return;
    }

    // scroll it to the top
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
    toolbar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.foreground,
        display: "grid",
        fontFamily: font.text,
        gap: "0.75rem",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) auto",
            [screen.belowReader]: "auto minmax(0, 1fr) auto",
        },
        height: frame.bar,
        paddingInline: frame.inset,
        position: "relative",
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
        display: { default: null, [compact]: "none" },
    },
    readTime: {
        display: { default: null, [tiny]: "none" },
    },
});

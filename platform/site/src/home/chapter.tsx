import * as stylex from "@stylexjs/stylex";
import { createSignal, type Setter } from "solid-js";

import type { HomeExample, HomeListing } from "../content/home";
import { homeHighlights } from "../generated/home-highlights";
import { tokens } from "../style/tokens.stylex";

const mobile = "@media (max-width: 767px)";

type ChapterProps = {
    /// The example shown by this chapter.
    example: HomeExample;

    /// The zero-based chapter index.
    index: number;
};

/// Render one numbered product chapter.
export function Chapter(props: ChapterProps) {
    const number = String(props.index + 1).padStart(2, "0");

    return (
        <section {...stylex.attrs(chapterStyles.chapter)} id={props.example.action}>
            <div {...stylex.attrs(chapterStyles.frame)}>
                <header {...stylex.attrs(chapterStyles.brief)}>
                    <div {...stylex.attrs(chapterStyles.heading)}>
                        <span {...stylex.attrs(chapterStyles.headingNumber)}>{number}</span>
                        <h2 {...stylex.attrs(chapterStyles.headingTitle)}>
                            {props.example.action}
                        </h2>
                    </div>

                    <div {...stylex.attrs(chapterStyles.copy)}>
                        <strong {...stylex.attrs(chapterStyles.claim)}>
                            {props.example.claim}
                        </strong>
                        <p {...stylex.attrs(chapterStyles.description)}>
                            {props.example.description}
                        </p>
                    </div>

                    <div {...stylex.attrs(chapterStyles.comparison)}>
                        <span {...stylex.attrs(chapterStyles.comparisonLabel)}>like</span>
                        <ul
                            aria-label="Comparable technologies"
                            {...stylex.attrs(chapterStyles.comparisonList)}
                        >
                            {props.example.like.map((item, index) => (
                                <li {...stylex.attrs(chapterStyles.comparisonItem)}>
                                    {index > 0 && (
                                        <span
                                            aria-hidden="true"
                                            {...stylex.attrs(chapterStyles.comparisonSeparator)}
                                        >
                                            ·
                                        </span>
                                    )}
                                    {item}
                                </li>
                            ))}
                        </ul>
                    </div>
                </header>

                <Listings example={props.example} />
            </div>
        </section>
    );
}

/// Render the technical listings for one product chapter.
function Listings(props: { example: HomeExample }) {
    const highlighted = homeHighlights[props.example.action];
    const editors = props.example.editors.map((listing, index) => ({
        lines: highlighted.editors[index],
        listing,
    }));

    return (
        <div {...stylex.attrs(listingStyles.listings)}>
            <Editors action={props.example.action} listings={editors} />
            {props.example.output !== undefined && highlighted.output !== null && (
                <Output
                    isAttached={editors.length > 0}
                    listing={props.example.output}
                    lines={highlighted.output}
                />
            )}
        </div>
    );
}

type HighlightedListing = {
    /// The listing content.
    listing: HomeListing;

    /// Highlighted HTML for each listing line.
    lines: readonly string[];
};

type EditorsProps = {
    /// The chapter action used to identify the tabs.
    action: string;

    /// The chapter editor listings.
    listings: readonly HighlightedListing[];
};

/// Render one editor or a tabbed group of editors.
function Editors(props: EditorsProps) {
    if (props.listings.length === 0) {
        return null;
    }
    if (props.listings.length === 1) {
        const [{ listing, lines }] = props.listings;

        return <Editor listing={listing} lines={lines} />;
    }

    return <EditorTabs action={props.action} listings={props.listings} />;
}

/// Render a tabbed group of editors.
function EditorTabs(props: EditorsProps) {
    const [activeIndex, setActiveIndex] = createSignal(0);

    return (
        <div {...stylex.attrs(listingStyles.editors)}>
            <div
                aria-label={`${props.action} editors`}
                role="tablist"
                {...stylex.attrs(listingStyles.tabs)}
            >
                {props.listings.map(({ listing }, index) => {
                    const tabId = `${props.action}-tab-${index}`;
                    const panelId = `${props.action}-panel-${index}`;

                    return (
                        <button
                            aria-controls={panelId}
                            aria-selected={activeIndex() === index}
                            id={tabId}
                            onClick={() => setActiveIndex(index)}
                            onKeyDown={(event) =>
                                selectListingTab(
                                    event,
                                    index,
                                    props.listings.length,
                                    setActiveIndex,
                                )
                            }
                            role="tab"
                            tabIndex={activeIndex() === index ? 0 : -1}
                            type="button"
                            {...stylex.attrs(
                                listingStyles.tab,
                                activeIndex() === index && listingStyles.activeTab,
                            )}
                        >
                            {listing.title}
                        </button>
                    );
                })}
            </div>

            {props.listings.map(({ listing, lines }, index) => (
                <div
                    aria-labelledby={`${props.action}-tab-${index}`}
                    hidden={activeIndex() !== index}
                    id={`${props.action}-panel-${index}`}
                    role="tabpanel"
                    tabIndex={0}
                >
                    <ListingBody listing={listing} lines={lines} />
                </div>
            ))}
        </div>
    );
}

type ListingProps = {
    /// The listing description.
    listing: HomeListing;

    /// Highlighted HTML for each listing line.
    lines: readonly string[];
};

type OutputProps = ListingProps & {
    /// Whether the output follows an editor.
    isAttached: boolean;
};

/// Render one file or configuration editor.
function Editor(props: ListingProps) {
    return (
        <figure data-publication-listing {...stylex.attrs(listingStyles.listing)}>
            <figcaption data-publication-caption {...stylex.attrs(listingStyles.caption)}>
                <span data-publication-caption-title>{props.listing.title}</span>
            </figcaption>

            <ListingBody listing={props.listing} lines={props.lines} />
        </figure>
    );
}

/// Render one terminal or diagnostic output beneath the editors.
function Output(props: OutputProps) {
    return (
        <figure
            data-publication-listing
            {...stylex.attrs(
                listingStyles.output,
                props.isAttached && listingStyles.attachedOutput,
            )}
        >
            <figcaption data-publication-caption {...stylex.attrs(listingStyles.outputTitle)}>
                <span data-publication-caption-title>{props.listing.title}</span>
            </figcaption>
            <ListingBody listing={props.listing} lines={props.lines} />
        </figure>
    );
}

/// Render the highlighted body shared by editors and outputs.
function ListingBody(props: ListingProps) {
    const hasLineNumbers =
        props.listing.language === "destack" || props.listing.language === "json";

    return (
        <div data-publication-body>
            <ol
                aria-label={props.listing.title}
                data-publication-lines
                {...stylex.attrs(listingStyles.lines)}
            >
                {props.lines.map((line, index) => (
                    <li data-publication-line {...stylex.attrs(listingStyles.line)}>
                        <span
                            aria-hidden="true"
                            data-publication-gutter
                            {...stylex.attrs(listingStyles.lineNumber)}
                        >
                            {hasLineNumbers ? index + 1 : ""}
                        </span>
                        <code
                            data-publication-code
                            {...stylex.attrs(listingStyles.code)}
                            innerHTML={line || " "}
                        />
                    </li>
                ))}
            </ol>
        </div>
    );
}

/// Select an adjacent listing tab with standard keyboard controls.
function selectListingTab(
    event: KeyboardEvent & { currentTarget: HTMLButtonElement },
    index: number,
    count: number,
    setActiveIndex: Setter<number>,
): void {
    // resolve a standard tab navigation key
    const nextIndex =
        event.key === "ArrowRight"
            ? (index + 1) % count
            : event.key === "ArrowLeft"
              ? (index - 1 + count) % count
              : event.key === "Home"
                ? 0
                : event.key === "End"
                  ? count - 1
                  : undefined;

    // ignore keys owned by the button or page
    if (nextIndex === undefined) {
        return;
    }

    // select the adjacent listing
    event.preventDefault();
    setActiveIndex(nextIndex);

    // move focus with the selected tab
    const tabs =
        event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>("[role='tab']");
    tabs?.[nextIndex]?.focus();
}

/// Product chapter styles.
const chapterStyles = stylex.create({
    brief: {
        alignContent: "start",
        display: "grid",
        gap: "1rem",
        minWidth: 0,
        position: "sticky",
        top: "1.5rem",
        [mobile]: {
            order: 1,
            position: "static",
        },
    },
    chapter: {
        width: "100%",
    },
    claim: {
        color: tokens.ink,
        fontSize: "1.08rem",
        fontWeight: 680,
        lineHeight: 1.05,
        maxWidth: "18rem",
    },
    copy: {
        alignContent: "start",
        display: "grid",
        gap: "0.65rem",
    },
    description: {
        color: tokens.soft,
        fontSize: "0.95rem",
        lineHeight: 1.5,
        margin: 0,
        maxWidth: "20rem",
    },
    frame: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        color: tokens.ink,
        display: "grid",
        gap: "clamp(2rem, 5vw, 4rem)",
        gridTemplateColumns: "minmax(16rem, 0.85fr) minmax(0, 1.5fr)",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        paddingBlock: "clamp(1.25rem, 2.5vw, 1.75rem)",
        position: "relative",
        width: `calc(100% - ${tokens.gutterLeft} - ${tokens.gutterRight})`,
        zIndex: 1,
        [mobile]: {
            gap: "1.5rem",
            gridTemplateColumns: "minmax(0, 1fr)",
        },
    },
    heading: {
        alignItems: "center",
        display: "grid",
        fontFamily: tokens.monoFont,
        gap: "0.8rem",
        gridTemplateColumns: "auto minmax(0, 1fr)",
        textTransform: "uppercase",
    },
    headingNumber: {
        color: tokens.orange,
        fontSize: "1.15rem",
        fontWeight: 600,
        lineHeight: 1,
    },
    headingTitle: {
        color: tokens.ink,
        fontFamily: tokens.monoFont,
        fontSize: "1.15rem",
        fontWeight: 600,
        letterSpacing: "0.06em",
        lineHeight: 1,
        margin: 0,
        textTransform: "uppercase",
    },
    comparison: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 400,
        gap: "0.5rem 0.75rem",
        letterSpacing: "0.035em",
    },
    comparisonItem: {
        color: tokens.ink,
        whiteSpace: "nowrap",
    },
    comparisonLabel: {
        color: tokens.orange,
        textTransform: "uppercase",
    },
    comparisonList: {
        display: "flex",
        flexWrap: "wrap",
        gap: "0.35rem 0.6rem",
        listStyle: "none",
        margin: 0,
        minWidth: 0,
        padding: 0,
    },
    comparisonSeparator: {
        color: tokens.line,
        marginRight: "0.6rem",
    },
});

/// Technical listing styles.
const listingStyles = stylex.create({
    attachedOutput: {
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
    },
    editors: {
        minWidth: 0,
    },
    listing: {
        backgroundColor: tokens.cream,
        color: tokens.ink,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        margin: 0,
        minWidth: 0,
    },
    listings: {
        backgroundColor: tokens.cream,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.hairline,
        minWidth: 0,
        [mobile]: {
            order: 2,
        },
    },
    caption: {
        margin: 0,
    },
    code: {
        whiteSpace: "pre",
    },
    line: {
        minWidth: "max-content",
    },
    lineNumber: {
        userSelect: "none",
    },
    lines: {
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    output: {
        backgroundColor: tokens.cream,
        color: tokens.ink,
        margin: 0,
        minWidth: 0,
    },
    outputTitle: {
        color: tokens.soft,
        margin: 0,
    },
    tabs: {
        alignItems: "stretch",
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        minHeight: tokens.publicationRow,
        minWidth: 0,
        overflowX: "auto",
    },
    tab: {
        ":focus-visible": {
            outline: `2px solid ${tokens.orange}`,
            outlineOffset: "-4px",
        },
        ":hover": {
            color: tokens.ink,
        },
        appearance: "none",
        backgroundColor: "transparent",
        borderBottomColor: "transparent",
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.hairline,
        borderLeftWidth: 0,
        borderRightWidth: 0,
        borderTopWidth: 0,
        color: tokens.soft,
        cursor: "pointer",
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        fontWeight: 600,
        letterSpacing: "0.04em",
        margin: 0,
        padding: "0.75rem 1rem",
        textAlign: "left",
        textTransform: "uppercase",
        whiteSpace: "nowrap",
    },
    activeTab: {
        borderBottomColor: tokens.orange,
        color: tokens.ink,
    },
});

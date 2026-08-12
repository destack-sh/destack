import * as stylex from "@stylexjs/stylex";

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
                        <h2 {...stylex.attrs(chapterStyles.headingTitle)}>{props.example.action}</h2>
                    </div>

                    <div {...stylex.attrs(chapterStyles.copy)}>
                        <strong {...stylex.attrs(chapterStyles.claim)}>{props.example.claim}</strong>
                        <p {...stylex.attrs(chapterStyles.description)}>{props.example.description}</p>
                    </div>

                    <div {...stylex.attrs(chapterStyles.comparison)}>
                        <span {...stylex.attrs(chapterStyles.comparisonLabel)}>like</span>
                        <ul aria-label="Comparable technologies" {...stylex.attrs(chapterStyles.comparisonList)}>
                            {props.example.like.map((item, index) => (
                                <li {...stylex.attrs(chapterStyles.comparisonItem)}>
                                    {index > 0 && (
                                        <span aria-hidden="true" {...stylex.attrs(chapterStyles.comparisonSeparator)}>
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
    const highlighted = homeHighlights[props.example.action] as readonly (readonly string[])[];

    return (
        <div {...stylex.attrs(listingStyles.listings)}>
            {props.example.listings.map((listing, index) => (
                <Listing listing={listing} lines={highlighted[index]} />
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

/// Render one file, command, configuration, or result listing.
function Listing(props: ListingProps) {
    const hasLineNumbers = props.listing.language === "destack" || props.listing.language === "json";

    return (
        <figure
            {...stylex.attrs(
                listingStyles.listing,
                props.listing.width === "full" && listingStyles.full,
            )}
        >
            <figcaption {...stylex.attrs(listingStyles.caption)}>
                <span aria-hidden="true" />
                <span>{props.listing.title}</span>
            </figcaption>

            <div {...stylex.attrs(listingStyles.body)} data-syntax>
                <ol aria-label={props.listing.title} {...stylex.attrs(listingStyles.lines)}>
                    {props.lines.map((line, index) => (
                        <li {...stylex.attrs(listingStyles.line)}>
                            <span aria-hidden="true" {...stylex.attrs(listingStyles.lineNumber)}>
                                {hasLineNumbers ? index + 1 : ""}
                            </span>
                            <code {...stylex.attrs(listingStyles.code)} innerHTML={line || " "} />
                        </li>
                    ))}
                </ol>
            </div>
        </figure>
    );
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
        borderTopWidth: tokens.stroke,
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
    listing: {
        backgroundColor: tokens.code,
        color: tokens.cream,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        margin: 0,
        minWidth: 0,
    },
    listings: {
        backgroundColor: tokens.ink,
        borderBottomColor: tokens.ink,
        borderBottomStyle: "solid",
        borderBottomWidth: tokens.stroke,
        borderTopColor: tokens.ink,
        borderTopStyle: "solid",
        borderTopWidth: tokens.stroke,
        display: "grid",
        gap: tokens.stroke,
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        minWidth: 0,
        [mobile]: {
            gridTemplateColumns: "minmax(0, 1fr)",
            order: 2,
        },
    },
    body: {
        fontFamily: tokens.monoFont,
        fontSize: "0.84rem",
        lineHeight: "1.55rem",
        overflowX: "auto",
        paddingBlock: "0.25rem 0.8rem",
    },
    caption: {
        alignItems: "baseline",
        backgroundColor: tokens.code,
        color: tokens.cream,
        display: "grid",
        fontFamily: tokens.monoFont,
        fontSize: "0.8rem",
        fontWeight: 600,
        gridTemplateColumns: "2.2rem minmax(0, 1fr)",
        letterSpacing: "0.04em",
        padding: "0.7rem 1rem 0.25rem",
        textTransform: "uppercase",
    },
    code: {
        whiteSpace: "pre",
    },
    full: {
        gridColumn: "1 / -1",
    },
    line: {
        display: "grid",
        gridTemplateColumns: "2.2rem minmax(0, 1fr)",
        minWidth: "max-content",
    },
    lineNumber: {
        color: "#66858d",
        userSelect: "none",
    },
    lines: {
        listStyle: "none",
        margin: 0,
        paddingInline: "1rem",
    },
});

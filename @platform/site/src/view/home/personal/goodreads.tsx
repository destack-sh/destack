import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { goodreads } from "./goodreads.stylex";

/** The header's links. */
const links: readonly string[] = ["Home", "My Books", "Browse ▾", "Community ▾"];

/** Your favourite books as their covers: colour and title. */
const favourites: readonly (readonly [cover: string, title: string])[] = [
    ["#1f3b5a", "The Dispossessed"],
    ["#c9b27c", "Piranesi"],
    ["#8a2b2b", "The Mythical Man-Month"],
    ["#2f5d3a", "Seeing Like a State"],
    ["#e2b63b", "Gödel, Escher, Bach"],
    ["#5a3d7a", "A Pattern Language"],
];

/** Your shelves and how many books each has. */
const shelves: readonly (readonly [name: string, count: number])[] = [
    ["read", 98],
    ["currently-reading", 2],
    ["to-read", 24],
    ["sci-fi", 31],
    ["programming", 17],
    ["history", 12],
    ["favorites", 9],
    ["design", 8],
];

/** Your latest reads: cover colour, title, author and your stars. */
const updates: readonly (readonly [cover: string, title: string, author: string, stars: number])[] =
    [
        ["#1f3b5a", "The Dispossessed", "Ursula K. Le Guin", 5],
        ["#c9b27c", "Piranesi", "Susanna Clarke", 4],
    ];

/** Draw your Goodreads profile: your favourite books, shelves, challenge and latest reads. */
export function Goodreads() {
    return (
        <div data-component="GoodreadsPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <span {...style.attrs(styles.wordmark)}>
                    good<b>reads</b>
                </span>
                <span {...style.attrs(styles.links)}>
                    {links.map((link) => (
                        <span>{link}</span>
                    ))}
                </span>
                <span {...style.attrs(styles.search)}>Search books</span>
                <span {...style.attrs(styles.me)}>F</span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* profile */}
                <main {...style.attrs(styles.main)}>
                    <div {...style.attrs(styles.profile)}>
                        <span {...style.attrs(styles.photoColumn)}>
                            <span {...style.attrs(styles.photo)}>F</span>
                            <span {...style.attrs(styles.link, styles.small)}>
                                124 ratings (4.10 avg)
                            </span>
                            <span {...style.attrs(styles.link, styles.small)}>38 reviews</span>
                        </span>
                        <span {...style.attrs(styles.details)}>
                            <h1 {...style.attrs(styles.name)}>Florian</h1>
                            <span {...style.attrs(styles.fact)}>
                                <b>Details</b> Zurich, Switzerland
                            </span>
                            <span {...style.attrs(styles.fact)}>
                                <b>Activity</b> Joined in March 2014
                            </span>
                        </span>
                    </div>

                    <span {...style.attrs(styles.heading)}>FLORIAN’S FAVORITE BOOKS</span>
                    <span {...style.attrs(styles.covers)}>
                        {favourites.map(([cover, title]) => (
                            <span
                                style={{ "background-color": cover }}
                                {...style.attrs(styles.cover)}
                            >
                                {title}
                            </span>
                        ))}
                    </span>

                    <span {...style.attrs(styles.heading)}>FLORIAN’S BOOKSHELVES</span>
                    <span {...style.attrs(styles.shelves)}>
                        {shelves.map(([name, count]) => (
                            <span {...style.attrs(styles.link)}>
                                {name} ({count})
                            </span>
                        ))}
                    </span>
                </main>

                {/* challenge and updates */}
                <aside {...style.attrs(styles.side)}>
                    <span {...style.attrs(styles.heading)}>2025 READING CHALLENGE</span>
                    <span {...style.attrs(styles.challenge)}>
                        <span {...style.attrs(styles.challengeBadge)}>
                            <b>2025</b>
                            <span>READING</span>
                            <span>CHALLENGE</span>
                        </span>
                        <span {...style.attrs(styles.challengeText)}>
                            Florian has read 21 books of his goal of 30!
                            <span {...style.attrs(styles.track)}>
                                <span {...style.attrs(styles.fill)} />
                            </span>
                            <span {...style.attrs(styles.muted, styles.small)}>21 of 30 (70%)</span>
                        </span>
                    </span>
                    <span {...style.attrs(styles.heading)}>RECENT UPDATES</span>
                    {updates.map(([cover, title, author, stars]) => (
                        <span {...style.attrs(styles.update)}>
                            <span
                                style={{ "background-color": cover }}
                                {...style.attrs(styles.thumb)}
                            />
                            <span {...style.attrs(styles.updateText)}>
                                <span>
                                    Florian rated <span {...style.attrs(styles.link)}>{title}</span>
                                </span>
                                <span {...style.attrs(styles.muted, styles.small)}>
                                    by {author}
                                </span>
                                <span {...style.attrs(styles.stars)}>{"★".repeat(stars)}</span>
                            </span>
                        </span>
                    ))}
                </aside>
            </div>
        </div>
    );
}

/** The Goodreads page styles. */
const styles = style.create({
    page: {
        backgroundColor: goodreads.surface,
        color: goodreads.ink,
        display: "grid",
        fontFamily: goodreads.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    head: {
        alignItems: "center",
        backgroundColor: goodreads.header,
        borderBottomColor: goodreads.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        boxShadow: `0 1px 2px ${goodreads.spine}`,
        display: "flex",
        gap: "20px",
        height: "50px",
        paddingInline: { default: "24px", [site.narrow]: "12px" },
    },
    wordmark: {
        color: goodreads.brown,
        fontFamily: goodreads.serif,
        fontSize: "22px",
        letterSpacing: "-0.03em",
    },
    links: {
        color: goodreads.ink,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "14px",
        gap: "18px",
    },
    search: {
        backgroundColor: goodreads.surface,
        borderColor: goodreads.line,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: goodreads.muted,
        flexGrow: 1,
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: goodreads.onDark,
        display: "inline-flex",
        flexShrink: 0,
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    body: {
        alignItems: "start",
        display: "grid",
        gap: "28px",
        gridTemplateColumns: { default: "minmax(0, 1fr) 200px", [site.narrow]: "minmax(0, 1fr)" },
        overflow: "hidden",
        paddingBlock: "20px",
        paddingInline: { default: "28px", [site.narrow]: "12px" },
    },
    main: {
        display: "grid",
        gap: "10px",
        minWidth: 0,
    },
    profile: {
        display: "flex",
        gap: "18px",
        paddingBlockEnd: "8px",
    },
    photoColumn: {
        alignItems: "center",
        display: "grid",
        gap: "3px",
        justifyItems: "center",
    },
    photo: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: goodreads.onDark,
        display: "inline-flex",
        fontSize: "36px",
        height: "92px",
        justifyContent: "center",
        marginBlockEnd: "6px",
        width: "92px",
    },
    details: {
        alignContent: "start",
        display: "grid",
        flexGrow: 1,
        gap: "8px",
    },
    name: {
        borderBottomColor: goodreads.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: goodreads.brown,
        fontFamily: goodreads.serif,
        fontSize: "20px",
        fontWeight: 700,
        margin: 0,
        paddingBlockEnd: "6px",
    },
    fact: {
        display: "flex",
        gap: "16px",
    },
    heading: {
        borderBottomColor: goodreads.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: goodreads.brown,
        fontSize: "11px",
        fontWeight: 700,
        letterSpacing: "0.03em",
        paddingBlockEnd: "5px",
        paddingBlockStart: "8px",
    },
    covers: {
        display: "flex",
        gap: "6px",
        overflow: "hidden",
    },
    cover: {
        boxShadow: `inset -3px 0 0 ${goodreads.spine}`,
        color: goodreads.onDark,
        flexShrink: 0,
        fontFamily: goodreads.serif,
        fontSize: "7px",
        height: "72px",
        lineHeight: 1.2,
        overflow: "hidden",
        padding: "5px",
        width: "48px",
    },
    shelves: {
        display: "grid",
        gap: "3px",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    },
    side: {
        alignContent: "start",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "10px",
    },
    challenge: {
        display: "flex",
        gap: "10px",
    },
    challengeBadge: {
        alignContent: "center",
        backgroundColor: goodreads.challenge,
        color: goodreads.onDark,
        display: "grid",
        flexShrink: 0,
        fontSize: "8px",
        fontWeight: 700,
        height: "74px",
        justifyItems: "center",
        width: "64px",
    },
    challengeText: {
        display: "grid",
        fontSize: "12px",
        gap: "6px",
    },
    track: {
        backgroundColor: goodreads.track,
        display: "block",
        height: "12px",
    },
    fill: {
        backgroundColor: goodreads.progress,
        display: "block",
        height: "100%",
        width: "70%",
    },
    update: {
        display: "flex",
        gap: "8px",
    },
    thumb: {
        boxShadow: `inset -2px 0 0 ${goodreads.spine}`,
        flexShrink: 0,
        height: "48px",
        width: "32px",
    },
    updateText: {
        display: "grid",
        fontSize: "12px",
        gap: "2px",
    },
    stars: {
        color: goodreads.star,
        letterSpacing: "1px",
    },
    link: {
        color: goodreads.teal,
    },
    small: {
        fontSize: "11px",
    },
    muted: {
        color: goodreads.muted,
    },
});

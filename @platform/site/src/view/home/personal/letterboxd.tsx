import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { letterboxd } from "./letterboxd.stylex";

/** The header's links. */
const links: readonly string[] = ["ACTIVITY", "FILMS", "LISTS", "MEMBERS", "JOURNAL"];

/** The profile's tabs, Diary chosen. */
const tabs: readonly string[] = ["Activity", "Films", "Diary", "Reviews", "Watchlist", "Lists"];

/** The diary's columns. */
const columns: readonly string[] = ["Month", "Day", "Film", "Released", "Rating", "Like"];

/** Your diary, newest first: day, poster art, film, year, stars and whether you liked it. */
const diary: readonly (readonly [
    day: string,
    poster: string,
    film: string,
    year: string,
    stars: string,
    isLiked: boolean,
])[] = [
    [
        "19",
        "linear-gradient(170deg, #9cc5c7 0%, #3c6e71 55%, #1d3536 100%)",
        "Perfect Days",
        "2023",
        "★★★★½",
        true,
    ],
    [
        "15",
        "linear-gradient(170deg, #e7dcc8 0%, #8a7f6a 55%, #2a2722 100%)",
        "Anatomy of a Fall",
        "2023",
        "★★★★",
        false,
    ],
    [
        "12",
        "linear-gradient(170deg, #f0c8a8 0%, #c26a5a 50%, #22313f 100%)",
        "Past Lives",
        "2023",
        "★★★★",
        true,
    ],
    [
        "08",
        "linear-gradient(170deg, #9fd3e6 0%, #3b7fa3 55%, #10283a 100%)",
        "Aftersun",
        "2022",
        "★★★★½",
        true,
    ],
    [
        "04",
        "linear-gradient(170deg, #8c8a5b 0%, #4b4a2e 60%, #1e1d12 100%)",
        "Stalker",
        "1979",
        "★★★★★",
        true,
    ],
    [
        "01",
        "linear-gradient(170deg, #7f8c8d 0%, #2b3337 60%, #101820 100%)",
        "The Zone of Interest",
        "2023",
        "★★★½",
        false,
    ],
];

/** Draw your Letterboxd diary: the films you watched this month, rated and liked. */
export function Letterboxd() {
    return (
        <div data-component="LetterboxdPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <span {...style.attrs(styles.logo)}>
                    <span {...style.attrs(styles.dots)}>
                        <span {...style.attrs(styles.dot, styles.orange)} />
                        <span {...style.attrs(styles.dot, styles.green)} />
                        <span {...style.attrs(styles.dot, styles.blue)} />
                    </span>
                    <b {...style.attrs(styles.wordmark)}>Letterboxd</b>
                </span>
                <span {...style.attrs(styles.links)}>
                    <span {...style.attrs(styles.account)}>
                        <span {...style.attrs(styles.face)}>F</span>
                        FLORIAN ▾
                    </span>
                    {links.map((link) => (
                        <span>{link}</span>
                    ))}
                </span>
                <span {...style.attrs(styles.search)} />
            </header>

            <main {...style.attrs(styles.main)}>
                {/* profile bar */}
                <div {...style.attrs(styles.profile)}>
                    <span {...style.attrs(styles.face)}>F</span>
                    <b {...style.attrs(styles.name)}>Florian</b>
                    <span {...style.attrs(styles.patron)}>PATRON</span>
                    <span {...style.attrs(styles.tabs)}>
                        {tabs.map((tab) => (
                            <span {...style.attrs(styles.tab, tab === "Diary" && styles.tabOn)}>
                                {tab}
                            </span>
                        ))}
                    </span>
                </div>

                {/* diary */}
                <span {...style.attrs(styles.diaryHead)}>
                    DIARY
                    <span {...style.attrs(styles.filters)}>
                        RATING ▾ <span>DIARY YEAR ▾</span> <span>Sort by WATCHED DATE ▾</span>
                    </span>
                </span>
                <div {...style.attrs(styles.table)}>
                    {columns.map((column) => (
                        <span
                            {...style.attrs(
                                styles.th,
                                (column === "Month" || column === "Released") && styles.wideOnly,
                            )}
                        >
                            {column}
                        </span>
                    ))}
                    {diary.map(([day, poster, film, year, stars, isLiked], index) => (
                        <>
                            <span {...style.attrs(styles.td, styles.wideOnly)}>
                                {index === 0 ? (
                                    <span {...style.attrs(styles.month)}>
                                        <span {...style.attrs(styles.rings)}>
                                            <span {...style.attrs(styles.ringPin)} />
                                            <span {...style.attrs(styles.ringPin)} />
                                            <span {...style.attrs(styles.ringPin)} />
                                        </span>
                                        <b {...style.attrs(styles.monthName)}>OCT</b>
                                        <span {...style.attrs(styles.monthYear)}>2025</span>
                                    </span>
                                ) : undefined}
                            </span>
                            <span {...style.attrs(styles.td, styles.day)}>{day}</span>
                            <span {...style.attrs(styles.td, styles.film)}>
                                <span
                                    style={{ "background-image": poster }}
                                    {...style.attrs(styles.poster)}
                                />
                                <b {...style.attrs(styles.title)}>{film}</b>
                            </span>
                            <span {...style.attrs(styles.td, styles.wideOnly, styles.year)}>
                                {year}
                            </span>
                            <span {...style.attrs(styles.td, styles.stars)}>{stars}</span>
                            <span {...style.attrs(styles.td, styles.like)}>
                                {isLiked ? "♥" : ""}
                            </span>
                        </>
                    ))}
                </div>
            </main>
        </div>
    );
}

/** The Letterboxd page styles. */
const styles = style.create({
    page: {
        backgroundImage: letterboxd.surface,
        color: letterboxd.ink,
        colorScheme: "dark",
        display: "grid",
        fontFamily: letterboxd.face,
        fontSize: "12px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    head: {
        alignItems: "center",
        backgroundColor: letterboxd.header,
        display: "flex",
        gap: "20px",
        height: "56px",
        paddingInline: { default: "28px", [site.narrow]: "12px" },
    },
    logo: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    dots: {
        display: "flex",
    },
    dot: {
        borderRadius: "50%",
        height: "15px",
        width: "15px",
    },
    orange: {
        backgroundColor: letterboxd.orange,
    },
    green: {
        backgroundColor: letterboxd.green,
        marginInline: "-2px",
    },
    blue: {
        backgroundColor: letterboxd.blue,
    },
    wordmark: {
        color: letterboxd.bright,
        fontSize: "21px",
        letterSpacing: "-0.03em",
    },
    links: {
        alignItems: "center",
        color: letterboxd.ink,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "11px",
        fontWeight: 600,
        gap: "14px",
        letterSpacing: "0.08em",
        marginInlineStart: "auto",
    },
    account: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: letterboxd.bright,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    search: {
        backgroundColor: letterboxd.field,
        borderRadius: "999px",
        display: { default: "block", [site.compact]: "none" },
        height: "26px",
        width: "110px",
    },
    main: {
        alignContent: "start",
        display: "grid",
        gap: "12px",
        overflow: "hidden",
        paddingBlock: "18px",
        paddingInline: { default: "28px", [site.narrow]: "12px" },
    },
    profile: {
        alignItems: "center",
        backgroundColor: letterboxd.bar,
        borderRadius: "4px",
        display: "flex",
        gap: "8px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    name: {
        color: letterboxd.bright,
        fontSize: "13px",
    },
    patron: {
        backgroundColor: letterboxd.patron,
        borderRadius: "2px",
        color: letterboxd.bright,
        fontSize: "9px",
        fontWeight: 700,
        paddingBlock: "1px",
        paddingInline: "4px",
    },
    tabs: {
        display: { default: "flex", [site.narrow]: "none" },
        gap: "12px",
        marginInlineStart: "auto",
    },
    tab: {
        fontSize: "12px",
    },
    tabOn: {
        borderBottomColor: letterboxd.bright,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: letterboxd.bright,
    },
    diaryHead: {
        alignItems: "center",
        borderBottomColor: letterboxd.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "12px",
        letterSpacing: "0.08em",
        paddingBlockEnd: "4px",
    },
    filters: {
        color: letterboxd.muted,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "10px",
        gap: "10px",
        marginInlineStart: "auto",
    },
    table: {
        alignItems: "center",
        display: "grid",
        gridTemplateColumns: {
            default: "62px 44px minmax(0, 1fr) 64px 82px 34px",
            [site.narrow]: "34px minmax(0, 1fr) 70px 24px",
        },
    },
    th: {
        borderBottomColor: letterboxd.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: letterboxd.muted,
        fontSize: "10px",
        letterSpacing: "0.06em",
        paddingBlock: "4px",
        textTransform: "uppercase",
    },
    td: {
        alignItems: "center",
        borderBottomColor: letterboxd.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        height: "62px",
    },
    wideOnly: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    month: {
        backgroundColor: letterboxd.bar,
        borderRadius: "3px",
        boxShadow: `0 2px 0 ${letterboxd.header}`,
        display: "grid",
        justifyItems: "center",
        paddingBlockEnd: "4px",
        position: "relative",
        width: "52px",
    },
    rings: {
        display: "flex",
        gap: "10px",
        marginBlockStart: "-4px",
    },
    ringPin: {
        backgroundColor: letterboxd.muted,
        borderRadius: "2px",
        height: "7px",
        width: "3px",
    },
    monthName: {
        color: letterboxd.bright,
        fontSize: "13px",
        fontWeight: 500,
        letterSpacing: "0.04em",
    },
    monthYear: {
        fontSize: "9px",
    },
    day: {
        color: letterboxd.day,
        fontSize: "22px",
        fontWeight: 300,
    },
    film: {
        gap: "10px",
        minWidth: 0,
    },
    poster: {
        borderRadius: "2px",
        boxShadow: `inset 0 0 0 1px ${letterboxd.edge}`,
        flexShrink: 0,
        height: "50px",
        width: "34px",
    },
    title: {
        color: letterboxd.bright,
        fontFamily: letterboxd.serif,
        fontSize: "16px",
        fontWeight: 700,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    year: {
        fontSize: "12px",
    },
    stars: {
        color: letterboxd.green,
        fontSize: "13px",
        letterSpacing: "-1px",
    },
    like: {
        color: letterboxd.orange,
        fontSize: "15px",
    },
});

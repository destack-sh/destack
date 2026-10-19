import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { steam } from "./steam.stylex";

/** The global header's links, Community chosen. */
const links: readonly string[] = ["STORE", "COMMUNITY", "ABOUT", "SUPPORT"];

/** Your recent games: capsule art, name, hours on record, last played, and achievements. */
const games: readonly (readonly [
    capsule: string,
    name: string,
    hours: string,
    played: string,
    achieved: number,
    total: number,
])[] = [
    [
        "linear-gradient(120deg, #6b3f12 0%, #c47a2c 60%, #e8b764 100%)",
        "Factorio",
        "1,204",
        "20 Oct",
        31,
        38,
    ],
    [
        "linear-gradient(120deg, #4f7a5a 0%, #a8c38e 55%, #f1e3b4 100%)",
        "Tiny Glade",
        "31",
        "18 Oct",
        9,
        20,
    ],
    [
        "linear-gradient(120deg, #10183c 0%, #36457a 50%, #e08a3a 100%)",
        "Outer Wilds",
        "27",
        "12 Oct",
        24,
        31,
    ],
];

/** The counts down the profile's right column. */
const counts: readonly (readonly [name: string, count: string])[] = [
    ["Badges", "17"],
    ["Games", "412"],
    ["Screenshots", "88"],
    ["Reviews", "4"],
    ["Groups", "6"],
];

/** Draw your Steam profile: your level, your recent games and their hours. */
export function Steam() {
    return (
        <div data-component="SteamPage" {...style.attrs(styles.page)}>
            {/* global header */}
            <header {...style.attrs(styles.head)}>
                <span {...style.attrs(styles.logo)}>
                    <span
                        style={{ "mask-image": "url(/diagram/steam.svg)" }}
                        {...style.attrs(styles.logoMark)}
                    />
                    STEAM
                </span>
                <span {...style.attrs(styles.links)}>
                    {links.map((link) => (
                        <span {...style.attrs(styles.link, link === "COMMUNITY" && styles.linkOn)}>
                            {link}
                        </span>
                    ))}
                </span>
            </header>

            {/* profile */}
            <main {...style.attrs(styles.column)}>
                <div {...style.attrs(styles.band)}>
                    <span {...style.attrs(styles.avatar)}>F</span>
                    <span {...style.attrs(styles.who)}>
                        <b {...style.attrs(styles.name)}>florian ▾</b>
                        <span {...style.attrs(styles.small)}>Florian · Zurich, Switzerland</span>
                        <span {...style.attrs(styles.summary)}>
                            Factory games and puzzle boxes.
                        </span>
                    </span>
                    <span {...style.attrs(styles.levelBox)}>
                        <span {...style.attrs(styles.levelLine)}>
                            Level <span {...style.attrs(styles.ring)}>23</span>
                        </span>
                        <span {...style.attrs(styles.badge)}>
                            <span {...style.attrs(styles.badgeArt)}>10</span>
                            <span {...style.attrs(styles.badgeText)}>
                                <b>Years of Service</b>
                                <span {...style.attrs(styles.small)}>500 XP</span>
                            </span>
                        </span>
                    </span>
                </div>
                <div {...style.attrs(styles.grid)}>
                    {/* recent activity */}
                    <section {...style.attrs(styles.section)}>
                        <span {...style.attrs(styles.bar)}>
                            Recent Activity
                            <span {...style.attrs(styles.barNote)}>38.2 hours past 2 weeks</span>
                        </span>
                        {games.map(([capsule, name, hours, played, achieved, total]) => (
                            <div {...style.attrs(styles.game)}>
                                <span
                                    style={{ "background-image": capsule }}
                                    {...style.attrs(styles.capsule)}
                                >
                                    {name.toUpperCase()}
                                </span>
                                <span {...style.attrs(styles.gameText)}>
                                    <span {...style.attrs(styles.gameHead)}>
                                        <span {...style.attrs(styles.gameName)}>{name}</span>
                                        <span {...style.attrs(styles.small, styles.right)}>
                                            {hours} hrs on record
                                            <br />
                                            last played on {played}
                                        </span>
                                    </span>
                                    <span {...style.attrs(styles.progress)}>
                                        <span {...style.attrs(styles.small)}>
                                            Achievement Progress {achieved} of {total}
                                        </span>
                                        <span {...style.attrs(styles.track)}>
                                            <span
                                                style={{ width: `${(achieved / total) * 100}%` }}
                                                {...style.attrs(styles.fill)}
                                            />
                                        </span>
                                    </span>
                                </span>
                            </div>
                        ))}
                    </section>

                    {/* status and counts */}
                    <aside {...style.attrs(styles.side)}>
                        <span {...style.attrs(styles.status)}>Currently Online</span>
                        {counts.map(([name, count]) => (
                            <span {...style.attrs(styles.count)}>
                                {name} <span {...style.attrs(styles.countNumber)}>{count}</span>
                            </span>
                        ))}
                    </aside>
                </div>
            </main>
        </div>
    );
}

/** The Steam page styles. */
const styles = style.create({
    page: {
        backgroundImage: steam.backdrop,
        color: steam.ink,
        colorScheme: "dark",
        display: "grid",
        fontFamily: steam.face,
        fontSize: "12px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    head: {
        alignItems: "center",
        backgroundColor: steam.header,
        display: "flex",
        gap: "28px",
        height: "60px",
        paddingInline: { default: "32px", [site.narrow]: "12px" },
    },
    logo: {
        alignItems: "center",
        color: steam.ink,
        display: "flex",
        fontSize: "21px",
        fontWeight: 700,
        gap: "8px",
        letterSpacing: "0.06em",
    },
    logoMark: {
        backgroundColor: steam.ink,
        height: "30px",
        maskRepeat: "no-repeat",
        maskSize: "contain",
        width: "30px",
    },
    links: {
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "14px",
        fontWeight: 600,
        gap: "16px",
    },
    link: {
        color: steam.bright,
        paddingBlock: "4px",
    },
    linkOn: {
        borderBottomColor: steam.blue,
        borderBottomStyle: "solid",
        borderBottomWidth: "3px",
        color: steam.blue,
    },
    column: {
        backgroundColor: steam.column,
        display: "grid",
        alignContent: "start",
        marginInline: { default: "24px", [site.narrow]: 0 },
        overflow: "hidden",
    },
    band: {
        alignItems: "start",
        backgroundImage: steam.band,
        display: "flex",
        gap: "18px",
        padding: "18px",
    },
    avatar: {
        alignItems: "center",
        backgroundColor: site.avatar,
        boxShadow: `0 0 0 2px ${steam.online}, 0 0 12px ${steam.online}`,
        color: steam.bright,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "40px",
        height: "96px",
        justifyContent: "center",
        width: "96px",
    },
    who: {
        display: "grid",
        gap: "4px",
        minWidth: 0,
    },
    name: {
        color: steam.bright,
        fontSize: "22px",
        fontWeight: 400,
    },
    summary: {
        color: steam.ink,
        fontSize: "12px",
        paddingBlockStart: "6px",
    },
    levelBox: {
        display: { default: "grid", [site.narrow]: "none" },
        gap: "10px",
        marginInlineStart: "auto",
        minWidth: "170px",
    },
    levelLine: {
        alignItems: "center",
        color: steam.bright,
        display: "flex",
        fontSize: "22px",
        gap: "8px",
    },
    ring: {
        alignItems: "center",
        borderColor: steam.level,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        display: "inline-flex",
        fontSize: "15px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    badge: {
        alignItems: "center",
        backgroundColor: steam.panel,
        display: "flex",
        gap: "10px",
        padding: "8px",
    },
    badgeArt: {
        alignItems: "center",
        backgroundColor: steam.level,
        borderRadius: "50%",
        color: steam.header,
        display: "inline-flex",
        fontSize: "12px",
        fontWeight: 700,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    badgeText: {
        color: steam.bright,
        display: "grid",
        fontSize: "11px",
    },
    grid: {
        display: "grid",
        gap: "14px",
        gridTemplateColumns: { default: "minmax(0, 1fr) 150px", [site.narrow]: "minmax(0, 1fr)" },
        padding: "14px",
    },
    section: {
        alignContent: "start",
        display: "grid",
        gap: "6px",
    },
    bar: {
        alignItems: "center",
        backgroundImage: steam.bar,
        color: steam.bright,
        display: "flex",
        fontSize: "15px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    barNote: {
        color: steam.muted,
        fontSize: "12px",
        marginInlineStart: "auto",
    },
    game: {
        backgroundColor: steam.panel,
        display: "flex",
        gap: "12px",
        padding: "8px",
    },
    capsule: {
        alignItems: "end",
        color: steam.bright,
        display: "flex",
        flexShrink: 0,
        fontSize: "11px",
        fontWeight: 800,
        height: "50px",
        letterSpacing: "0.04em",
        padding: "5px",
        width: { default: "134px", [site.narrow]: "84px" },
    },
    gameText: {
        display: "grid",
        flexGrow: 1,
        gap: "6px",
        minWidth: 0,
    },
    gameHead: {
        alignItems: "start",
        display: "flex",
        gap: "8px",
    },
    gameName: {
        color: steam.bright,
        fontSize: "14px",
    },
    right: {
        marginInlineStart: "auto",
        textAlign: "end",
    },
    progress: {
        display: { default: "grid", [site.narrow]: "none" },
        gap: "4px",
    },
    track: {
        backgroundColor: steam.track,
        display: "block",
        height: "5px",
    },
    fill: {
        backgroundColor: steam.blue,
        display: "block",
        height: "100%",
    },
    side: {
        alignContent: "start",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "10px",
    },
    status: {
        color: steam.online,
        fontSize: "17px",
    },
    count: {
        color: steam.bright,
        fontSize: "13px",
    },
    countNumber: {
        color: steam.muted,
        fontSize: "18px",
        marginInlineStart: "4px",
    },
    small: {
        color: steam.muted,
        fontSize: "11px",
    },
});

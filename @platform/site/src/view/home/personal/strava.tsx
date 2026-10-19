import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { strava } from "./strava.stylex";

/** The strokes of each of the site's own glyphs, on a 24-unit grid. */
const glyphs = {
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    plus: ["M12 5v14", "M5 12h14"],
    caret: ["m7 10 5 5 5-5"],
    run: ["M13 4.5a1.5 1.5 0 1 0 3 0 1.5 1.5 0 0 0-3 0", "m7 21 3-6 3 2v5", "M6 12l3-4h4l3 4 3 1"],
    thumb: [
        "M7 11v9H4v-9z",
        "M7 11l4-7a2 2 0 0 1 3 2l-1 4h6a2 2 0 0 1 2 2.3l-1.2 6A2 2 0 0 1 17.8 20H7",
    ],
    comment: ["M4 5h16v11H9l-5 4z"],
} satisfies Record<string, readonly string[]>;

/** The site's top links. */
const links: readonly string[] = ["Dashboard", "Training", "Maps", "Challenges"];

/** Your latest runs, newest first: title, when, distance, pace, time, kudos and route. */
const runs: readonly (readonly [
    title: string,
    when: string,
    distance: string,
    pace: string,
    time: string,
    kudos: number,
    route: string,
])[] = [
    [
        "Morning run",
        "October 21, 2025 at 7:02 AM · Zurich",
        "8.20 km",
        "5:08 /km",
        "42m 06s",
        6,
        "M20 70 L44 52 L70 58 L96 34 L128 40 L150 22 L176 30 L196 52 L170 74 L130 70 L96 80 L60 84 Z",
    ],
    [
        "Long run",
        "October 19, 2025 at 8:30 AM · Zurich",
        "21.10 km",
        "5:31 /km",
        "1h 56m",
        14,
        "M14 80 L40 66 L58 40 L92 30 L120 18 L158 24 L190 44 L198 70 L170 86 L120 82 L80 88 L40 90 Z",
    ],
];

/** The kilometres you ran each of the last seven days, oldest first, Wednesday the 15th on. */
const week: readonly number[] = [0, 0, 6.4, 0, 21.1, 0, 8.2];

/** Draw your Strava dashboard: your profile, your week and your latest runs. */
export function Strava() {
    return (
        <div data-component="StravaPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <b {...style.attrs(styles.logo)}>STRAVA</b>
                <span {...style.attrs(styles.links)}>
                    {links.map((link) => (
                        <span {...style.attrs(styles.link)}>
                            {link}
                            <Mark name="caret" size={13} />
                        </span>
                    ))}
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <Mark name="search" size={18} />
                    <Mark name="bell" size={18} />
                    <span {...style.attrs(styles.me)}>F</span>
                    <span {...style.attrs(styles.upload)}>
                        <Mark name="plus" size={16} />
                    </span>
                </span>
            </header>

            {/* dashboard */}
            <div {...style.attrs(styles.body)}>
                {/* profile and week */}
                <aside {...style.attrs(styles.side)}>
                    <div {...style.attrs(styles.card, styles.profile)}>
                        <span {...style.attrs(styles.avatar)}>F</span>
                        <b {...style.attrs(styles.name)}>Florian</b>
                        <span {...style.attrs(styles.counts)}>
                            {[
                                ["Following", "48"],
                                ["Followers", "52"],
                                ["Activities", "412"],
                            ].map(([label, count]) => (
                                <span {...style.attrs(styles.count)}>
                                    <span {...style.attrs(styles.muted)}>{label}</span>
                                    <b>{count}</b>
                                </span>
                            ))}
                        </span>
                    </div>
                    <div {...style.attrs(styles.card, styles.weekCard)}>
                        <span {...style.attrs(styles.muted)}>Last 7 days</span>
                        <b {...style.attrs(styles.total)}>35.7 km</b>
                        <span {...style.attrs(styles.bars)}>
                            {week.map((kilometres) => (
                                <span
                                    style={{ height: `${Math.max(2, kilometres * 2)}px` }}
                                    {...style.attrs(styles.bar, kilometres > 0 && styles.barOn)}
                                />
                            ))}
                        </span>
                        <span {...style.attrs(styles.days)}>
                            {["W", "T", "F", "S", "S", "M", "T"].map((day) => (
                                <span>{day}</span>
                            ))}
                        </span>
                    </div>
                </aside>

                {/* feed */}
                <main {...style.attrs(styles.feed)}>
                    {runs.map(([title, when, distance, pace, time, kudos, route]) => (
                        <article {...style.attrs(styles.card, styles.activity)}>
                            <span {...style.attrs(styles.who)}>
                                <span {...style.attrs(styles.face)}>F</span>
                                <span {...style.attrs(styles.whoText)}>
                                    <b>Florian</b>
                                    <span {...style.attrs(styles.muted, styles.small)}>
                                        <Mark name="run" size={13} />
                                        {when}
                                    </span>
                                </span>
                            </span>
                            <b {...style.attrs(styles.title)}>{title}</b>
                            <span {...style.attrs(styles.stats)}>
                                {[
                                    ["Distance", distance],
                                    ["Pace", pace],
                                    ["Time", time],
                                ].map(([label, value]) => (
                                    <span {...style.attrs(styles.stat)}>
                                        <span {...style.attrs(styles.muted, styles.small)}>
                                            {label}
                                        </span>
                                        <b {...style.attrs(styles.value)}>{value}</b>
                                    </span>
                                ))}
                            </span>
                            <svg viewBox="0 0 212 100" {...style.attrs(styles.map)}>
                                <path
                                    d="M0 30 H212 M0 64 H212 M60 0 V100 M150 0 V100"
                                    {...style.attrs(styles.road)}
                                />
                                <path d="M100 0 h40 v26 h-40z" {...style.attrs(styles.park)} />
                                <path d={route} {...style.attrs(styles.route)} />
                            </svg>
                            <span {...style.attrs(styles.kudos)}>
                                <span {...style.attrs(styles.muted, styles.small)}>
                                    {kudos} kudos · 2 comments
                                </span>
                                <span {...style.attrs(styles.actions)}>
                                    <Mark name="thumb" size={17} />
                                    <Mark name="comment" size={17} />
                                </span>
                            </span>
                        </article>
                    ))}
                </main>
            </div>
        </div>
    );
}

/** Draw one of the site's glyphs at a size in pixels. */
function Mark(properties: { name: keyof typeof glyphs; size: number }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
            {...style.attrs(styles.mark)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** The Strava page styles. */
const styles = style.create({
    page: {
        backgroundColor: strava.surface,
        color: strava.ink,
        display: "grid",
        fontFamily: strava.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    head: {
        alignItems: "center",
        backgroundColor: strava.card,
        borderBottomColor: strava.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "24px",
        height: "54px",
        paddingInline: { default: "24px", [site.narrow]: "12px" },
    },
    logo: {
        color: strava.orange,
        fontSize: "20px",
        fontStyle: "italic",
        fontWeight: 900,
        letterSpacing: "-0.02em",
    },
    links: {
        display: { default: "flex", [site.narrow]: "none" },
        gap: "18px",
    },
    link: {
        alignItems: "center",
        display: "flex",
        gap: "2px",
    },
    headEnd: {
        alignItems: "center",
        color: strava.muted,
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: strava.onOrange,
        display: "inline-flex",
        fontSize: "12px",
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    upload: {
        alignItems: "center",
        backgroundColor: strava.orange,
        borderRadius: "50%",
        color: strava.onOrange,
        display: "inline-flex",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    body: {
        alignItems: "start",
        display: "grid",
        gap: "16px",
        gridTemplateColumns: { default: "200px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        overflow: "hidden",
        padding: "16px",
    },
    side: {
        display: { default: "grid", [site.narrow]: "none" },
        gap: "12px",
    },
    card: {
        backgroundColor: strava.card,
        borderColor: strava.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
    },
    profile: {
        alignItems: "center",
        display: "grid",
        gap: "8px",
        justifyItems: "center",
        padding: "16px",
    },
    avatar: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderColor: strava.card,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "3px",
        boxShadow: `0 0 0 1px ${strava.line}`,
        color: strava.onOrange,
        display: "inline-flex",
        fontSize: "24px",
        height: "60px",
        justifyContent: "center",
        width: "60px",
    },
    name: {
        fontSize: "16px",
    },
    counts: {
        display: "flex",
        gap: "12px",
        textAlign: "center",
    },
    count: {
        display: "grid",
        fontSize: "11px",
        gap: "2px",
    },
    weekCard: {
        display: "grid",
        gap: "6px",
        padding: "14px",
    },
    total: {
        fontSize: "20px",
    },
    bars: {
        alignItems: "end",
        display: "grid",
        gap: "6px",
        gridTemplateColumns: "repeat(7, 1fr)",
        height: "46px",
    },
    bar: {
        backgroundColor: strava.line,
        borderRadius: "2px",
    },
    barOn: {
        backgroundColor: strava.orange,
    },
    days: {
        color: strava.muted,
        display: "grid",
        fontSize: "10px",
        gridTemplateColumns: "repeat(7, 1fr)",
        textAlign: "center",
    },
    feed: {
        display: "grid",
        gap: "12px",
    },
    activity: {
        display: "grid",
        gap: "10px",
        padding: "14px",
    },
    who: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: strava.onOrange,
        display: "inline-flex",
        fontSize: "13px",
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    whoText: {
        display: "grid",
        gap: "2px",
    },
    small: {
        alignItems: "center",
        display: "flex",
        fontSize: "11px",
        gap: "4px",
    },
    muted: {
        color: strava.muted,
    },
    title: {
        fontSize: "18px",
        fontWeight: 700,
    },
    stats: {
        display: "flex",
        gap: "20px",
    },
    stat: {
        borderInlineStartColor: strava.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: { default: "1px", ":first-child": 0 },
        display: "grid",
        gap: "2px",
        paddingInlineStart: { default: "20px", ":first-child": 0 },
    },
    value: {
        fontSize: "17px",
        fontWeight: 500,
    },
    map: {
        backgroundColor: strava.map,
        borderRadius: "4px",
        height: "110px",
        width: "100%",
    },
    road: {
        fill: "none",
        stroke: strava.road,
        strokeWidth: 5,
    },
    park: {
        fill: strava.park,
    },
    route: {
        fill: "none",
        stroke: strava.orange,
        strokeLinejoin: "round",
        strokeWidth: 3,
    },
    kudos: {
        alignItems: "center",
        display: "flex",
    },
    actions: {
        color: strava.muted,
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },
});

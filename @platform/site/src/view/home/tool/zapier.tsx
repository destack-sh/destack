import * as style from "@destack/style";

import { welcome } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { zapier } from "./zapier.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    back: ["m15 18-6-6 6-6"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    play: ["M7 5v14l11-7z"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    check: ["m6 12.5 4 4 8-9"],
    caret: ["m7 10 5 5 5-5"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    search: ["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14", "m20 20-4-4"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    arrow: ["M5 12h14", "m13 6 6 6-6 6"],
    folder: ["M3 6h6l2 2h10v11H3z"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8", "M4 21a8 8 0 0 1 16 0"],
    close: ["m6 6 12 12", "M18 6 6 18"],
    hook: [
        "M8.5 17a3 3 0 1 1 2.5-4.6L14 7",
        "M14 7a3 3 0 1 1 2.8 2",
        "M16.8 9h0l2.2 4.5a3 3 0 1 1-2.5 4.5H11",
    ],
} satisfies Record<string, readonly string[]>;

/** The mark of each step's app: its icon and colour, or a glyph of the editor's own. */
const marks: readonly (readonly [icon: string, tint: string])[] = [
    ["", ""],
    ["slack", "#4a154b"],
    ["gmail", "#ea4335"],
];

/** Draw the welcome automation as a zap in its editor, its Slack step chosen. */
export function Zapier() {
    return (
        <div data-component="ZapierPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <Mark name="back" size={18} />
                <Favicon icon="zapier" tint="#ff4f00" size={20} />
                <b {...style.attrs(styles.zapName)}>Welcome new signups</b>
                <Mark name="pencil" size={14} />
                <span {...style.attrs(styles.status)}>
                    <span {...style.attrs(styles.statusDot)} />
                    On
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <span {...style.attrs(styles.testRun)}>
                        <Mark name="play" size={13} />
                        Test run
                    </span>
                    <span {...style.attrs(styles.publish)}>Publish</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* steps on the canvas */}
                <div {...style.attrs(styles.canvas)}>
                    {welcome.map(([app, step, detail], index) => (
                        <>
                            {index > 0 ? (
                                <span {...style.attrs(styles.joint)}>
                                    <span {...style.attrs(styles.line)} />
                                    <span {...style.attrs(styles.add)}>
                                        <Mark name="plus" size={12} />
                                    </span>
                                    <span {...style.attrs(styles.line)} />
                                </span>
                            ) : undefined}
                            <article {...style.attrs(styles.card, index === 1 && styles.cardOn)}>
                                <span {...style.attrs(styles.cardHead)}>
                                    <span {...style.attrs(styles.kind)}>
                                        {index === 0 ? <Mark name="bolt" size={12} /> : undefined}
                                        {index === 0 ? "Trigger" : "Action"}
                                    </span>
                                    <Mark name="more" size={16} />
                                </span>
                                <span {...style.attrs(styles.cardBody)}>
                                    <span {...style.attrs(styles.appMark)}>
                                        {index === 0 ? (
                                            <Mark name="hook" size={18} />
                                        ) : (
                                            <Favicon
                                                icon={marks[index]?.[0] ?? ""}
                                                tint={marks[index]?.[1] ?? ""}
                                                size={18}
                                            />
                                        )}
                                    </span>
                                    <span {...style.attrs(styles.cardText)}>
                                        <b>
                                            {index + 1}. {step}
                                        </b>
                                        <span {...style.attrs(styles.cardDetail)}>
                                            {index === 0 ? "Webhooks by Zapier" : app} · {detail}
                                        </span>
                                    </span>
                                    <span {...style.attrs(styles.done)}>
                                        <Mark name="check" size={12} />
                                    </span>
                                </span>
                            </article>
                        </>
                    ))}
                    <span {...style.attrs(styles.zoom)}>
                        <Mark name="minus" size={14} />
                        100%
                        <Mark name="plus" size={14} />
                    </span>
                </div>

                {/* the chosen step's setup */}
                <aside {...style.attrs(styles.panel)}>
                    <span {...style.attrs(styles.panelHead)}>
                        <Favicon icon="slack" tint="#4a154b" size={18} />
                        <b>2. Post in #launch</b>
                    </span>
                    <span {...style.attrs(styles.tabs)}>
                        <span {...style.attrs(styles.tabOn)}>Setup</span>
                        <span>Configure</span>
                        <span>Test</span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>App</span>
                        <span {...style.attrs(styles.input)}>
                            Slack
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>Action event</span>
                        <span {...style.attrs(styles.input)}>
                            Send Channel Message
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.field)}>
                        <span {...style.attrs(styles.fieldName)}>Account</span>
                        <span {...style.attrs(styles.input)}>
                            Slack Symbol #2
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.continue)}>Continue</span>
                </aside>
            </div>
        </div>
    );
}

/** One run in the Zap history: who it ran for and when. */
export type ZapRun = readonly [who: string, when: string];

/** The apps the launch askers Zap writes to, by mark and colour. */
const runApps: readonly (readonly [icon: string, tint: string])[] = [
    ["airtable", "#18bfff"],
    ["trello", "#0052cc"],
    ["slack", "#4a154b"],
    ["googlecalendar", "#4285f4"],
];

/** The history's filters. */
const filters: readonly (readonly [glyph: keyof typeof glyphs, name: string])[] = [
    ["calendar", "Date range"],
    ["bolt", "Zaps"],
    ["plus", "Apps"],
    ["folder", "Folders"],
    ["person", "Owners"],
];

/** Draw the Zap history: a Zap's runs, newest first, each with its status, apps and tasks. */
export function ZapierHistory(properties: { zap: string; runs: readonly ZapRun[] }) {
    return (
        <div data-component="ZapierHistory" {...style.attrs(styles.history)}>
            <b {...style.attrs(styles.historyTitle)}>Zap history</b>
            <span {...style.attrs(styles.historyTabs)}>
                <span {...style.attrs(styles.historyTab, styles.historyTabOn)}>Zap runs</span>
                <span {...style.attrs(styles.historyTab)}>Task usage</span>
            </span>
            <span {...style.attrs(styles.historySearch)}>
                <Mark name="search" size={14} />
                Search Zap runs
            </span>
            <span {...style.attrs(styles.historyFilters)}>
                {filters.map(([glyph, name]) => (
                    <span {...style.attrs(styles.historyFilter)}>
                        <Mark name={glyph} size={13} />
                        {name}
                    </span>
                ))}
            </span>
            <span {...style.attrs(styles.historyChips)}>
                <span {...style.attrs(styles.historyChip)}>
                    <Mark name="calendar" size={13} />
                    <b>Today</b>
                </span>
                <span {...style.attrs(styles.historyChip)}>
                    <Mark name="bolt" size={13} />
                    {properties.zap}
                    <Mark name="close" size={12} />
                </span>
                <span {...style.attrs(styles.historyClear)}>Clear all</span>
            </span>
            <span {...style.attrs(styles.historyStatuses)}>All statuses</span>
            {properties.runs.map(([who, when]) => (
                <span {...style.attrs(styles.historyRun)}>
                    <span {...style.attrs(styles.historyBox)} />
                    <span {...style.attrs(styles.historyOk)}>
                        <Mark name="check" size={12} />
                    </span>
                    <b>Success</b>
                    <span {...style.attrs(styles.historyApps)}>
                        {runApps.map(([icon, tint]) => (
                            <span {...style.attrs(styles.historyApp)}>
                                <Favicon icon={icon} tint={tint} size={13} />
                            </span>
                        ))}
                    </span>
                    <span {...style.attrs(styles.historyName)}>
                        <b>
                            {properties.zap} · {who}
                        </b>
                        <span>{when}</span>
                    </span>
                    <span {...style.attrs(styles.historyTasks)}>
                        <b>4</b>
                        tasks
                    </span>
                    <span {...style.attrs(styles.historyOwner)}>FC</span>
                    <Mark name="arrow" size={16} />
                </span>
            ))}
        </div>
    );
}

/** Draw one of the app's own glyphs in the current colour. */
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

/** The Zapier page styles. */
const styles = style.create({
    page: {
        backgroundColor: zapier.surface,
        color: zapier.ink,
        display: "grid",
        fontFamily: zapier.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },

    // header
    head: {
        alignItems: "center",
        borderBottomColor: zapier.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "10px",
        height: "52px",
        paddingInline: "14px",
    },
    zapName: {
        color: zapier.ink,
        fontSize: "15px",
        whiteSpace: "nowrap",
    },
    status: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.ink,
        display: { default: "inline-flex", [site.narrow]: "none" },
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    statusDot: {
        backgroundColor: zapier.on,
        borderRadius: "50%",
        height: "7px",
        width: "7px",
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        marginInlineStart: "auto",
    },
    testRun: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.ink,
        display: { default: "inline-flex", [site.narrow]: "none" },
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    publish: {
        backgroundColor: zapier.orange,
        borderRadius: "6px",
        color: zapier.onAccent,
        fontWeight: 600,
        paddingBlock: "7px",
        paddingInline: "16px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "minmax(0, 1fr) 260px", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // canvas
    canvas: {
        alignContent: "start",
        backgroundColor: zapier.canvas,
        backgroundImage: `radial-gradient(${zapier.dots} 1px, transparent 1px)`,
        backgroundSize: "16px 16px",
        display: "grid",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlock: "28px",
        paddingInline: "16px",
        position: "relative",
    },
    card: {
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: zapier.lift,
        display: "grid",
        gap: "10px",
        maxWidth: "340px",
        padding: "12px",
        width: "100%",
    },
    cardOn: {
        borderColor: zapier.purple,
        boxShadow: `0 0 0 2px ${zapier.purple}`,
    },
    cardHead: {
        alignItems: "center",
        color: zapier.muted,
        display: "flex",
        justifyContent: "space-between",
    },
    kind: {
        alignItems: "center",
        backgroundColor: zapier.badge,
        borderRadius: "4px",
        color: zapier.ink,
        display: "inline-flex",
        fontSize: "11px",
        fontWeight: 600,
        gap: "4px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    cardBody: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
    },
    appMark: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.orange,
        display: "inline-flex",
        flexShrink: 0,
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    cardText: {
        display: "grid",
        gap: "2px",
        minWidth: 0,
    },
    cardDetail: {
        color: zapier.muted,
        fontSize: "12px",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    done: {
        alignItems: "center",
        backgroundColor: zapier.on,
        borderRadius: "50%",
        color: zapier.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        height: "18px",
        justifyContent: "center",
        marginInlineStart: "auto",
        width: "18px",
    },
    joint: {
        alignItems: "center",
        color: zapier.muted,
        display: "flex",
        flexDirection: "column",
    },
    line: {
        backgroundColor: zapier.line,
        height: "10px",
        width: "1px",
    },
    add: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    zoom: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderRadius: "6px",
        bottom: "12px",
        boxShadow: zapier.lift,
        color: zapier.muted,
        display: "flex",
        gap: "10px",
        insetInlineStart: "12px",
        paddingBlock: "6px",
        paddingInline: "10px",
        position: "absolute",
    },

    // panel
    panel: {
        alignContent: "start",
        borderInlineStartColor: zapier.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "12px",
        padding: "16px",
    },
    panelHead: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
        fontSize: "14px",
    },
    tabs: {
        borderBottomColor: zapier.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "16px",
    },
    tabOn: {
        boxShadow: `inset 0 -2px 0 ${zapier.purple}`,
        color: zapier.ink,
        fontWeight: 600,
        paddingBlockEnd: "8px",
    },
    field: {
        display: "grid",
        gap: "6px",
    },
    fieldName: {
        fontSize: "12px",
        fontWeight: 600,
    },
    input: {
        alignItems: "center",
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "7px",
        paddingInline: "10px",
    },
    continue: {
        backgroundColor: zapier.purple,
        borderRadius: "6px",
        color: zapier.onAccent,
        fontWeight: 600,
        paddingBlock: "8px",
        textAlign: "center",
    },

    // the Zap history
    history: {
        alignContent: "start",
        backgroundColor: zapier.canvas,
        color: zapier.ink,
        display: "grid",
        fontFamily: zapier.face,
        fontSize: "12.5px",
        gap: "10px",
        height: "100%",
        overflow: "hidden",
        paddingBlock: "16px",
        paddingInline: "20px",
    },
    historyTitle: {
        fontSize: "22px",
        fontWeight: 700,
    },
    historyTabs: {
        borderBottomColor: zapier.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "18px",
        justifySelf: "start",
    },
    historyTab: {
        fontSize: "15px",
        fontWeight: 600,
        paddingBlock: "4px",
    },
    historyTabOn: {
        boxShadow: `inset 0 -3px 0 ${zapier.purple}`,
    },
    historySearch: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "8px",
        justifySelf: "start",
        paddingBlock: "6px",
        paddingInline: "10px",
        width: "45%",
    },
    historyFilters: {
        display: "grid",
        gap: "8px",
        gridTemplateColumns: "repeat(5, minmax(0, 1fr))",
    },
    historyFilter: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.muted,
        display: "flex",
        gap: "6px",
        overflow: "hidden",
        paddingBlock: "6px",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    historyChips: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    historyChip: {
        alignItems: "center",
        borderColor: zapier.ink,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "10px",
        whiteSpace: "nowrap",
    },
    historyClear: {
        color: zapier.purple,
        fontWeight: 600,
    },
    historyStatuses: {
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: zapier.muted,
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    historyRun: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        columnGap: "10px",
        display: "grid",
        gridTemplateColumns: "14px 20px 58px auto minmax(0, 1fr) 36px 26px 16px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    historyBox: {
        borderColor: zapier.muted,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        height: "13px",
        width: "13px",
    },
    historyOk: {
        alignItems: "center",
        backgroundColor: zapier.on,
        borderRadius: "50%",
        color: zapier.onAccent,
        display: "flex",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    historyApps: {
        display: "flex",
    },
    historyApp: {
        alignItems: "center",
        backgroundColor: zapier.surface,
        borderColor: zapier.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        height: "22px",
        justifyContent: "center",
        marginInlineStart: "-3px",
        width: "22px",
    },
    historyName: {
        display: "grid",
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    historyTasks: {
        display: "grid",
        fontSize: "11px",
        justifyItems: "center",
    },
    historyOwner: {
        alignItems: "center",
        backgroundColor: zapier.ink,
        borderRadius: "50%",
        color: zapier.onAccent,
        display: "flex",
        fontSize: "9.5px",
        fontWeight: 700,
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
});

import * as style from "@destack/style";

import { press, type PressState, signups } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { airtable } from "./airtable.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    caret: ["m7 10 5 5 5-5"],
    history: ["M4 12a8 8 0 1 0 2.3-5.7L4 8.5", "M4 4v4.5h4.5", "M12 8v4l3 2"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    views: ["M4 6h16", "M4 12h16", "M4 18h16"],
    grid: ["M4 4h16v16H4z", "M4 10h16", "M4 15h16", "M10 4v16"],
    hide: ["M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z", "M4 4l16 16"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    group: ["M4 5h16v5H4z", "M4 14h16v5H4z"],
    sort: ["M7 4v16", "m3 8 4-4 4 4", "M17 20V4", "m13 16 4 4 4-4"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    text: ["M6 18 11 6h2l5 12", "M8 14h8"],
    link: [
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1",
        "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    ],
    select: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8 11 4 4 4-4"],
    date: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
    plus: ["M12 5v14", "M5 12h14"],
} satisfies Record<string, readonly string[]>;

/** The base's tabs, Data chosen. */
const baseTabs: readonly string[] = ["Data", "Automations", "Interfaces", "Forms"];

/** The fields over the grid: name and type glyph. */
const fields: readonly (readonly [name: string, glyph: keyof typeof glyphs])[] = [
    ["Name", "text"],
    ["Outlet", "link"],
    ["Status", "select"],
    ["Last contact", "date"],
    ["Owner", "person"],
];

/** The fields over the signups' grid: name and type glyph. */
const signupFields: readonly (readonly [name: string, glyph: keyof typeof glyphs])[] = [
    ["Email", "text"],
    ["Source", "select"],
    ["Joined", "date"],
    ["Status", "select"],
    ["Notes", "text"],
];

/** The name each status pill shows. */
const statusNames: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** The tables of the launch base. */
type Table = "Press" | "Signups";

/** Draw the press list as the launch base's grid. */
export function Airtable() {
    return <Base table="Press" />;
}

/** Draw the waitlist's signups as the launch base's grid. */
export function AirtableSignups() {
    return <Base table="Signups" />;
}

/** Draw the launch base with one of its tables open: its tabs, its view bar and its grid. */
function Base(properties: { table: Table }) {
    return (
        <div data-component="AirtablePage" {...style.attrs(styles.page)}>
            {/* base header */}
            <header {...style.attrs(styles.head)}>
                <span {...style.attrs(styles.baseIcon)}>L</span>
                <b {...style.attrs(styles.baseName)}>Launch</b>
                <Mark name="caret" size={14} />
                <span {...style.attrs(styles.baseTabs)}>
                    {baseTabs.map((tab) => (
                        <span {...style.attrs(styles.baseTab, tab === "Data" && styles.baseTabOn)}>
                            {tab}
                        </span>
                    ))}
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <Mark name="history" size={17} />
                    <Mark name="help" size={17} />
                    <span {...style.attrs(styles.share)}>Share</span>
                    <Mark name="bell" size={17} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            {/* table tabs */}
            <div {...style.attrs(styles.tables)}>
                <TableTab name="Press" isOn={properties.table === "Press"} />
                <TableTab name="Signups" isOn={properties.table === "Signups"} />
                <span {...style.attrs(styles.table)}>Content</span>
                <span {...style.attrs(styles.table, styles.add)}>
                    <Mark name="plus" size={13} />
                    Add or import
                </span>
            </div>

            {/* view bar */}
            <div {...style.attrs(styles.viewBar)}>
                <span {...style.attrs(styles.tool)}>
                    <Mark name="views" size={15} />
                    Views
                </span>
                <span {...style.attrs(styles.view)}>
                    <Mark name="grid" size={15} />
                    Grid view
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.tool, styles.wide)}>
                    <Mark name="hide" size={15} />
                    Hide fields
                </span>
                <span {...style.attrs(styles.tool)}>
                    <Mark name="filter" size={15} />
                    Filter
                </span>
                <span {...style.attrs(styles.tool, styles.wide)}>
                    <Mark name="group" size={15} />
                    Group
                </span>
                <span {...style.attrs(styles.tool)}>
                    <Mark name="sort" size={15} />
                    Sort
                </span>
                <span {...style.attrs(styles.viewEnd)}>
                    <Mark name="search" size={15} />
                </span>
            </div>

            {/* grid */}
            {properties.table === "Press" ? <PressGrid /> : <SignupsGrid />}
        </div>
    );
}

/** Draw a table's tab, open or not. */
function TableTab(properties: { name: Table; isOn: boolean }) {
    return (
        <span {...style.attrs(styles.table, properties.isOn && styles.tableOn)}>
            {properties.name}
            {properties.isOn ? <Mark name="caret" size={12} /> : undefined}
        </span>
    );
}

/** Draw the press list's grid. */
function PressGrid() {
    return (
        <div {...style.attrs(styles.grid)}>
            <span {...style.attrs(styles.row, styles.headRow)}>
                <span {...style.attrs(styles.number)} />
                {fields.map(([name, glyph], index) => (
                    <span
                        {...style.attrs(
                            styles.cell,
                            styles.headCell,
                            index === 0 && styles.primary,
                        )}
                    >
                        <Mark name={glyph} size={13} />
                        {name}
                    </span>
                ))}
            </span>
            {press.map(([name, outlet, state, last], index) => (
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.number)}>{index + 1}</span>
                    <span {...style.attrs(styles.cell, styles.primary)}>{name}</span>
                    <span {...style.attrs(styles.cell)}>
                        <span {...style.attrs(styles.chip)}>{outlet}</span>
                    </span>
                    <span {...style.attrs(styles.cell)}>
                        <span
                            {...style.attrs(
                                styles.pill,
                                state === "scheduled"
                                    ? styles.green
                                    : state === "replied"
                                      ? styles.blue
                                      : styles.gray,
                            )}
                        >
                            {statusNames[state]}
                        </span>
                    </span>
                    <span {...style.attrs(styles.cell)}>{last}, 2025</span>
                    <span {...style.attrs(styles.cell)}>
                        <span {...style.attrs(styles.owner)}>F</span>
                        Florian
                    </span>
                </span>
            ))}
            <span {...style.attrs(styles.row)}>
                <span {...style.attrs(styles.number)}>
                    <Mark name="plus" size={13} />
                </span>
            </span>
            <span {...style.attrs(styles.footer)}>{press.length} records</span>
        </div>
    );
}

/** Draw the signups' grid: the latest of the waitlist's people. */
function SignupsGrid() {
    return (
        <div {...style.attrs(styles.grid)}>
            <span {...style.attrs(styles.row, styles.headRow)}>
                <span {...style.attrs(styles.number)} />
                {signupFields.map(([name, glyph], index) => (
                    <span
                        {...style.attrs(
                            styles.cell,
                            styles.headCell,
                            index === 0 && styles.primary,
                        )}
                    >
                        <Mark name={glyph} size={13} />
                        {name}
                    </span>
                ))}
            </span>
            {signups.map(([email, source, joined, state], index) => (
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.number)}>{index + 1}</span>
                    <span {...style.attrs(styles.cell, styles.primary)}>{email}</span>
                    <span {...style.attrs(styles.cell)}>
                        <span {...style.attrs(styles.chip)}>{source}</span>
                    </span>
                    <span {...style.attrs(styles.cell)}>{joined}</span>
                    <span {...style.attrs(styles.cell)}>
                        <span
                            {...style.attrs(
                                styles.pill,
                                state === "invited" ? styles.green : styles.gray,
                            )}
                        >
                            {state === "invited" ? "Invited" : "Waiting"}
                        </span>
                    </span>
                    <span {...style.attrs(styles.cell)} />
                </span>
            ))}
            <span {...style.attrs(styles.row)}>
                <span {...style.attrs(styles.number)}>
                    <Mark name="plus" size={13} />
                </span>
            </span>
            <span {...style.attrs(styles.footer)}>1,840 records</span>
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

/** The Airtable page styles. */
const styles = style.create({
    page: {
        backgroundColor: airtable.surface,
        color: airtable.ink,
        display: "grid",
        fontFamily: airtable.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
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

    // base header
    head: {
        alignItems: "center",
        borderBottomColor: airtable.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: airtable.muted,
        display: "flex",
        gap: "8px",
        height: "52px",
        paddingInline: "14px",
    },
    baseIcon: {
        alignItems: "center",
        backgroundColor: airtable.base,
        borderRadius: "8px",
        color: airtable.onAccent,
        display: "inline-flex",
        fontSize: "14px",
        fontWeight: 700,
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    baseName: {
        color: airtable.ink,
        fontSize: "17px",
    },
    baseTabs: {
        display: { default: "flex", [site.compact]: "none" },
        gap: "18px",
        marginInlineStart: "24px",
    },
    baseTab: {
        paddingBlock: "16px",
    },
    baseTabOn: {
        boxShadow: `inset 0 -2px 0 ${airtable.base}`,
        color: airtable.ink,
        fontWeight: 600,
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    share: {
        backgroundColor: airtable.base,
        borderRadius: "6px",
        color: airtable.onAccent,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: airtable.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },

    // table tabs
    tables: {
        alignItems: "end",
        backgroundColor: airtable.tabs,
        display: "flex",
        gap: "2px",
        height: "36px",
        paddingInline: "10px",
    },
    table: {
        alignItems: "center",
        borderStartEndRadius: "6px",
        borderStartStartRadius: "6px",
        color: airtable.muted,
        display: "flex",
        gap: "6px",
        height: "30px",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    tableOn: {
        backgroundColor: airtable.surface,
        color: airtable.ink,
        fontWeight: 600,
    },
    add: {
        display: { default: "flex", [site.narrow]: "none" },
    },

    // view bar
    viewBar: {
        alignItems: "center",
        borderBottomColor: airtable.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: airtable.muted,
        display: "flex",
        gap: "14px",
        height: "42px",
        overflow: "hidden",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    tool: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
    },
    view: {
        alignItems: "center",
        color: airtable.ink,
        display: "flex",
        fontWeight: 600,
        gap: "6px",
    },
    wide: {
        display: { default: "flex", [site.compact]: "none" },
    },
    viewEnd: {
        display: "flex",
        marginInlineStart: "auto",
    },

    // grid
    grid: {
        alignContent: "start",
        display: "grid",
        overflow: "hidden",
    },
    row: {
        borderBottomColor: airtable.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gridTemplateColumns: {
            default: "42px 160px 160px 130px 120px 120px",
            [site.narrow]: "36px 130px 140px 120px 110px 110px",
        },
        height: "33px",
    },
    headRow: {
        backgroundColor: airtable.header,
        color: airtable.muted,
    },
    number: {
        alignItems: "center",
        color: airtable.muted,
        display: "flex",
        fontSize: "12px",
        justifyContent: "center",
    },
    cell: {
        alignItems: "center",
        borderInlineEndColor: airtable.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        gap: "6px",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    headCell: {
        fontWeight: 500,
    },
    primary: {
        fontWeight: 600,
    },
    chip: {
        backgroundColor: airtable.link,
        borderColor: airtable.linkEdge,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        overflow: "hidden",
        paddingInline: "6px",
        textOverflow: "ellipsis",
    },
    pill: {
        borderRadius: "999px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    green: {
        backgroundColor: airtable.green,
    },
    blue: {
        backgroundColor: airtable.blue,
    },
    gray: {
        backgroundColor: airtable.gray,
    },
    owner: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: airtable.onAccent,
        display: "inline-flex",
        fontSize: "9px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    footer: {
        backgroundColor: airtable.header,
        color: airtable.muted,
        fontSize: "12px",
        paddingBlock: "8px",
        paddingInline: "14px",
    },
});

import * as style from "@destack/style";

import { press, type PressState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { googleSheets } from "./googlesheets.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    move: ["M4 6a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"],
    cloud: ["M7 18a4 4 0 0 1-.5-8A6 6 0 0 1 18 9a4.5 4.5 0 0 1-.5 9z", "m9.5 13.5 2 2 3.5-4"],
    history: ["M4 12a8 8 0 1 0 2.3-5.7L4 8.5", "M4 4v4.5h4.5", "M12 8v4l3 2"],
    comment: ["M5 5h14v11H9l-4 4z"],
    globe: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M3 12h18",
        "M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18",
    ],
    caret: ["m7 10 5 5 5-5"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    undo: ["M9 14 4 9l5-5", "M4 9h10a6 6 0 0 1 0 12h-3"],
    redo: ["m15 14 5-5-5-5", "M20 9H10a6 6 0 0 0 0 12h3"],
    print: ["M7 9V4h10v5", "M6 18H4v-7h16v7h-2", "M7 14h10v6H7z"],
    paint: ["M5 4h12v5H5z", "M17 6.5h2v5h-7v3", "M11 14.5h2V20h-2z"],
    fill: ["m5 12 7-7 7 7-7 7z", "M19 16s2 2.2 2 3.5a2 2 0 0 1-4 0c0-1.3 2-3.5 2-3.5z"],
    borders: ["M4 4h16v16H4z", "M12 4v16", "M4 12h16"],
    filter: ["M4 5h16l-6 7v6l-4 2v-8z"],
    sigma: ["M17 5H7l5 7-5 7h10"],
    plus: ["M12 5v14", "M5 12h14"],
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    explore: ["m12 3 2 6 6 2-6 2-2 6-2-6-6-2 6-2z"],
} satisfies Record<string, readonly string[]>;

/** The menus across the top of the spreadsheet. */
const menus: readonly string[] = [
    "File",
    "Edit",
    "View",
    "Insert",
    "Format",
    "Data",
    "Tools",
    "Help",
];

/** The grid's columns: letter and header. */
const columns: readonly (readonly [letter: string, header: string])[] = [
    ["A", "Name"],
    ["B", "Outlet"],
    ["C", "Status"],
    ["D", "Last contact"],
    ["E", "Notes"],
];

/** The name each status chip shows. */
const statusNames: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** Draw the press list as a shared spreadsheet, a status cell chosen. */
export function GoogleSheets() {
    return (
        <div data-component="GoogleSheetsPage" {...style.attrs(styles.page)}>
            {/* title bar with the menus */}
            <header {...style.attrs(styles.head)}>
                <Favicon icon="googlesheets" tint="#34a853" size={34} />
                <span {...style.attrs(styles.titles)}>
                    <span {...style.attrs(styles.name)}>
                        Launch – Press
                        <span {...style.attrs(styles.nameTools)}>
                            <Mark name="star" size={16} />
                            <Mark name="move" size={16} />
                            <Mark name="cloud" size={16} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.menus)}>
                        {menus.map((menu) => (
                            <span>{menu}</span>
                        ))}
                    </span>
                </span>
                <span {...style.attrs(styles.end)}>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="history" size={20} />
                    </span>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="comment" size={20} />
                    </span>
                    <span {...style.attrs(styles.share)}>
                        <Mark name="globe" size={16} />
                        Share
                    </span>
                    <span {...style.attrs(styles.face)}>F</span>
                </span>
            </header>

            {/* toolbar */}
            <div {...style.attrs(styles.toolbar)}>
                <Mark name="search" size={17} />
                <Mark name="undo" size={17} />
                <Mark name="redo" size={17} />
                <Mark name="print" size={17} />
                <Mark name="paint" size={17} />
                <span {...style.attrs(styles.select)}>
                    100%
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.rule)} />
                <span {...style.attrs(styles.symbols)}>$ % .0 .00 123</span>
                <span {...style.attrs(styles.rule)} />
                <span {...style.attrs(styles.select, styles.wideFlex)}>
                    Default…
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.size)}>10</span>
                <b {...style.attrs(styles.letter)}>B</b>
                <i {...style.attrs(styles.letter)}>I</i>
                <s {...style.attrs(styles.letter)}>S</s>
                <span {...style.attrs(styles.letter, styles.underlineA)}>A</span>
                <Mark name="fill" size={17} />
                <Mark name="borders" size={17} />
                <span {...style.attrs(styles.rule)} />
                <Mark name="filter" size={17} />
                <Mark name="sigma" size={17} />
            </div>

            {/* formula bar */}
            <div {...style.attrs(styles.formula)}>
                <span {...style.attrs(styles.nameBox)}>
                    C3
                    <Mark name="caret" size={11} />
                </span>
                <span {...style.attrs(styles.fx)}>fx</span>
                <span>Replied</span>
            </div>

            {/* grid */}
            <div {...style.attrs(styles.grid)}>
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.corner)} />
                    {columns.map(([letter]) => (
                        <span {...style.attrs(styles.colHead, letter === "C" && styles.headOn)}>
                            {letter}
                        </span>
                    ))}
                </span>
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.rowHead)}>1</span>
                    {columns.map(([, header]) => (
                        <span {...style.attrs(styles.cell, styles.bold)}>{header}</span>
                    ))}
                </span>
                {press.map(([name, outlet, state, last], index) => (
                    <span {...style.attrs(styles.row)}>
                        <span {...style.attrs(styles.rowHead, index === 1 && styles.headOn)}>
                            {index + 2}
                        </span>
                        <span {...style.attrs(styles.cell)}>{name}</span>
                        <span {...style.attrs(styles.cell)}>{outlet}</span>
                        <span {...style.attrs(styles.cell, index === 1 && styles.cellOn)}>
                            <span
                                {...style.attrs(
                                    styles.chip,
                                    state === "scheduled"
                                        ? styles.green
                                        : state === "replied"
                                          ? styles.sky
                                          : styles.gray,
                                )}
                            >
                                {statusNames[state]}
                                <Mark name="caret" size={10} />
                            </span>
                            {index === 1 ? <span {...style.attrs(styles.handle)} /> : undefined}
                        </span>
                        <span {...style.attrs(styles.cell, styles.number)}>{last}, 2025</span>
                        <span {...style.attrs(styles.cell, styles.quiet)}>
                            {state === "scheduled" ? "Call Wed 15:00" : ""}
                        </span>
                    </span>
                ))}
                {[7, 8, 9].map((number) => (
                    <span {...style.attrs(styles.row)}>
                        <span {...style.attrs(styles.rowHead)}>{number}</span>
                        {columns.map(() => (
                            <span {...style.attrs(styles.cell)} />
                        ))}
                    </span>
                ))}
            </div>

            {/* sheet tabs */}
            <footer {...style.attrs(styles.tabs)}>
                <Mark name="plus" size={16} />
                <Mark name="menu" size={16} />
                <span {...style.attrs(styles.tab, styles.tabOn)}>
                    Press
                    <Mark name="caret" size={11} />
                </span>
                <span {...style.attrs(styles.tab)}>Signups</span>
                <span {...style.attrs(styles.tab, styles.wideFlex)}>Content</span>
                <span {...style.attrs(styles.explore)}>
                    <Mark name="explore" size={16} />
                </span>
            </footer>
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

/** The Google Sheets page styles. */
const styles = style.create({
    page: {
        backgroundColor: googleSheets.chrome,
        color: googleSheets.ink,
        display: "grid",
        fontFamily: googleSheets.face,
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto auto minmax(0, 1fr) auto",
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

    // title bar
    head: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        paddingBlockEnd: "2px",
        paddingBlockStart: "8px",
        paddingInline: "12px",
    },
    titles: {
        display: "grid",
        gap: "2px",
        minWidth: 0,
    },
    name: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        gap: "10px",
        whiteSpace: "nowrap",
    },
    nameTools: {
        color: googleSheets.muted,
        display: "flex",
        gap: "10px",
    },
    menus: {
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "14px",
        gap: "14px",
    },
    end: {
        alignItems: "center",
        color: googleSheets.muted,
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    wide: {
        display: { default: "inline", [site.narrow]: "none" },
    },
    wideFlex: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    share: {
        alignItems: "center",
        backgroundColor: googleSheets.share,
        borderRadius: "999px",
        color: googleSheets.shareInk,
        display: "inline-flex",
        fontSize: "14px",
        fontWeight: 500,
        gap: "8px",
        height: "40px",
        paddingInline: "18px",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleSheets.onAccent,
        display: "inline-flex",
        fontSize: "14px",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },

    // toolbar
    toolbar: {
        alignItems: "center",
        backgroundColor: googleSheets.bar,
        borderRadius: "24px",
        color: googleSheets.muted,
        display: "flex",
        fontSize: "14px",
        gap: "12px",
        height: "40px",
        marginBlock: "6px",
        marginInline: "8px",
        overflow: "hidden",
        paddingInline: "14px",
        whiteSpace: "nowrap",
    },
    select: {
        alignItems: "center",
        display: "inline-flex",
        gap: "6px",
    },
    rule: {
        backgroundColor: googleSheets.line,
        flexShrink: 0,
        height: "20px",
        width: "1px",
    },
    symbols: {
        color: googleSheets.ink,
        letterSpacing: "0.04em",
    },
    size: {
        borderColor: googleSheets.muted,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleSheets.ink,
        paddingInline: "6px",
    },
    letter: {
        color: googleSheets.ink,
        fontFamily: googleSheets.grid,
        fontSize: "15px",
    },
    underlineA: {
        textDecorationLine: "underline",
        textDecorationThickness: "3px",
    },

    // formula bar
    formula: {
        alignItems: "center",
        backgroundColor: googleSheets.cell,
        borderBottomColor: googleSheets.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderTopColor: googleSheets.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        fontFamily: googleSheets.grid,
        fontSize: "13px",
        gap: "12px",
        height: "28px",
        paddingInline: "8px",
    },
    nameBox: {
        alignItems: "center",
        borderInlineEndColor: googleSheets.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        gap: "16px",
        paddingInlineEnd: "10px",
    },
    fx: {
        color: googleSheets.muted,
        fontStyle: "italic",
    },

    // grid
    grid: {
        alignContent: "start",
        backgroundColor: googleSheets.cell,
        display: "grid",
        fontFamily: googleSheets.grid,
        fontSize: "13px",
        overflow: "hidden",
    },
    row: {
        display: "grid",
        gridTemplateColumns: "46px 140px 150px 130px 110px 150px",
        height: "24px",
    },
    corner: {
        backgroundColor: googleSheets.header,
        borderBottomColor: googleSheets.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: googleSheets.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
    },
    colHead: {
        alignItems: "center",
        backgroundColor: googleSheets.header,
        borderBottomColor: googleSheets.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: googleSheets.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: googleSheets.muted,
        display: "flex",
        fontSize: "11px",
        justifyContent: "center",
    },
    rowHead: {
        alignItems: "center",
        backgroundColor: googleSheets.header,
        borderBottomColor: googleSheets.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: googleSheets.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: googleSheets.muted,
        display: "flex",
        fontSize: "11px",
        justifyContent: "center",
    },
    headOn: {
        backgroundColor: googleSheets.selected,
        color: googleSheets.ink,
        fontWeight: 600,
    },
    cell: {
        alignItems: "center",
        borderBottomColor: googleSheets.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: googleSheets.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "4px",
        position: "relative",
        whiteSpace: "nowrap",
    },
    cellOn: {
        boxShadow: `inset 0 0 0 2px ${googleSheets.blue}`,
    },
    handle: {
        backgroundColor: googleSheets.blue,
        borderColor: googleSheets.cell,
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "-3px",
        height: "6px",
        insetInlineEnd: "-3px",
        position: "absolute",
        width: "6px",
    },
    bold: {
        fontWeight: 700,
    },
    number: {
        justifyContent: "flex-end",
    },
    quiet: {
        color: googleSheets.muted,
    },
    chip: {
        alignItems: "center",
        borderRadius: "999px",
        display: "inline-flex",
        fontSize: "12px",
        gap: "4px",
        paddingInline: "8px",
    },
    green: {
        backgroundColor: googleSheets.green,
    },
    sky: {
        backgroundColor: googleSheets.sky,
    },
    gray: {
        backgroundColor: googleSheets.gray,
    },

    // sheet tabs
    tabs: {
        alignItems: "center",
        borderTopColor: googleSheets.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        color: googleSheets.muted,
        display: "flex",
        fontSize: "13px",
        gap: "12px",
        height: "38px",
        paddingInline: "12px",
    },
    tab: {
        alignItems: "center",
        borderRadius: "6px",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "12px",
    },
    tabOn: {
        backgroundColor: googleSheets.tabOn,
        color: googleSheets.blue,
        fontWeight: 600,
    },
    explore: {
        display: "flex",
        marginInlineStart: "auto",
    },
});

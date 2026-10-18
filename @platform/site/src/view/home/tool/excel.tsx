import * as style from "@destack/style";

import { press, type PressState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { excel } from "./excel.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    launcher: [
        "M5 5h.01",
        "M12 5h.01",
        "M19 5h.01",
        "M5 12h.01",
        "M12 12h.01",
        "M19 12h.01",
        "M5 19h.01",
        "M12 19h.01",
        "M19 19h.01",
    ],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    caret: ["m7 10 5 5 5-5"],
    undo: ["M9 14 4 9l5-5", "M4 9h10a6 6 0 0 1 0 12h-3"],
    paste: ["M8 4h8v3H8z", "M6 5H5v16h14V5h-1"],
    borders: ["M4 4h16v16H4z", "M12 4v16", "M4 12h16"],
    fill: ["m5 12 7-7 7 7-7 7z", "M19 16s2 2.2 2 3.5a2 2 0 0 1-4 0c0-1.3 2-3.5 2-3.5z"],
    align: ["M4 6h16", "M4 10h10", "M4 14h16", "M4 18h10"],
    merge: ["M4 6h16v12H4z", "m9 12 3-2v4z", "m15 12-3-2v4z"],
    sort: ["M7 4v16", "m3 8 4-4 4 4", "M17 20V4", "m13 16 4 4 4-4"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    comment: ["M5 5h14v11H9l-4 4z"],
    close: ["M18 6 6 18", "m6 6 12 12"],
    check: ["m6 12.5 4 4 8-9"],
    plus: ["M12 5v14", "M5 12h14"],
    filterArrow: ["m8 10 4 4 4-4"],
} satisfies Record<string, readonly string[]>;

/** The ribbon's tabs, Home chosen, with whether a compact window keeps each. */
const ribbonTabs: readonly (readonly [name: string, isKept: boolean])[] = [
    ["File", true],
    ["Home", true],
    ["Insert", true],
    ["Formulas", false],
    ["Data", true],
    ["Review", false],
    ["View", false],
];

/** The grid's columns: letter and the table's header. */
const columns: readonly (readonly [letter: string, header: string])[] = [
    ["A", "Name"],
    ["B", "Outlet"],
    ["C", "Status"],
    ["D", "Last contact"],
];

/** The name each status shows. */
const statusNames: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** Draw the press list as a workbook in the shared drive, a status cell chosen. */
export function Excel() {
    return (
        <div data-component="ExcelPage" {...style.attrs(styles.page)}>
            {/* title bar */}
            <header {...style.attrs(styles.title)}>
                <Mark name="launcher" size={18} />
                <Favicon icon="excel" tint="#107c41" size={20} />
                <span {...style.attrs(styles.file)}>
                    <b>Launch press list</b>
                    <span {...style.attrs(styles.saved)}>· Saved</span>
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={15} />
                    Search (Option + Q)
                </span>
                <span {...style.attrs(styles.face)}>F</span>
            </header>

            {/* ribbon */}
            <div {...style.attrs(styles.ribbon)}>
                <span {...style.attrs(styles.tabs)}>
                    {ribbonTabs.map(([tab, isKept]) => (
                        <span
                            {...style.attrs(
                                styles.tab,
                                !isKept && styles.tabSpare,
                                tab === "Home" && styles.tabOn,
                            )}
                        >
                            {tab}
                        </span>
                    ))}
                    <span {...style.attrs(styles.tabsEnd)}>
                        <span {...style.attrs(styles.chip)}>
                            <Mark name="pencil" size={14} />
                            Editing
                            <Mark name="caret" size={11} />
                        </span>
                        <span {...style.attrs(styles.shareButton)}>
                            <Mark name="share" size={14} />
                            Share
                        </span>
                    </span>
                </span>
                <span {...style.attrs(styles.commands)}>
                    <Mark name="undo" size={16} />
                    <Mark name="paste" size={16} />
                    <span {...style.attrs(styles.rule)} />
                    <span {...style.attrs(styles.select)}>
                        Aptos Narrow
                        <Mark name="caret" size={11} />
                    </span>
                    <span {...style.attrs(styles.select)}>
                        11
                        <Mark name="caret" size={11} />
                    </span>
                    <b>B</b>
                    <i>I</i>
                    <u>U</u>
                    <Mark name="borders" size={16} />
                    <Mark name="fill" size={16} />
                    <span {...style.attrs(styles.rule)} />
                    <Mark name="align" size={16} />
                    <Mark name="merge" size={16} />
                    <span {...style.attrs(styles.select)}>
                        General
                        <Mark name="caret" size={11} />
                    </span>
                    <Mark name="sort" size={16} />
                </span>
            </div>

            {/* formula bar */}
            <div {...style.attrs(styles.formula)}>
                <span {...style.attrs(styles.nameBox)}>
                    C3
                    <Mark name="caret" size={11} />
                </span>
                <span {...style.attrs(styles.formulaTools)}>
                    <Mark name="close" size={13} />
                    <Mark name="check" size={13} />
                    <i>fx</i>
                </span>
                <span>Replied</span>
            </div>

            {/* grid */}
            <div {...style.attrs(styles.grid)}>
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.head)} />
                    {columns.map(([letter]) => (
                        <span {...style.attrs(styles.head, letter === "C" && styles.headOn)}>
                            {letter}
                        </span>
                    ))}
                    <span {...style.attrs(styles.head)}>E</span>
                </span>
                <span {...style.attrs(styles.row)}>
                    <span {...style.attrs(styles.head)}>1</span>
                    {columns.map(([, header]) => (
                        <span {...style.attrs(styles.cell, styles.tableHead)}>
                            {header}
                            <span {...style.attrs(styles.filterButton)}>
                                <Mark name="filterArrow" size={10} />
                            </span>
                        </span>
                    ))}
                    <span {...style.attrs(styles.cell)} />
                </span>
                {press.map(([name, outlet, state, last], index) => (
                    <span {...style.attrs(styles.row)}>
                        <span {...style.attrs(styles.head, index === 1 && styles.headOn)}>
                            {index + 2}
                        </span>
                        <span {...style.attrs(styles.cell, index % 2 === 0 && styles.band)}>
                            {name}
                        </span>
                        <span {...style.attrs(styles.cell, index % 2 === 0 && styles.band)}>
                            {outlet}
                        </span>
                        <span
                            {...style.attrs(
                                styles.cell,
                                index % 2 === 0 && styles.band,
                                index === 1 && styles.cellOn,
                            )}
                        >
                            {statusNames[state]}
                            {index === 1 ? <span {...style.attrs(styles.handle)} /> : undefined}
                        </span>
                        <span
                            {...style.attrs(
                                styles.cell,
                                styles.number,
                                index % 2 === 0 && styles.band,
                            )}
                        >
                            {last}, 2025
                        </span>
                        <span {...style.attrs(styles.cell)} />
                    </span>
                ))}
                {[7, 8, 9].map((number) => (
                    <span {...style.attrs(styles.row)}>
                        <span {...style.attrs(styles.head)}>{number}</span>
                        {[0, 1, 2, 3, 4].map(() => (
                            <span {...style.attrs(styles.cell)} />
                        ))}
                    </span>
                ))}
            </div>

            {/* sheet tabs and status */}
            <footer {...style.attrs(styles.foot)}>
                <Mark name="plus" size={15} />
                <span {...style.attrs(styles.sheet)}>Press</span>
                <span {...style.attrs(styles.sheetQuiet)}>Signups</span>
                <span {...style.attrs(styles.status)}>Ready</span>
                <span {...style.attrs(styles.zoom)}>100%</span>
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

/** The Excel page styles. */
const styles = style.create({
    page: {
        backgroundColor: excel.chrome,
        color: excel.ink,
        display: "grid",
        fontFamily: excel.face,
        fontSize: "13px",
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
        strokeWidth: 1.6,
    },

    // title bar
    title: {
        alignItems: "center",
        color: excel.muted,
        display: "flex",
        gap: "12px",
        height: "46px",
        paddingInline: "14px",
    },
    file: {
        alignItems: "center",
        color: excel.ink,
        display: "flex",
        gap: "6px",
        whiteSpace: "nowrap",
    },
    saved: {
        color: excel.muted,
        display: { default: "inline", [site.narrow]: "none" },
    },
    search: {
        alignItems: "center",
        backgroundColor: excel.surface,
        borderColor: excel.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "30px",
        marginInline: "auto",
        maxWidth: "360px",
        paddingInline: "10px",
        width: "40%",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: excel.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "28px",
        justifyContent: "center",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
        width: "28px",
    },

    // ribbon
    ribbon: {
        backgroundColor: excel.surface,
        borderRadius: "8px",
        boxShadow: excel.lift,
        display: "grid",
        marginBlockEnd: "6px",
        marginInline: "8px",
    },
    tabs: {
        alignItems: "center",
        display: "flex",
        paddingBlockStart: "4px",
        paddingInline: "6px",
    },
    tab: {
        color: excel.muted,
        display: { default: "inline", [site.narrow]: "none" },
        paddingBlock: "7px",
        paddingInline: "9px",
        whiteSpace: "nowrap",
    },
    tabSpare: {
        display: { default: "inline", [site.compact]: "none" },
    },
    tabOn: {
        boxShadow: `inset 0 -3px 0 -1px ${excel.green}`,
        color: excel.ink,
        display: "inline",
        fontWeight: 600,
    },
    tabsEnd: {
        alignItems: "center",
        display: "flex",
        flexShrink: 0,
        gap: "6px",
        marginInlineStart: "auto",
    },
    chip: {
        alignItems: "center",
        borderColor: excel.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        height: "28px",
        paddingInline: "10px",
    },
    shareButton: {
        alignItems: "center",
        backgroundColor: excel.share,
        borderRadius: "4px",
        color: excel.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        height: "28px",
        paddingInline: "10px",
    },
    commands: {
        alignItems: "center",
        borderTopColor: excel.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "flex",
        gap: "10px",
        height: "40px",
        overflow: "hidden",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    rule: {
        backgroundColor: excel.line,
        flexShrink: 0,
        height: "20px",
        width: "1px",
    },
    select: {
        alignItems: "center",
        borderColor: excel.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        height: "24px",
        paddingInline: "8px",
    },

    // formula bar
    formula: {
        alignItems: "center",
        backgroundColor: excel.surface,
        borderBottomColor: excel.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        fontFamily: excel.grid,
        gap: "12px",
        height: "28px",
        paddingInline: "8px",
    },
    nameBox: {
        alignItems: "center",
        borderInlineEndColor: excel.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        gap: "18px",
        paddingInlineEnd: "10px",
    },
    formulaTools: {
        alignItems: "center",
        color: excel.muted,
        display: "flex",
        gap: "10px",
    },

    // grid
    grid: {
        alignContent: "start",
        backgroundColor: excel.surface,
        display: "grid",
        fontFamily: excel.grid,
        fontSize: "13px",
        overflow: "hidden",
    },
    row: {
        display: "grid",
        gridTemplateColumns: "40px 140px 150px 120px 120px minmax(80px, 1fr)",
        height: "22px",
    },
    head: {
        alignItems: "center",
        backgroundColor: excel.header,
        borderBottomColor: excel.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: excel.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: excel.muted,
        display: "flex",
        fontSize: "12px",
        justifyContent: "center",
    },
    headOn: {
        backgroundColor: excel.selected,
        boxShadow: `inset 0 -2px 0 ${excel.green}`,
        color: excel.green,
        fontWeight: 600,
    },
    cell: {
        alignItems: "center",
        borderBottomColor: excel.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        borderInlineEndColor: excel.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "4px",
        position: "relative",
        whiteSpace: "nowrap",
    },
    tableHead: {
        backgroundColor: excel.tableHead,
        color: excel.onAccent,
        fontWeight: 700,
        justifyContent: "space-between",
    },
    filterButton: {
        alignItems: "center",
        backgroundColor: excel.surface,
        borderRadius: "2px",
        color: excel.muted,
        display: "inline-flex",
        height: "14px",
        justifyContent: "center",
        width: "14px",
    },
    band: {
        backgroundColor: excel.band,
    },
    cellOn: {
        boxShadow: `inset 0 0 0 2px ${excel.green}`,
    },
    handle: {
        backgroundColor: excel.green,
        borderColor: excel.surface,
        borderStyle: "solid",
        borderWidth: "1px",
        bottom: "-3px",
        height: "6px",
        insetInlineEnd: "-3px",
        position: "absolute",
        width: "6px",
    },
    number: {
        justifyContent: "flex-end",
    },

    // footer
    foot: {
        alignItems: "center",
        color: excel.muted,
        display: "flex",
        fontSize: "12px",
        gap: "12px",
        height: "32px",
        paddingInline: "12px",
    },
    sheet: {
        boxShadow: `inset 0 -2px 0 ${excel.green}`,
        color: excel.green,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    sheetQuiet: {
        paddingInline: "8px",
    },
    status: {
        marginInlineStart: "auto",
    },
    zoom: {
        color: excel.ink,
    },
});

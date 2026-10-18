import * as style from "@destack/style";

import { press, type PressState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { coda } from "./coda.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    page: ["M6 3h9l4 4v14H6z", "M14 3v5h5"],
    plus: ["M12 5v14", "M5 12h14"],
    caret: ["m7 10 5 5 5-5"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    sort: ["M7 4v16", "m3 8 4-4 4 4", "M17 20V4", "m13 16 4 4 4-4"],
    text: ["M6 18 11 6h2l5 12", "M8 14h8"],
    select: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8 11 4 4 4-4"],
    date: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    person: ["M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8z", "M4 21a8 8 0 0 1 16 0"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
} satisfies Record<string, readonly string[]>;

/** The doc's pages: emoji, name, and whether it is the page shown. */
const documentPages: readonly (readonly [emoji: string, name: string])[] = [
    ["🚀", "Launch plan"],
    ["📰", "Press"],
    ["✉️", "Signups"],
    ["📈", "Metrics"],
];

/** The name each status chip shows. */
const statusNames: Readonly<Record<PressState, string>> = {
    pitched: "Pitched",
    replied: "Replied",
    scheduled: "Call booked",
};

/** Draw the press list as a table on a page of the launch doc. */
export function Coda() {
    return (
        <div data-component="CodaPage" {...style.attrs(styles.page)}>
            {/* page list */}
            <nav {...style.attrs(styles.side)}>
                <span {...style.attrs(styles.doc)}>
                    <Favicon icon="coda" tint="#ee5a29" size={18} />
                    <b>Destack launch</b>
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={13} />
                    Search
                </span>
                {documentPages.map(([emoji, name]) => (
                    <span {...style.attrs(styles.pageLink, name === "Press" && styles.pageOn)}>
                        <span>{emoji}</span>
                        {name}
                    </span>
                ))}
                <span {...style.attrs(styles.pageLink, styles.muted)}>
                    <Mark name="plus" size={13} />
                    New page
                </span>
            </nav>

            {/* the page */}
            <main {...style.attrs(styles.main)}>
                <header {...style.attrs(styles.bar)}>
                    <span {...style.attrs(styles.crumbs)}>Destack launch / Press</span>
                    <span {...style.attrs(styles.barEnd)}>
                        <span {...style.attrs(styles.face)}>F</span>
                        <span {...style.attrs(styles.share)}>Share</span>
                        <Mark name="more" size={16} />
                    </span>
                </header>
                <article {...style.attrs(styles.article)}>
                    <span aria-hidden="true" {...style.attrs(styles.emoji)}>
                        📰
                    </span>
                    <b {...style.attrs(styles.title)}>Press</b>
                    <p {...style.attrs(styles.text)}>
                        Everyone we pitch for launch week, and where each conversation stands.
                    </p>

                    <span {...style.attrs(styles.tableHead)}>
                        <b>Press contacts</b>
                        <span {...style.attrs(styles.viewChip)}>
                            All
                            <Mark name="caret" size={11} />
                        </span>
                        <span {...style.attrs(styles.tableTools)}>
                            <Mark name="filter" size={14} />
                            <Mark name="sort" size={14} />
                            <Mark name="bolt" size={14} />
                            <Mark name="search" size={14} />
                        </span>
                    </span>
                    <div {...style.attrs(styles.table)}>
                        <span {...style.attrs(styles.row, styles.headRow)}>
                            <span {...style.attrs(styles.cell)}>
                                <Mark name="text" size={12} />
                                Name
                            </span>
                            <span {...style.attrs(styles.cell)}>
                                <Mark name="text" size={12} />
                                Outlet
                            </span>
                            <span {...style.attrs(styles.cell)}>
                                <Mark name="select" size={12} />
                                Status
                            </span>
                            <span {...style.attrs(styles.cell, styles.wide)}>
                                <Mark name="date" size={12} />
                                Last contact
                            </span>
                        </span>
                        {press.map(([name, outlet, state, last]) => (
                            <span {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.cell, styles.strong)}>{name}</span>
                                <span {...style.attrs(styles.cell)}>{outlet}</span>
                                <span {...style.attrs(styles.cell)}>
                                    <span
                                        {...style.attrs(
                                            styles.chip,
                                            state === "scheduled"
                                                ? styles.green
                                                : state === "replied"
                                                  ? styles.blue
                                                  : styles.yellow,
                                        )}
                                    >
                                        {statusNames[state]}
                                    </span>
                                </span>
                                <span {...style.attrs(styles.cell, styles.wide, styles.muted)}>
                                    {last}
                                </span>
                            </span>
                        ))}
                        <span {...style.attrs(styles.newRow)}>
                            <Mark name="plus" size={13} />
                            New row
                        </span>
                    </div>
                </article>
            </main>
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

/** The Coda page styles. */
const styles = style.create({
    page: {
        backgroundColor: coda.surface,
        color: coda.ink,
        display: "grid",
        fontFamily: coda.face,
        fontSize: "14px",
        gridTemplateColumns: { default: "210px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
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
    muted: {
        color: coda.muted,
    },

    // page list
    side: {
        backgroundColor: coda.side,
        borderInlineEndColor: coda.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "2px",
        paddingBlock: "14px",
        paddingInline: "10px",
    },
    doc: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
        paddingBlockEnd: "12px",
        paddingInline: "6px",
    },
    search: {
        alignItems: "center",
        color: coda.muted,
        display: "flex",
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    pageLink: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    pageOn: {
        backgroundColor: coda.selected,
        fontWeight: 600,
    },

    // the page
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    bar: {
        alignItems: "center",
        display: "flex",
        height: "48px",
        paddingInline: "20px",
    },
    crumbs: {
        color: coda.muted,
        fontSize: "13px",
    },
    barEnd: {
        alignItems: "center",
        color: coda.muted,
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: coda.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    share: {
        backgroundColor: coda.red,
        borderRadius: "6px",
        color: coda.onAccent,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    article: {
        alignContent: "start",
        display: "grid",
        gap: "12px",
        overflow: "hidden",
        paddingBlockEnd: "20px",
        paddingInline: { default: "48px", [site.narrow]: "16px" },
    },
    emoji: {
        fontSize: "36px",
        lineHeight: 1,
        marginBlockStart: "12px",
    },
    title: {
        fontSize: "32px",
        fontWeight: 700,
        letterSpacing: "-0.02em",
    },
    text: {
        color: coda.muted,
        lineHeight: 1.5,
        margin: 0,
    },
    tableHead: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        marginBlockStart: "8px",
    },
    viewChip: {
        alignItems: "center",
        borderColor: coda.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: coda.muted,
        display: "inline-flex",
        fontSize: "12px",
        gap: "4px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    tableTools: {
        color: coda.muted,
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    table: {
        borderColor: coda.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        overflow: "hidden",
    },
    row: {
        borderBottomColor: coda.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 1.2fr) minmax(0, 1.2fr) minmax(0, 1fr) minmax(0, 0.8fr)",
            [site.compact]: "minmax(0, 1.2fr) minmax(0, 1.2fr) minmax(0, 1fr)",
        },
        minHeight: "36px",
    },
    headRow: {
        color: coda.muted,
        fontSize: "12px",
        fontWeight: 500,
    },
    cell: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: "10px",
        whiteSpace: "nowrap",
    },
    wide: {
        display: { default: "flex", [site.compact]: "none" },
    },
    strong: {
        fontWeight: 600,
    },
    chip: {
        borderRadius: "999px",
        fontSize: "12px",
        paddingBlock: "2px",
        paddingInline: "10px",
    },
    green: {
        backgroundColor: coda.green,
    },
    blue: {
        backgroundColor: coda.blue,
    },
    yellow: {
        backgroundColor: coda.yellow,
    },
    newRow: {
        alignItems: "center",
        color: coda.muted,
        display: "flex",
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
});

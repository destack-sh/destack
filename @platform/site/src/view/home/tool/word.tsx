import * as style from "@destack/style";

import { launch } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { word } from "./word.stylex";

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
    bullets: ["M9 6h11", "M9 12h11", "M9 18h11", "M4.5 6h.01", "M4.5 12h.01", "M4.5 18h.01"],
    numbers: ["M10 6h10", "M10 12h10", "M10 18h10", "M4 5h1v4", "M4 15h2l-2 3h2"],
    align: ["M4 6h16", "M4 10h10", "M4 14h16", "M4 18h10"],
    find: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    mic: [
        "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z",
        "M19 11a7 7 0 0 1-14 0",
        "M12 18v3",
    ],
    comment: ["M5 5h14v11H9l-4 4z"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    gear: [
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M4 12h2",
        "M18 12h2",
        "M12 4v2",
        "M12 18v2",
        "m6.3 6.3 1.4 1.4",
        "m16.3 16.3 1.4 1.4",
        "m6.3 17.7 1.4-1.4",
        "m16.3 7.7 1.4-1.4",
    ],
} satisfies Record<string, readonly string[]>;

/** The ribbon's tabs, Home chosen, with whether a compact window keeps each. */
const ribbonTabs: readonly (readonly [name: string, isKept: boolean])[] = [
    ["File", true],
    ["Home", true],
    ["Insert", true],
    ["Draw", false],
    ["Layout", true],
    ["References", false],
    ["Review", false],
    ["View", false],
    ["Help", false],
];

/** Draw the Destack launch plan as a document in the shared drive, open in the browser. */
export function Word() {
    return (
        <div data-component="WordPage" {...style.attrs(styles.page)}>
            {/* title bar */}
            <header {...style.attrs(styles.title)}>
                <Mark name="launcher" size={18} />
                <Favicon icon="word" tint="#185abd" size={20} />
                <span {...style.attrs(styles.file)}>
                    <b>Destack launch</b>
                    <span {...style.attrs(styles.saved)}>
                        · Saved
                        <Mark name="caret" size={12} />
                    </span>
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={15} />
                    Search (Option + Q)
                </span>
                <span {...style.attrs(styles.titleEnd)}>
                    <Mark name="gear" size={18} />
                    <span {...style.attrs(styles.face)}>F</span>
                </span>
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
                        <span {...style.attrs(styles.chip, styles.roomy)}>
                            <Mark name="comment" size={15} />
                            Comments
                        </span>
                        <span {...style.attrs(styles.chip)}>
                            <Mark name="pencil" size={15} />
                            Editing
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.share)}>
                            <Mark name="share" size={15} />
                            Share
                            <Mark name="caret" size={12} />
                        </span>
                    </span>
                </span>
                <span {...style.attrs(styles.commands)}>
                    <Mark name="undo" size={17} />
                    <Mark name="caret" size={10} />
                    <Mark name="paste" size={17} />
                    <span {...style.attrs(styles.rule)} />
                    <span {...style.attrs(styles.select, styles.font)}>
                        Aptos (Body)
                        <Mark name="caret" size={12} />
                    </span>
                    <span {...style.attrs(styles.select)}>
                        12
                        <Mark name="caret" size={12} />
                    </span>
                    <b {...style.attrs(styles.letter)}>B</b>
                    <i {...style.attrs(styles.letter)}>I</i>
                    <u {...style.attrs(styles.letter)}>U</u>
                    <span {...style.attrs(styles.letter, styles.color)}>A</span>
                    <span {...style.attrs(styles.rule)} />
                    <Mark name="bullets" size={17} />
                    <Mark name="numbers" size={17} />
                    <Mark name="align" size={17} />
                    <span {...style.attrs(styles.rule)} />
                    <span {...style.attrs(styles.styleName)}>
                        Normal
                        <Mark name="caret" size={12} />
                    </span>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="find" size={17} />
                    </span>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="mic" size={17} />
                    </span>
                    <Mark name="more" size={17} />
                </span>
            </div>

            {/* the page on its desk */}
            <div {...style.attrs(styles.desk)}>
                <article {...style.attrs(styles.paper)}>
                    <b data-morph-title {...style.attrs(styles.heading1)}>
                        Destack launch
                    </b>
                    <p {...style.attrs(styles.meta)}>
                        Status: In progress · Launch: October 24, 2025 · Demo: October 22, 11:00 ·
                        Owner: Florian Cäsar
                    </p>
                    <p {...style.attrs(styles.text)}>
                        Ship to the waitlist on Thursday, then open signups on Friday.
                    </p>
                    <b {...style.attrs(styles.heading2)}>Before launch</b>
                    <ul {...style.attrs(styles.list)}>
                        {launch.map(([item, isDone]) => (
                            <li {...style.attrs(styles.item)}>
                                <span {...style.attrs(styles.box, isDone && styles.boxDone)}>
                                    {isDone ? (
                                        <svg viewBox="0 0 24 24" {...style.attrs(styles.tick)}>
                                            <path d="m6 12.5 4 4 8-9" />
                                        </svg>
                                    ) : undefined}
                                </span>
                                <span {...style.attrs(isDone && styles.done)}>{item}</span>
                            </li>
                        ))}
                    </ul>
                </article>
            </div>

            {/* status bar */}
            <footer {...style.attrs(styles.status)}>
                <span>Page 1 of 1</span>
                <span>52 words</span>
                <span {...style.attrs(styles.wide)}>English (U.S.)</span>
                <span {...style.attrs(styles.statusEnd)}>100%</span>
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

/** The Word page styles. */
const styles = style.create({
    page: {
        backgroundColor: word.chrome,
        color: word.ink,
        display: "grid",
        fontFamily: word.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto",
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
        color: word.muted,
        display: "flex",
        gap: "12px",
        height: "48px",
        paddingInline: "14px",
    },
    file: {
        alignItems: "center",
        color: word.ink,
        display: "flex",
        gap: "6px",
        whiteSpace: "nowrap",
    },
    saved: {
        alignItems: "center",
        color: word.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "4px",
    },
    search: {
        alignItems: "center",
        backgroundColor: word.field,
        borderColor: word.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "30px",
        marginInline: "auto",
        maxWidth: "380px",
        paddingInline: "10px",
        width: "40%",
    },
    titleEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: word.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },

    // ribbon
    ribbon: {
        backgroundColor: word.ribbon,
        borderRadius: "8px",
        boxShadow: word.ribbonLift,
        display: "grid",
        marginBlockEnd: "8px",
        marginInline: "8px",
    },
    tabs: {
        alignItems: "center",
        display: "flex",
        minWidth: 0,
        overflow: "hidden",
        paddingBlockStart: "4px",
        paddingInline: "6px",
    },
    tab: {
        color: word.muted,
        display: { default: "inline", [site.narrow]: "none" },
        paddingBlock: "8px",
        paddingInline: "10px",
        whiteSpace: "nowrap",
    },
    tabSpare: {
        display: { default: "inline", [site.compact]: "none" },
    },
    tabOn: {
        boxShadow: `inset 0 -3px 0 -1px ${word.blue}`,
        color: word.ink,
        display: "inline",
        fontWeight: 600,
    },
    tabsEnd: {
        alignItems: "center",
        backgroundColor: word.ribbon,
        display: "flex",
        flexShrink: 0,
        gap: "6px",
        marginInlineStart: "auto",
    },
    chip: {
        alignItems: "center",
        borderColor: word.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        height: "30px",
        paddingInline: "10px",
    },
    share: {
        alignItems: "center",
        backgroundColor: word.blue,
        borderRadius: "4px",
        color: word.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        height: "30px",
        paddingInline: "10px",
    },
    commands: {
        alignItems: "center",
        borderTopColor: word.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        color: word.ink,
        display: "flex",
        gap: "10px",
        height: "42px",
        overflow: "hidden",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    rule: {
        backgroundColor: word.line,
        flexShrink: 0,
        height: "22px",
        width: "1px",
    },
    select: {
        alignItems: "center",
        borderColor: word.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        height: "26px",
        paddingInline: "8px",
    },
    font: {
        minWidth: "120px",
        justifyContent: "space-between",
    },
    letter: {
        fontFamily: word.text,
        fontSize: "16px",
    },
    color: {
        textDecorationColor: "currentColor",
        textDecorationLine: "underline",
        textDecorationThickness: "3px",
    },
    styleName: {
        alignItems: "center",
        borderColor: word.blue,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "10px",
        height: "28px",
        paddingInline: "12px",
    },
    wide: {
        display: { default: "inline-flex", [site.narrow]: "none" },
    },
    roomy: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },

    // page
    desk: {
        display: "grid",
        justifyItems: "center",
        overflow: "hidden",
        paddingBlockStart: { default: "12px", [site.narrow]: "0" },
        paddingInline: { default: "24px", [site.narrow]: "0" },
    },
    paper: {
        alignContent: "start",
        backgroundColor: word.paper,
        boxShadow: word.paperLift,
        boxSizing: "border-box",
        display: "grid",
        fontFamily: word.text,
        gap: "12px",
        maxWidth: "640px",
        minHeight: "100%",
        paddingBlock: { default: "64px", [site.narrow]: "24px" },
        paddingInline: { default: "80px", [site.narrow]: "20px" },
        width: "100%",
    },
    heading1: {
        fontFamily: word.display,
        fontSize: "32px",
        fontWeight: 400,
        letterSpacing: "-0.01em",
    },
    meta: {
        color: word.muted,
        fontSize: "14px",
        margin: 0,
    },
    text: {
        fontSize: "16px",
        lineHeight: 1.45,
        margin: 0,
    },
    heading2: {
        color: word.heading,
        fontFamily: word.display,
        fontSize: "21px",
        fontWeight: 400,
        marginBlockStart: "10px",
    },
    list: {
        display: "grid",
        gap: "6px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    item: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        gap: "10px",
    },
    box: {
        alignItems: "center",
        borderColor: word.ink,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxSizing: "border-box",
        display: "inline-flex",
        flexShrink: 0,
        height: "15px",
        justifyContent: "center",
        width: "15px",
    },
    boxDone: {
        backgroundColor: word.blue,
        borderColor: word.blue,
    },
    tick: {
        fill: "none",
        height: "12px",
        stroke: word.onAccent,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 3.5,
        width: "12px",
    },
    done: {
        color: word.muted,
        textDecorationLine: "line-through",
    },

    // status bar
    status: {
        alignItems: "center",
        color: word.muted,
        display: "flex",
        fontSize: "12px",
        gap: "16px",
        height: "26px",
        paddingInline: "14px",
    },
    statusEnd: {
        marginInlineStart: "auto",
    },
});

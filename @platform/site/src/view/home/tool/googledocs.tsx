import * as style from "@destack/style";

import { launch } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { googleDocuments } from "./googledocs.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    move: [
        "M4 6a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z",
        "M10 13h6",
        "m14 11 2 2-2 2",
    ],
    cloud: ["M7 18a4 4 0 0 1-.5-8A6 6 0 0 1 18 9a4.5 4.5 0 0 1-.5 9z", "m9.5 13.5 2 2 3.5-4"],
    history: ["M4 12a8 8 0 1 0 2.3-5.7L4 8.5", "M4 4v4.5h4.5", "M12 8v4l3 2"],
    comment: ["M5 5h14v11H9l-4 4z"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    caret: ["m7 10 5 5 5-5"],
    globe: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M3 12h18",
        "M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18",
    ],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    undo: ["M9 14 4 9l5-5", "M4 9h10a6 6 0 0 1 0 12h-3"],
    redo: ["m15 14 5-5-5-5", "M20 9H10a6 6 0 0 0 0 12h3"],
    print: ["M7 9V4h10v5", "M6 18H4v-7h16v7h-2", "M7 14h10v6H7z"],
    spell: ["M4 15 8 5l4 10", "M5.5 11h5", "m13 16 3 3 5-6"],
    paint: ["M5 4h12v5H5z", "M17 6.5h2v5h-7v3", "M11 14.5h2V20h-2z"],
    minus: ["M6 12h12"],
    plus: ["M12 6v12", "M6 12h12"],
    link: [
        "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1",
        "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    ],
    image: ["M4 5h16v14H4z", "m4 16 5-5 4 4 3-3 4 4", "M15 9h.01"],
    align: ["M4 6h16", "M4 10h10", "M4 14h16", "M4 18h10"],
    checklist: ["m4 7 2 2 3-3", "M12 7h8", "m4 15 2 2 3-3", "M12 15h8"],
    bullets: ["M9 6h11", "M9 12h11", "M9 18h11", "M4.5 6h.01", "M4.5 12h.01", "M4.5 18h.01"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    up: ["m7 14 5-5 5 5"],
    outline: ["M4 6h10", "M4 12h16", "M4 18h12"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
} satisfies Record<string, readonly string[]>;

/** The menus across the top of the word processor. */
const menus: readonly string[] = [
    "File",
    "Edit",
    "View",
    "Insert",
    "Format",
    "Tools",
    "Extensions",
    "Help",
];

/** Draw the Destack launch plan as a shared document, its link open to anyone who has it. */
export function GoogleDocuments() {
    return (
        <div data-component="GoogleDocsPage" {...style.attrs(styles.page)}>
            {/* title bar with the menus */}
            <header {...style.attrs(styles.head)}>
                <Favicon icon="googledocs" tint="#4285f4" size={36} />
                <span {...style.attrs(styles.titles)}>
                    <span {...style.attrs(styles.name)}>
                        Destack launch
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
                    <span {...style.attrs(styles.meet)}>
                        <Mark name="video" size={20} />
                        <Mark name="caret" size={14} />
                    </span>
                    <span {...style.attrs(styles.share)}>
                        <Mark name="globe" size={16} />
                        Share
                        <span {...style.attrs(styles.shareCaret)}>
                            <Mark name="caret" size={14} />
                        </span>
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
                <Mark name="spell" size={17} />
                <Mark name="paint" size={17} />
                <span {...style.attrs(styles.select)}>
                    100%
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.rule)} />
                <span {...style.attrs(styles.select, styles.wideFlex)}>
                    Normal text
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.rule, styles.wideFlex)} />
                <span {...style.attrs(styles.select, styles.wideFlex)}>
                    Arial
                    <Mark name="caret" size={12} />
                </span>
                <span {...style.attrs(styles.rule)} />
                <span {...style.attrs(styles.size)}>
                    <Mark name="minus" size={14} />
                    <span {...style.attrs(styles.sizeValue)}>11</span>
                    <Mark name="plus" size={14} />
                </span>
                <span {...style.attrs(styles.rule)} />
                <b {...style.attrs(styles.letter)}>B</b>
                <i {...style.attrs(styles.letter)}>I</i>
                <u {...style.attrs(styles.letter)}>U</u>
                <span {...style.attrs(styles.letter, styles.colorA)}>A</span>
                <span {...style.attrs(styles.rule)} />
                <Mark name="link" size={17} />
                <Mark name="comment" size={17} />
                <Mark name="image" size={17} />
                <span {...style.attrs(styles.rule)} />
                <Mark name="align" size={17} />
                <Mark name="checklist" size={17} />
                <Mark name="bullets" size={17} />
                <span {...style.attrs(styles.editing)}>
                    <Mark name="pencil" size={16} />
                    <span {...style.attrs(styles.wide)}>Editing</span>
                    <Mark name="caret" size={12} />
                </span>
                <Mark name="up" size={16} />
            </div>

            {/* ruler */}
            <div {...style.attrs(styles.rulerRow)}>
                <span {...style.attrs(styles.ruler)} />
            </div>

            {/* the page on its desk */}
            <div {...style.attrs(styles.desk)}>
                <span {...style.attrs(styles.outline)}>
                    <Mark name="outline" size={18} />
                </span>
                <article {...style.attrs(styles.paper)}>
                    <b data-morph-title {...style.attrs(styles.title)}>
                        Destack launch
                    </b>
                    <p {...style.attrs(styles.meta)}>
                        <span {...style.attrs(styles.status)}>
                            In progress
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.chip)}>
                            <Mark name="calendar" size={13} />
                            Oct 24, 2025
                        </span>
                        <span {...style.attrs(styles.chip)}>
                            <Mark name="calendar" size={13} />
                            Demo · Oct 22, 11:00
                        </span>
                        <span {...style.attrs(styles.chip)}>
                            <span {...style.attrs(styles.person)}>F</span>
                            Florian Cäsar
                        </span>
                    </p>
                    <p {...style.attrs(styles.text)}>
                        Ship to the waitlist on Thursday, then open signups on Friday.
                    </p>
                    <b {...style.attrs(styles.heading)}>Before launch</b>
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

/** The Google Docs page styles. */
const styles = style.create({
    page: {
        backgroundColor: googleDocuments.chrome,
        color: googleDocuments.ink,
        display: "grid",
        fontFamily: googleDocuments.face,
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
        color: googleDocuments.muted,
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
        color: googleDocuments.muted,
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    wide: {
        display: { default: "inline", [site.narrow]: "none" },
    },
    wideFlex: {
        display: { default: "inline-flex", [site.narrow]: "none" },
    },
    meet: {
        alignItems: "center",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "2px",
    },
    share: {
        alignItems: "center",
        backgroundColor: googleDocuments.share,
        borderRadius: "999px",
        color: googleDocuments.shareInk,
        display: "inline-flex",
        fontSize: "14px",
        fontWeight: 500,
        gap: "8px",
        height: "40px",
        paddingInlineStart: "16px",
    },
    shareCaret: {
        alignItems: "center",
        borderInlineStartColor: googleDocuments.paper,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "8px",
    },
    face: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleDocuments.onAccent,
        display: "inline-flex",
        fontSize: "14px",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },

    // toolbar
    toolbar: {
        alignItems: "center",
        backgroundColor: googleDocuments.bar,
        borderRadius: "24px",
        color: googleDocuments.muted,
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
        backgroundColor: googleDocuments.line,
        flexShrink: 0,
        height: "20px",
        width: "1px",
    },
    size: {
        alignItems: "center",
        display: "inline-flex",
        gap: "4px",
    },
    sizeValue: {
        borderColor: googleDocuments.muted,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleDocuments.ink,
        paddingInline: "6px",
    },
    letter: {
        color: googleDocuments.ink,
        fontFamily: googleDocuments.text,
        fontSize: "15px",
    },
    colorA: {
        textDecorationColor: googleDocuments.ink,
        textDecorationLine: "underline",
        textDecorationThickness: "3px",
    },
    editing: {
        alignItems: "center",
        borderRadius: "999px",
        color: googleDocuments.ink,
        display: "inline-flex",
        gap: "6px",
        marginInlineStart: "auto",
        paddingBlock: "4px",
        paddingInline: "8px",
    },

    // ruler
    rulerRow: {
        display: { default: "flex", [site.narrow]: "none" },
        justifyContent: "center",
        paddingBlockEnd: "6px",
    },
    ruler: {
        backgroundImage: `repeating-linear-gradient(90deg, ${googleDocuments.line} 0 1px, transparent 1px 12px)`,
        backgroundPositionY: "bottom",
        backgroundRepeat: "repeat-x",
        backgroundSize: "100% 5px",
        borderBottomColor: googleDocuments.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        height: "16px",
        maxWidth: "640px",
        width: "70%",
    },

    // page
    desk: {
        display: "grid",
        justifyItems: "center",
        overflow: "hidden",
        paddingInline: { default: "24px", [site.narrow]: "0" },
        position: "relative",
    },
    outline: {
        color: googleDocuments.muted,
        display: { default: "flex", [site.narrow]: "none" },
        insetInlineStart: "18px",
        position: "absolute",
        top: "8px",
    },
    paper: {
        alignContent: "start",
        backgroundColor: googleDocuments.paper,
        boxShadow: googleDocuments.lift,
        boxSizing: "border-box",
        display: "grid",
        fontFamily: googleDocuments.text,
        gap: "12px",
        maxWidth: "640px",
        minHeight: "100%",
        paddingBlock: { default: "64px", [site.narrow]: "24px" },
        paddingInline: { default: "72px", [site.narrow]: "20px" },
        width: "100%",
    },
    title: {
        fontSize: "29px",
        fontWeight: 400,
    },
    meta: {
        display: "flex",
        flexWrap: "wrap",
        gap: "6px",
        margin: 0,
    },
    status: {
        alignItems: "center",
        backgroundColor: googleDocuments.status,
        borderRadius: "999px",
        color: googleDocuments.statusInk,
        display: "inline-flex",
        fontFamily: googleDocuments.face,
        fontSize: "13px",
        fontWeight: 500,
        gap: "4px",
        paddingBlock: "3px",
        paddingInlineEnd: "6px",
        paddingInlineStart: "10px",
    },
    chip: {
        alignItems: "center",
        backgroundColor: googleDocuments.chip,
        borderRadius: "999px",
        display: "inline-flex",
        fontFamily: googleDocuments.face,
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "3px",
        paddingInlineEnd: "10px",
        paddingInlineStart: "8px",
    },
    person: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleDocuments.onAccent,
        display: "inline-flex",
        fontSize: "10px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    text: {
        fontSize: "15px",
        lineHeight: 1.55,
        margin: 0,
    },
    heading: {
        fontSize: "20px",
        fontWeight: 400,
        marginBlockStart: "10px",
    },
    list: {
        display: "grid",
        gap: "8px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    item: {
        alignItems: "center",
        display: "flex",
        fontSize: "15px",
        gap: "10px",
    },
    box: {
        alignItems: "center",
        borderColor: googleDocuments.muted,
        borderRadius: "2px",
        borderStyle: "solid",
        borderWidth: "2px",
        boxSizing: "border-box",
        display: "inline-flex",
        flexShrink: 0,
        height: "15px",
        justifyContent: "center",
        width: "15px",
    },
    boxDone: {
        backgroundColor: googleDocuments.accent,
        borderColor: googleDocuments.accent,
    },
    tick: {
        fill: "none",
        height: "12px",
        stroke: googleDocuments.onAccent,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 3.5,
        width: "12px",
    },
    done: {
        color: googleDocuments.done,
        textDecorationLine: "line-through",
    },
});

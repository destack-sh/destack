import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { figjam } from "./figjam.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    caret: ["m7 10 5 5 5-5"],
    play: ["M7 5v14l11-7z"],
    cursor: ["M5 3l14 7-6 2-2 6z"],
    marker: ["M14 4l6 6-9 9H5v-6z", "M12 6l6 6"],
    shapes: ["M8 4l4 7H4z", "M15 13a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z", "M13 4h7v7h-7z"],
    stamp: ["M9 4h6l-1 6h4v4H6v-4h4z", "M5 18h14"],
    section: ["M4 6h16v14H4z", "M4 6h7V3H4z"],
    text: ["M5 6h14", "M12 6v13"],
    table: ["M4 5h16v14H4z", "M4 10h16", "M10 5v14"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
} satisfies Record<string, readonly string[]>;

/** The dock's tools, the cursor chosen and the stickies drawn apart. */
const tools: readonly (keyof typeof glyphs)[] = [
    "cursor",
    "marker",
    "shapes",
    "stamp",
    "section",
    "text",
    "table",
    "more",
];

/** The sticky colour of each section, by its place on the board. */
const tints: readonly ("yellow" | "green" | "sky")[] = ["yellow", "green", "sky"];

/** Draw the launch plan as a whiteboard of sections and stickies, your agent's cursor on it. */
export function FigJam() {
    return (
        <div data-component="FigJamPage" {...style.attrs(styles.page)}>
            {/* sections of stickies */}
            <div {...style.attrs(styles.sections)}>
                {board.map(([column, notes], index) => (
                    <section {...style.attrs(styles.section)}>
                        <span {...style.attrs(styles.sectionName)}>{column}</span>
                        <div {...style.attrs(styles.notes)}>
                            {notes.map((note) => {
                                const task = tasks.find(([id]) => id === note);

                                return (
                                    <span
                                        {...style.attrs(
                                            styles.sticky,
                                            task === undefined
                                                ? styles.pink
                                                : tints[index] === "green"
                                                  ? styles.green
                                                  : tints[index] === "sky"
                                                    ? styles.sky
                                                    : styles.yellow,
                                        )}
                                    >
                                        {task === undefined ? note : task[1]}
                                        <span {...style.attrs(styles.author)}>
                                            {task === undefined ? "Florian" : task[0]}
                                        </span>
                                    </span>
                                );
                            })}
                        </div>
                    </section>
                ))}
            </div>

            {/* your agent's cursor */}
            <span {...style.attrs(styles.cursor)}>
                <svg viewBox="0 0 24 24" {...style.attrs(styles.cursorMark)}>
                    <path d="M5 3l14 7-6 2-2 6z" />
                </svg>
                <span {...style.attrs(styles.cursorName)}>Agent</span>
            </span>

            {/* file menu */}
            <span {...style.attrs(styles.file)}>
                <span {...style.attrs(styles.logo)} />
                Destack launch plan
                <Mark name="caret" size={12} />
            </span>

            {/* collaborators and sharing */}
            <span {...style.attrs(styles.shareBar)}>
                <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                <Mark name="play" size={14} />
                <span {...style.attrs(styles.share)}>Share</span>
            </span>

            {/* dock */}
            <nav {...style.attrs(styles.dock)}>
                {tools.map((tool, index) => (
                    <>
                        <span {...style.attrs(styles.tool, tool === "cursor" && styles.toolOn)}>
                            <Mark name={tool} size={20} />
                        </span>
                        {index === 1 ? (
                            <span {...style.attrs(styles.stack)}>
                                <span {...style.attrs(styles.stackNote, styles.sky)} />
                                <span {...style.attrs(styles.stackNote, styles.green)} />
                                <span {...style.attrs(styles.stackNote, styles.yellow)} />
                            </span>
                        ) : undefined}
                    </>
                ))}
            </nav>
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

/** The FigJam page styles. */
const styles = style.create({
    page: {
        backgroundColor: figjam.canvas,
        color: figjam.ink,
        fontFamily: figjam.face,
        fontSize: "13px",
        height: "100%",
        overflow: "hidden",
        position: "relative",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },

    // sections
    sections: {
        display: "grid",
        gap: "24px",
        gridTemplateColumns: "repeat(3, 210px)",
        insetInlineStart: { default: "40px", [site.narrow]: "16px" },
        position: "absolute",
        top: "84px",
    },
    section: {
        backgroundColor: figjam.sectionFill,
        borderRadius: "12px",
        display: "grid",
        gap: "10px",
        paddingBlockEnd: "14px",
        paddingBlockStart: "36px",
        paddingInline: "14px",
        position: "relative",
    },
    sectionName: {
        backgroundColor: figjam.violet,
        borderRadius: "6px",
        color: figjam.onAccent,
        fontSize: "12px",
        fontWeight: 600,
        insetInlineStart: "10px",
        paddingBlock: "3px",
        paddingInline: "8px",
        position: "absolute",
        top: "10px",
    },
    notes: {
        display: "grid",
        gap: "10px",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    },
    sticky: {
        aspectRatio: "1",
        borderRadius: "4px",
        boxShadow: figjam.stickyLift,
        color: figjam.stickyInk,
        display: "flex",
        flexDirection: "column",
        fontSize: "12px",
        justifyContent: "space-between",
        lineHeight: 1.3,
        padding: "8px",
    },
    yellow: {
        backgroundColor: figjam.yellow,
    },
    green: {
        backgroundColor: figjam.green,
    },
    sky: {
        backgroundColor: figjam.sky,
    },
    pink: {
        backgroundColor: figjam.pink,
    },
    author: {
        alignSelf: "end",
        fontSize: "9px",
        opacity: 0.6,
    },
    cursor: {
        alignItems: "start",
        display: "flex",
        insetInlineStart: { default: "420px", [site.narrow]: "220px" },
        position: "absolute",
        top: "300px",
    },
    cursorMark: {
        fill: site.agent,
        height: "18px",
        stroke: figjam.onAccent,
        strokeWidth: 1.5,
        width: "18px",
    },
    cursorName: {
        backgroundColor: site.agent,
        borderRadius: "999px",
        color: figjam.onAccent,
        fontSize: "11px",
        marginBlockStart: "14px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },

    // floating bars
    file: {
        alignItems: "center",
        backgroundColor: figjam.pill,
        borderRadius: "999px",
        color: figjam.onAccent,
        display: "flex",
        fontWeight: 500,
        gap: "8px",
        insetInlineStart: "12px",
        paddingBlock: "6px",
        paddingInlineEnd: "14px",
        paddingInlineStart: "6px",
        position: "absolute",
        top: "12px",
    },
    logo: {
        backgroundImage: figjam.logo,
        borderRadius: "50%",
        height: "22px",
        width: "22px",
    },
    shareBar: {
        alignItems: "center",
        backgroundColor: figjam.panel,
        borderRadius: "999px",
        boxShadow: figjam.lift,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        insetInlineEnd: "12px",
        paddingBlock: "5px",
        paddingInline: "6px",
        position: "absolute",
        top: "12px",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: figjam.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    share: {
        backgroundColor: figjam.blue,
        borderRadius: "999px",
        color: figjam.onAccent,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    dock: {
        alignItems: "center",
        backgroundColor: figjam.panel,
        borderRadius: "14px",
        bottom: "14px",
        boxShadow: figjam.lift,
        display: "flex",
        gap: "4px",
        insetInlineStart: "50%",
        paddingBlock: "6px",
        paddingInline: "8px",
        position: "absolute",
        transform: "translateX(-50%)",
    },
    tool: {
        alignItems: "center",
        borderRadius: "8px",
        display: "inline-flex",
        height: "40px",
        justifyContent: "center",
        width: "40px",
    },
    toolOn: {
        backgroundColor: figjam.blue,
        color: figjam.onAccent,
    },
    stack: {
        display: "grid",
        height: "40px",
        marginInline: "6px",
        position: "relative",
        width: "34px",
    },
    stackNote: {
        borderRadius: "3px",
        boxShadow: figjam.stickyLift,
        gridArea: "1 / 1",
        height: "26px",
        marginBlockStart: "8px",
        width: "26px",
        ":nth-child(1)": { transform: "rotate(-10deg) translateX(-2px)" },
        ":nth-child(2)": { transform: "rotate(6deg) translateX(4px)" },
    },
});

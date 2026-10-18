import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { miro } from "./miro.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
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
    upload: ["M12 16V5", "m7 10 5-5 5 5", "M5 19h14"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    play: ["M7 5v14l11-7z"],
    cursor: ["M5 3l14 7-6 2-2 6z"],
    template: ["M4 4h16v6H4z", "M4 14h7v6H4z", "M15 14h5v6h-5z"],
    text: ["M5 6h14", "M12 6v13"],
    sticky: ["M5 5h14v9l-5 5H5z", "M14 19v-5h5"],
    shapes: ["M8 4l4 7H4z", "M15 13a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z", "M13 4h7v7h-7z"],
    pen: ["M4 20h4L19 9l-4-4L4 16z"],
    comment: ["M5 5h14v11H9l-4 4z"],
    frame: ["M7 3v18", "M17 3v18", "M3 7h18", "M3 17h18"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
    map: ["M4 5h16v14H4z", "M12 12h8v7h-8z"],
} satisfies Record<string, readonly string[]>;

/** The palette's tools, from the select cursor down. */
const palette: readonly (keyof typeof glyphs)[] = [
    "cursor",
    "template",
    "text",
    "sticky",
    "shapes",
    "pen",
    "comment",
    "frame",
    "plus",
];

/** The colour of each frame's notes, by the frame's place on the board. */
const frameStickies: readonly ("yellow" | "green" | "blue")[] = ["yellow", "green", "blue"];

/** Draw the launch plan as a whiteboard of frames and sticky notes, your agent's cursor on it. */
export function Miro() {
    return (
        <div data-component="MiroPage" {...style.attrs(styles.page)}>
            {/* frames of sticky notes */}
            <div {...style.attrs(styles.frames)}>
                {board.map(([column, notes], frame) => (
                    <section {...style.attrs(styles.frame)}>
                        <span {...style.attrs(styles.frameName)}>{column}</span>
                        <div {...style.attrs(styles.notes)}>
                            {notes.map((note) => {
                                const task = tasks.find(([id]) => id === note);

                                return (
                                    <span
                                        {...style.attrs(
                                            styles.sticky,
                                            frameStickies[frame] === "green"
                                                ? styles.green
                                                : frameStickies[frame] === "blue"
                                                  ? styles.blue
                                                  : styles.yellow,
                                            task === undefined && styles.pink,
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
            <svg aria-hidden="true" {...style.attrs(styles.connector)} viewBox="0 0 24 10">
                <path d="M1 5h21m-4-3.5L22 5l-4 3.5" />
            </svg>

            {/* your agent's cursor */}
            <span {...style.attrs(styles.cursor)}>
                <Mark name="cursor" size={18} />
                <span {...style.attrs(styles.cursorName)}>Agent</span>
            </span>

            {/* board header */}
            <header {...style.attrs(styles.boardCard)}>
                <span {...style.attrs(styles.logo)}>miro</span>
                <b {...style.attrs(styles.boardName)}>Destack launch plan</b>
                <span {...style.attrs(styles.cardTools)}>
                    <Mark name="gear" size={16} />
                    <Mark name="upload" size={16} />
                    <Mark name="search" size={16} />
                </span>
            </header>

            {/* collaborators and sharing */}
            <div {...style.attrs(styles.shareCard)}>
                <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                <span {...style.attrs(styles.present)}>
                    <Mark name="play" size={13} />
                    Present
                </span>
                <span {...style.attrs(styles.share)}>Share</span>
            </div>

            {/* tool palette */}
            <nav {...style.attrs(styles.palette)}>
                {palette.map((tool) => (
                    <span {...style.attrs(styles.tool, tool === "cursor" && styles.toolOn)}>
                        <Mark name={tool} size={18} />
                    </span>
                ))}
            </nav>

            {/* zoom */}
            <div {...style.attrs(styles.zoom)}>
                <Mark name="map" size={15} />
                <Mark name="minus" size={15} />
                <span>100%</span>
                <Mark name="plus" size={15} />
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

/** The Miro page styles. */
const styles = style.create({
    page: {
        backgroundColor: miro.canvas,
        backgroundImage: `radial-gradient(${miro.dots} 1px, transparent 1px)`,
        backgroundSize: "20px 20px",
        color: miro.ink,
        fontFamily: miro.face,
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

    // frames
    frames: {
        display: "grid",
        gap: "28px",
        gridTemplateColumns: { default: "repeat(3, 200px)", [site.narrow]: "repeat(3, 170px)" },
        insetInlineStart: { default: "96px", [site.narrow]: "20px" },
        position: "absolute",
        top: "92px",
    },
    frame: {
        backgroundColor: miro.panel,
        borderColor: miro.line,
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gap: "10px",
        padding: "14px",
        position: "relative",
    },
    frameName: {
        color: miro.muted,
        fontSize: "12px",
        insetInlineStart: 0,
        position: "absolute",
        top: "-20px",
    },
    notes: {
        display: "grid",
        gap: "10px",
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
    },
    sticky: {
        aspectRatio: "1",
        boxShadow: miro.stickyLift,
        color: miro.stickyInk,
        display: "flex",
        flexDirection: "column",
        fontSize: "11px",
        justifyContent: "space-between",
        lineHeight: 1.3,
        padding: "8px",
    },
    yellow: {
        backgroundColor: miro.stickyYellow,
    },
    green: {
        backgroundColor: miro.stickyGreen,
    },
    blue: {
        backgroundColor: miro.stickyBlue,
    },
    pink: {
        backgroundColor: miro.stickyPink,
    },
    author: {
        fontSize: "9px",
        opacity: 0.6,
    },
    connector: {
        fill: "none",
        height: "10px",
        insetInlineStart: { default: "300px", [site.narrow]: "194px" },
        position: "absolute",
        stroke: miro.ink,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.5,
        top: "170px",
        width: "20px",
    },
    cursor: {
        alignItems: "start",
        color: site.agent,
        display: "flex",
        gap: "2px",
        insetInlineStart: { default: "420px", [site.narrow]: "220px" },
        position: "absolute",
        top: "260px",
    },
    cursorName: {
        backgroundColor: site.agent,
        borderRadius: "4px",
        color: miro.onAccent,
        fontSize: "11px",
        marginBlockStart: "14px",
        paddingBlock: "1px",
        paddingInline: "6px",
    },

    // floating panels
    boardCard: {
        alignItems: "center",
        backgroundColor: miro.panel,
        borderRadius: "8px",
        boxShadow: miro.lift,
        display: "flex",
        gap: "12px",
        insetInlineStart: "12px",
        paddingBlock: "8px",
        paddingInline: "10px",
        position: "absolute",
        top: "12px",
    },
    logo: {
        backgroundColor: miro.yellow,
        borderRadius: "6px",
        color: miro.stickyInk,
        fontSize: "13px",
        fontWeight: 800,
        letterSpacing: "-0.02em",
        paddingBlock: "4px",
        paddingInline: "6px",
    },
    boardName: {
        fontSize: "14px",
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    cardTools: {
        color: miro.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "12px",
    },
    shareCard: {
        alignItems: "center",
        backgroundColor: miro.panel,
        borderRadius: "8px",
        boxShadow: miro.lift,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "6px",
        insetInlineEnd: "12px",
        paddingBlock: "6px",
        paddingInline: "8px",
        position: "absolute",
        top: "12px",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: miro.onAccent,
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
    present: {
        alignItems: "center",
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        paddingInline: "8px",
    },
    share: {
        backgroundColor: miro.blue,
        borderRadius: "6px",
        color: miro.onAccent,
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    palette: {
        backgroundColor: miro.panel,
        borderRadius: "8px",
        boxShadow: miro.lift,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "4px",
        insetInlineStart: "12px",
        padding: "4px",
        position: "absolute",
        top: "80px",
    },
    tool: {
        alignItems: "center",
        borderRadius: "6px",
        color: miro.ink,
        display: "inline-flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    toolOn: {
        backgroundColor: miro.canvas,
        color: miro.blue,
    },
    zoom: {
        alignItems: "center",
        backgroundColor: miro.panel,
        borderRadius: "8px",
        bottom: "12px",
        boxShadow: miro.lift,
        color: miro.ink,
        display: "flex",
        gap: "12px",
        insetInlineEnd: "12px",
        paddingBlock: "8px",
        paddingInline: "12px",
        position: "absolute",
    },
});

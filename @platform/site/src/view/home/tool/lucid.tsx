import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { lucid } from "./lucid.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    play: ["M7 5v14l11-7z"],
    cursor: ["M5 3l14 7-6 2-2 6z"],
    sticky: ["M5 5h14v9l-5 5H5z", "M14 19v-5h5"],
    shapes: ["M8 4l4 7H4z", "M15 13a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z", "M13 4h7v7h-7z"],
    pen: ["M4 20h4L19 9l-4-4L4 16z"],
    text: ["M5 6h14", "M12 6v13"],
    frame: ["M7 3v18", "M17 3v18", "M3 7h18", "M3 17h18"],
    template: ["M4 4h16v6H4z", "M4 14h7v6H4z", "M15 14h5v6h-5z"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
} satisfies Record<string, readonly string[]>;

/** The menus across the top of the document. */
const menus: readonly string[] = ["File", "Edit", "Select", "View", "Insert", "Arrange", "Share"];

/** The toolbar's tools, the select cursor chosen. */
const tools: readonly (keyof typeof glyphs)[] = [
    "cursor",
    "sticky",
    "shapes",
    "pen",
    "text",
    "frame",
    "template",
];

/** The sticky note colour of each frame, by its place on the board. */
const tints: readonly ("yellow" | "green" | "blue")[] = ["yellow", "green", "blue"];

/** Draw the launch plan as a brainstorming board of frames and sticky notes. */
export function Lucid() {
    return (
        <div data-component="LucidPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.head)}>
                <span {...style.attrs(styles.logo)}>L</span>
                <span {...style.attrs(styles.titles)}>
                    <span {...style.attrs(styles.title)}>
                        <b>Destack launch plan</b>
                        <Mark name="star" size={14} />
                    </span>
                    <span {...style.attrs(styles.menus)}>
                        {menus.map((menu) => (
                            <span>{menu}</span>
                        ))}
                    </span>
                </span>
                <span {...style.attrs(styles.headEnd)}>
                    <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                    <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                    <span {...style.attrs(styles.present)}>
                        <Mark name="play" size={13} />
                        Present
                    </span>
                    <span {...style.attrs(styles.share)}>Share</span>
                </span>
            </header>

            <div {...style.attrs(styles.canvas)}>
                {/* frames of sticky notes */}
                <div {...style.attrs(styles.frames)}>
                    {board.map(([column, notes], index) => (
                        <section {...style.attrs(styles.frame)}>
                            <span {...style.attrs(styles.frameName)}>{column}</span>
                            <div {...style.attrs(styles.notes)}>
                                {notes.map((note) => {
                                    const task = tasks.find(([id]) => id === note);

                                    return (
                                        <span
                                            {...style.attrs(
                                                styles.sticky,
                                                task === undefined
                                                    ? styles.purple
                                                    : tints[index] === "green"
                                                      ? styles.green
                                                      : tints[index] === "blue"
                                                        ? styles.blue
                                                        : styles.yellow,
                                            )}
                                        >
                                            {task === undefined ? note : task[1]}
                                        </span>
                                    );
                                })}
                            </div>
                        </section>
                    ))}
                </div>

                {/* toolbar */}
                <nav {...style.attrs(styles.toolbar)}>
                    {tools.map((tool) => (
                        <span {...style.attrs(styles.tool, tool === "cursor" && styles.toolOn)}>
                            <Mark name={tool} size={19} />
                        </span>
                    ))}
                </nav>

                {/* zoom */}
                <span {...style.attrs(styles.zoom)}>
                    <Mark name="minus" size={14} />
                    100%
                    <Mark name="plus" size={14} />
                </span>
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

/** The Lucid page styles. */
const styles = style.create({
    page: {
        backgroundColor: lucid.canvas,
        color: lucid.ink,
        display: "grid",
        fontFamily: lucid.face,
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
    head: {
        alignItems: "center",
        backgroundColor: lucid.panel,
        borderBottomColor: lucid.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    logo: {
        alignItems: "center",
        backgroundColor: lucid.orange,
        borderRadius: "8px",
        color: lucid.onAccent,
        display: "inline-flex",
        fontSize: "16px",
        fontWeight: 800,
        height: "34px",
        justifyContent: "center",
        width: "34px",
    },
    titles: {
        display: "grid",
        gap: "2px",
    },
    title: {
        alignItems: "center",
        color: lucid.ink,
        display: "flex",
        fontSize: "14px",
        gap: "8px",
    },
    menus: {
        color: lucid.muted,
        display: { default: "flex", [site.narrow]: "none" },
        fontSize: "12px",
        gap: "12px",
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
        marginInlineStart: "auto",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: lucid.onAccent,
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
        borderColor: lucid.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: { default: "inline-flex", [site.narrow]: "none" },
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    share: {
        backgroundColor: lucid.orange,
        borderRadius: "6px",
        color: lucid.onAccent,
        fontWeight: 700,
        paddingBlock: "7px",
        paddingInline: "16px",
    },
    canvas: {
        backgroundImage: `radial-gradient(${lucid.dots} 1px, transparent 1px)`,
        backgroundSize: "18px 18px",
        overflow: "hidden",
        position: "relative",
    },
    frames: {
        display: "grid",
        gap: "28px",
        gridTemplateColumns: "repeat(3, 210px)",
        insetInlineStart: { default: "84px", [site.narrow]: "16px" },
        position: "absolute",
        top: "48px",
    },
    frame: {
        borderColor: lucid.line,
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        padding: "14px",
        position: "relative",
    },
    frameName: {
        color: lucid.muted,
        fontSize: "12px",
        fontWeight: 600,
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
        boxShadow: lucid.stickyLift,
        color: lucid.stickyInk,
        display: "flex",
        fontSize: "12px",
        lineHeight: 1.3,
        padding: "8px",
    },
    yellow: {
        backgroundColor: lucid.yellow,
    },
    green: {
        backgroundColor: lucid.green,
    },
    blue: {
        backgroundColor: lucid.blue,
    },
    purple: {
        backgroundColor: lucid.purple,
    },
    toolbar: {
        backgroundColor: lucid.panel,
        borderRadius: "8px",
        boxShadow: lucid.lift,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "2px",
        insetInlineStart: "14px",
        padding: "5px",
        position: "absolute",
        top: "40px",
    },
    tool: {
        alignItems: "center",
        borderRadius: "6px",
        display: "inline-flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    toolOn: {
        backgroundColor: lucid.blue,
    },
    zoom: {
        alignItems: "center",
        backgroundColor: lucid.panel,
        borderRadius: "8px",
        bottom: "12px",
        boxShadow: lucid.lift,
        display: "flex",
        gap: "10px",
        insetInlineEnd: "12px",
        paddingBlock: "6px",
        paddingInline: "10px",
        position: "absolute",
    },
});

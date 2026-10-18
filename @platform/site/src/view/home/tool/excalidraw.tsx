import * as style from "@destack/style";

import { board, tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { excalidraw } from "./excalidraw.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    lock: ["M7 11V7a5 5 0 0 1 10 0v4", "M5 11h14v10H5z"],
    hand: [
        "M8 13V6a1.5 1.5 0 0 1 3 0v6",
        "M11 11V4.5a1.5 1.5 0 0 1 3 0V11",
        "M14 11V6a1.5 1.5 0 0 1 3 0v8a6 6 0 0 1-6 6h-1a6 6 0 0 1-5-3l-2-4a1.5 1.5 0 0 1 2.5-1.5L8 13",
    ],
    cursor: ["M5 3l14 7-6 2-2 6z"],
    square: ["M5 5h14v14H5z"],
    diamond: ["M12 3 21 12 12 21 3 12z"],
    circle: ["M12 20a8 8 0 1 0 0-16 8 8 0 0 0 0 16z"],
    arrow: ["M5 19 19 5", "M11 5h8v8"],
    line: ["M5 19 19 5"],
    pen: ["M4 20h4L19 9l-4-4L4 16z"],
    text: ["M5 6h14", "M12 6v13"],
    eraser: ["M8 20h12", "m4 14 9-9 6 6-7 7H8z"],
    library: ["M5 4h4v16H5z", "M11 4h4v16h-4z", "m17 5 3 15"],
    undo: ["M9 14 4 9l5-5", "M4 9h10a6 6 0 0 1 0 12h-3"],
    redo: ["m15 14 5-5-5-5", "M20 9H10a6 6 0 0 0 0 12h3"],
    plus: ["M12 5v14", "M5 12h14"],
    minus: ["M5 12h14"],
} satisfies Record<string, readonly string[]>;

/** The toolbar's tools, the selection chosen. */
const tools: readonly (keyof typeof glyphs)[] = [
    "lock",
    "hand",
    "cursor",
    "square",
    "diamond",
    "circle",
    "arrow",
    "line",
    "pen",
    "text",
    "eraser",
];

/** The fill of each column's boxes, by its place on the board. */
const tints: readonly ("yellow" | "green" | "blue")[] = ["yellow", "green", "blue"];

/** Draw the launch plan as a hand-drawn sketch of boxes and arrows. */
export function Excalidraw() {
    return (
        <div data-component="ExcalidrawPage" {...style.attrs(styles.page)}>
            {/* the sketch */}
            <div {...style.attrs(styles.sketch)}>
                {board.map(([column, notes], index) => (
                    <section {...style.attrs(styles.column)}>
                        <span {...style.attrs(styles.heading)}>{column}</span>
                        {notes.map((note) => {
                            const task = tasks.find(([id]) => id === note);

                            return (
                                <span
                                    {...style.attrs(
                                        styles.box,
                                        tints[index] === "green"
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
                    </section>
                ))}
                <svg aria-hidden="true" viewBox="0 0 60 30" {...style.attrs(styles.arrow)}>
                    <path d="M2 18c14-4 30-6 52-3" />
                    <path d="m46 9 9 6-10 5" />
                </svg>
            </div>

            {/* menu */}
            <span {...style.attrs(styles.island, styles.menu)}>
                <Mark name="menu" size={18} />
            </span>

            {/* toolbar */}
            <nav {...style.attrs(styles.island, styles.toolbar)}>
                {tools.map((tool, index) => (
                    <span {...style.attrs(styles.tool, tool === "cursor" && styles.toolOn)}>
                        <Mark name={tool} size={16} />
                        {index > 1 ? (
                            <span {...style.attrs(styles.key)}>{index - 1}</span>
                        ) : undefined}
                    </span>
                ))}
            </nav>

            {/* sharing */}
            <span {...style.attrs(styles.end)}>
                <span {...style.attrs(styles.share)}>Share</span>
                <span {...style.attrs(styles.island, styles.library)}>
                    <Mark name="library" size={16} />
                    Library
                </span>
            </span>

            {/* zoom and history */}
            <span {...style.attrs(styles.foot)}>
                <span {...style.attrs(styles.island, styles.zoom)}>
                    <Mark name="minus" size={14} />
                    100%
                    <Mark name="plus" size={14} />
                </span>
                <span {...style.attrs(styles.island, styles.zoom)}>
                    <Mark name="undo" size={14} />
                    <Mark name="redo" size={14} />
                </span>
            </span>
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

/** The Excalidraw page styles. */
const styles = style.create({
    page: {
        backgroundColor: excalidraw.canvas,
        color: excalidraw.ink,
        fontFamily: excalidraw.face,
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
        strokeWidth: 1.7,
    },
    sketch: {
        display: "grid",
        fontFamily: excalidraw.hand,
        gap: "44px",
        gridTemplateColumns: "repeat(3, 170px)",
        insetInlineStart: { default: "70px", [site.narrow]: "16px" },
        position: "absolute",
        top: "92px",
    },
    column: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
    },
    heading: {
        fontSize: "22px",
    },
    box: {
        borderColor: excalidraw.ink,
        borderEndEndRadius: "225px 15px",
        borderEndStartRadius: "15px 255px",
        borderStartEndRadius: "15px 225px",
        borderStartStartRadius: "255px 15px",
        borderStyle: "solid",
        borderWidth: "2px",
        fontSize: "15px",
        lineHeight: 1.25,
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    yellow: {
        backgroundColor: excalidraw.yellow,
    },
    green: {
        backgroundColor: excalidraw.green,
    },
    blue: {
        backgroundColor: excalidraw.blue,
    },
    arrow: {
        fill: "none",
        height: "30px",
        insetInlineStart: "164px",
        position: "absolute",
        stroke: excalidraw.ink,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2,
        top: "60px",
        width: "60px",
    },
    island: {
        backgroundColor: excalidraw.island,
        borderRadius: "8px",
        boxShadow: excalidraw.lift,
    },
    menu: {
        alignItems: "center",
        display: "inline-flex",
        height: "36px",
        insetInlineStart: "14px",
        justifyContent: "center",
        position: "absolute",
        top: "14px",
        width: "36px",
    },
    toolbar: {
        display: "flex",
        gap: "2px",
        insetInlineStart: "50%",
        padding: "4px",
        position: "absolute",
        top: "14px",
        transform: "translateX(-50%)",
    },
    tool: {
        alignItems: "center",
        borderRadius: "6px",
        display: "inline-flex",
        height: "34px",
        justifyContent: "center",
        position: "relative",
        width: "34px",
    },
    toolOn: {
        backgroundColor: excalidraw.chosen,
        color: excalidraw.violet,
    },
    key: {
        bottom: "1px",
        color: excalidraw.muted,
        fontSize: "8px",
        insetInlineEnd: "3px",
        position: "absolute",
    },
    end: {
        alignItems: "center",
        display: { default: "flex", [site.compact]: "none" },
        gap: "8px",
        insetInlineEnd: "14px",
        position: "absolute",
        top: "14px",
    },
    share: {
        backgroundColor: excalidraw.violet,
        borderRadius: "8px",
        color: excalidraw.onAccent,
        fontWeight: 600,
        paddingBlock: "9px",
        paddingInline: "14px",
    },
    library: {
        alignItems: "center",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    foot: {
        bottom: "14px",
        display: "flex",
        gap: "8px",
        insetInlineStart: "14px",
        position: "absolute",
    },
    zoom: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
});
